use warp::reject;
use warp::Filter;

use crate::services::helpers::traefik_helper::{add_to_deploy, verif_app};

use crate::services::helpers::docker_helper::{
    build_image, docker_compose, generate_and_write_dockerfile,
};
use crate::services::helpers::github_helper::{clone_repo, create_temp_dir, remove_temp_dir};

#[derive(Debug)]
struct CustomError(String);

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl reject::Reject for CustomError {}

/// Creates the route for app creation.
///
/// This route listens for POST requests at the `/create` path and expects a JSON body.
/// The JSON body should contain the following keys:
/// - `app_name`: The name of the application (default: "default-app").
/// - `app_type`: The type of the application (e.g., "nodejs", default: "nodejs").
/// - `github_url`: The GitHub URL for the application repository (required).
///
/// Returns a boxed Warp filter that handles app creation requests.
pub fn create_app_route() -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    warp::post()
        .and(warp::path("create"))
        .and(warp::body::json()) // Expect JSON body
        .and_then(handle_create_app)
        .boxed()
}

/// Creates the route for health checks.
///
/// This route listens for GET requests at the `/health` path.
/// It is used to verify the server's status and returns a JSON response "OK".
///
/// Returns a boxed Warp filter that handles health check requests.
pub fn health_check_route() -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    warp::get()
        .and(warp::path("health"))
        .map(|| warp::reply::json(&"OK"))
        .boxed()
}

/// Handles the app creation logic.
///
/// Extracts `app_name`, `app_type`, and `github_url` from the JSON body.
/// If `github_url` is missing or empty, it returns a 400 Bad Request response.
/// Otherwise, it returns a 201 Created response indicating the app was created.
///
/// # Arguments
///
/// * `body` - The JSON body received in the POST request.
///
/// # Returns
///
/// A result containing a Warp reply or a Warp rejection.
async fn handle_create_app(body: serde_json::Value) -> Result<impl warp::Reply, warp::Rejection> {
    let app_name = body
        .get("app_name")
        .and_then(|v| v.as_str())
        .unwrap_or("default-app");
    let app_type = body
        .get("app_type")
        .and_then(|v| v.as_str())
        .unwrap_or("nodejs");
    let github_url = body.get("github_url").and_then(|v| v.as_str());

    if let Some(github_url) = github_url {
        if github_url.is_empty() {
            return Ok(warp::reply::with_status(
                "GitHub URL is required".to_string(),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }

        let directory = create_temp_dir(app_name).map_err(|e| {
            warp::reject::custom(CustomError(format!(
                "Failed to create temp directory: {}",
                e
            )))
        })?;

        clone_repo(github_url, directory.to_str().get_or_insert("")).map_err(|e| {
            warp::reject::custom(CustomError(format!("Failed to clone repository: {}", e)))
        })?;

        generate_and_write_dockerfile(app_type, directory.to_str().get_or_insert(""))
            .map_err(|e| warp::reject::custom(CustomError(e)))?;

        build_image(app_name, directory.to_str().get_or_insert("")).map_err(|e| {
            warp::reject::custom(CustomError(format!("Failed to build Docker image: {}", e)))
        })?;

        if let Ok(1) = verif_app(app_name) {
            println!(
                "Application {} already deployed, updating it right now.",
                app_name
            );
            docker_compose().map_err(|e| {
                warp::reject::custom(CustomError(format!(
                    "Failed to execute docker compose: {}",
                    e
                )))
            })?;
        } else {
            add_to_deploy(app_name, "3000").map_err(|e| {
                warp::reject::custom(CustomError(format!(
                    "Failed to add app to deploy file: {}",
                    e
                )))
            })?;

            docker_compose().map_err(|e| {
                warp::reject::custom(CustomError(format!(
                    "Failed to execute docker compose: {}",
                    e
                )))
            })?;
        }

        remove_temp_dir(&directory).map_err(|e| {
            warp::reject::custom(CustomError(format!(
                "Failed to remove temp directory: {}",
                e
            )))
        })?;

        let string_response = format!(
            "Created app: {} of type: {} with GitHub URL: {}",
            app_name, app_type, github_url
        );

        Ok(warp::reply::with_status(
            string_response,
            warp::http::StatusCode::CREATED,
        ))
    } else {
        Ok(warp::reply::with_status(
            "GitHub URL is required".to_string(),
            warp::http::StatusCode::BAD_REQUEST,
        ))
    }
}
