mod routes;
mod services;

use crate::routes::{create_app_route, health_check_route,remove_app_route};
use std::env;
use routes::{start_app_route, stop_app_route};
use warp::Filter;

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

    let app_port: u16 = env::var("APP_PORT")
        .unwrap_or_else(|_| "3030".to_string())
        .parse()
        .unwrap_or(3030);

    let api_routes = create_app_route().or(health_check_route()).or(remove_app_route()).or(stop_app_route()).or(start_app_route());

    warp::serve(api_routes)
        .run(([127, 0, 0, 1], app_port))
        .await;
}
