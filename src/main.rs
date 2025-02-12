mod routes;
mod services;

use crate::routes::{create_app_route, get_apps_route, health_check_route, remove_app_route};
use crate::services::websocket::ws_route;

use std::env;
use tokio::sync::broadcast;
use warp::http::Method;
use warp::Filter;
use crate::services::helpers::docker_helper::start_docker_compose;

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
        .with(cors);

    if let Err(e) = start_docker_compose() {
        eprintln!("Failed to start Docker Compose: {}", e);
    }

    println!("🚀 Server running on http://127.0.0.1:{}", app_port);

    warp::serve(api_routes)
        .run(([127, 0, 0, 1], app_port))
        .await;


}
