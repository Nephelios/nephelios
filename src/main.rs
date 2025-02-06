mod routes;
mod services;

use crate::routes::{create_app_route, health_check_route};
use routes::get_apps_route;
use std::env;
use warp::http::Method;
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

    let app_port: u16 = env::var("NEPHELIOS_PORT")
        .unwrap_or_else(|_| "3030".to_string())
        .parse()
        .unwrap_or(3030);

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(&[Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(vec!["Content-Type"]);

    let api_routes = create_app_route()
        .or(health_check_route())
        .or(get_apps_route())
        .with(cors);

    println!("🚀 Server running on http://127.0.0.1:{}", app_port);

    warp::serve(api_routes)
        .run(([127, 0, 0, 1], app_port))
        .await;
}
