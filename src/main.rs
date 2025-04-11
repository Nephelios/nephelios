mod routes;
mod services;

use crate::routes::{create_app_route, get_apps_route, health_check_route, remove_app_route, stop_app_route,start_app_route, create_metrics_route};
use crate::services::websocket::ws_route;

use crate::services::helpers::docker_helper::{check_swarm, deploy_nephelios_stack, init_swarm, leave_swarm, prune_images, stop_nephelios_stack};
use std::env;
use tokio::sync::broadcast;
use warp::http::Method;
use warp::Filter;
mod metrics;
use crate::metrics::{CONTAINER_CPU, CONTAINER_MEM, REGISTRY};

/// Entry point for the application.
///
/// Initializes and starts the Warp server. The server listens on `127.0.0.1:3030`
/// and provides the following routes:
/// - `/create` (POST): Handles app creation requests. Expects a JSON body with app details.
/// - `/health` (GET): Provides a simple health check endpoint to verify the server's status.
///
/// Combines the routes using Warp's `or` filter and serves them.
///
/// # Example
///
/// To start the server, run the application and use the following curl commands:
/// ```sh
/// # Health check
/// curl http://127.0.0.1:3030/health
///
/// # App creation (example)
/// curl -X POST http://127.0.0.1:3030/create \
///      -H "Content-Type: application/json" \
///      -d '{"app_name": "my-app", "app_type": "nodejs", "github_url": "https://github.com/user/repo"}'
/// ```
#[tokio::main]
async fn main() {
    println!("🚀 Starting Nephelios...");
    dotenv::dotenv().ok();

    let app_port: u16 = env::var("NEPHELIOS_PORT")
        .unwrap_or_else(|_| "3030".to_string())
        .parse()
        .unwrap_or(3030);

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(&[Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(vec!["Content-Type"]);

    let (status_tx, status_rx) = broadcast::channel(32);
    let api_routes = create_app_route(status_tx.clone())
    .or(health_check_route())
    .or(get_apps_route())
    .or(ws_route(status_rx))
    .or(remove_app_route())
    .or(stop_app_route())
    .or(start_app_route())
    .or(create_metrics_route())
    .with(cors);

    REGISTRY.register(Box::new(CONTAINER_CPU.clone())).unwrap();
    REGISTRY.register(Box::new(CONTAINER_MEM.clone())).unwrap();

    // Source : https://stackoverflow.com/a/71279547
    let (_addr, server) = warp::serve(api_routes)
        .bind_with_graceful_shutdown(([127, 0, 0, 1], app_port), async {
            tokio::signal::ctrl_c().await.ok();
        });

    let ip_addr = _addr.ip();

    println!("🚀 Pruning Docker images...");
    let res_prune_images = prune_images().await;
    match res_prune_images {
        Ok(_) => println!("✅ Docker images pruned successfully"),
        Err(e) => eprintln!("❌ Failed to prune Docker images: {}", e)
    }

    println!("🚀 Check if Docker Swarm is initialized...");
    let is_alive = check_swarm();
    match is_alive {
        Ok(res) => {
            if res {
                println!("✅ Docker Swarm is already initialized")
            } else {
                println!("❌ Docker Swarm is not initialized");
                println!("🚀 Init Docker Swarm...");
                let result_init_swarm = init_swarm(ip_addr);
                match result_init_swarm {
                    Ok(_) => println!("✅ Docker Swarm initialized successfully"),
                    Err(e) => {
                        eprintln!("❌ Failed to initialize Docker Swarm: {}", e);
                        return;
                    }
                }
            }
        },
        Err(e) => {
            println!("❌ Failed to check Docker Swarm: {}", e);
            return;
        }
    }

    println!("🚀 Starting Nephelios Stack...");
    let result_start_stack = deploy_nephelios_stack();
    match result_start_stack {
        Ok(_) => println!("✅ Nephelios Stack started successfully"),
        Err(e) => {
            eprintln!("❌ Failed to start Nephelios Stack: {}", e);
            return;
        }
    }

    println!("🚀 Server running on http://{}:{}", ip_addr.to_string(), app_port);

    match tokio::join!(tokio::task::spawn(server)).0 {
        Ok(()) => println!("serving"),
        Err(e) => println!("ERROR: Thread join error {}", e)
    };

    println!("🛑 Terminating Nephelios Stack...");
    let result_rm_stack = stop_nephelios_stack();
    match result_rm_stack {
        Ok(_) => println!("✅ Nephelios Stack terminated successfully"),
        Err(e) => eprintln!("❌ Failed to terminate Nephelios Stack: {}", e)
    }

    if env::var("LEAVE_SWARM").unwrap_or_else(|_| "false".to_string()) == "true" {
        println!("🛑 Leaving Docker Swarm...");
        let result_leave_swarm = leave_swarm();
        match result_leave_swarm {
            Ok(_) => println!("✅ Left Docker Swarm successfully"),
            Err(e) => eprintln!("❌ Failed to leave Docker Swarm: {}", e)
        }
    }

    println!("🛑 Pruning Docker images...");
    let res_prune_images = prune_images().await;
    match res_prune_images {
        Ok(_) => println!("✅ Docker images pruned successfully"),
        Err(e) => eprintln!("❌ Failed to prune Docker images: {}", e)
    }

    println!("👋 Goodbye!");
}
