use crate::services::helpers::traefik_helper::{add_to_deploy, verif_app};

use crate::services::helpers::docker_helper::{
    build_image, generate_and_write_dockerfile, list_deployed_apps, remove_container,
    start_docker_compose, stop_container, AppMetadata,
};

use crate::services::helpers::traefik_helper::remove_app_compose;

use crate::services::helpers::github_helper::{clone_repo, create_temp_dir, remove_temp_dir};
use serde_json::json;
use serde_json::Value;
use warp::{reject, Filter};

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
        .and(warp::body::json()) // Expect a JSON body
        .and_then(handle_create_app)
        .boxed()
}

/// Creates the route for app removal.
///
/// This route listens for POST requests at the `/remove` path and expects a JSON body.
/// The JSON body should contain the following key:
/// - `app_name`: The name of the application (default: "default-app").
///
/// Returns a boxed Warp filter that handles app removal requests.

pub fn remove_app_route() -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    warp::post()
        .and(warp::path("remove"))
        .and(warp::body::json()) // Expect a JSON body
        .and_then(handle_remove_app)
        .boxed()
}


/// Creates the route for stopping an app.
///
/// This route listens for POST requests at the `/stop` path and expects a JSON body.
/// The JSON body should contain the following key:
/// - `app_name`: The name of the application (default: "default-app").
///
/// Returns a boxed Warp filter that handles app stop requests.

pub fn stop_app_route() -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    warp::post()
    .and(warp::path("stop"))
    .and(warp::body::json()) // Expect a JSON body
    .and_then(handle_stop_app)
    .boxed()
}

/// Creates the route for starting an app.
///
/// This route listens for POST requests at the `/start` path and expects a JSON body.
/// The JSON body should contain the following key:
/// - `app_name`: The name of the application (default: "default-app").
///
/// Returns a boxed Warp filter that handles app start requests.

pub fn start_app_route() -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    warp::post()
    .and(warp::path("start"))
    .and(warp::body::json()) // Expect a JSON body
    .and_then(handle_start_app)
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

/// Handles the app start logic.
///
/// Extracts `app_name` from the JSON body and performs the necessary steps to start the app:
/// adding the app to the deployment list and starting the Docker Compose process.
///
/// # Arguments
///
/// * `body` - The JSON body received in the request, expected to contain `app_name`.
///
/// # Returns
///
/// A result containing a Warp reply or a Warp rejection.

async fn handle_start_app(body: Value) -> Result<impl warp::Reply, warp::Rejection>{

    let app_name = body
        .get("app_name")
        .and_then(Value::as_str)
        .unwrap_or("default-app");

    let _ = add_to_deploy(app_name, "3000").map_err(|e|{
        warp::reject::custom(CustomError(format!(
            "Failed to stop container for app {}: {}",
            app_name, e
        )))
    })?;

    let _ = start_docker_compose().map_err(|e|{
        warp::reject::custom(CustomError(format!(
            "Failed to stop container for app {}: {}",
            app_name, e
        )))
    })?;

    Ok(warp::reply::with_status(
        format!("start app: {}.", app_name),
        warp::http::StatusCode::CREATED,
        ))

}

/// Handles the app stop logic.
///
/// Extracts `app_name` from the JSON body and performs the necessary steps to stop the app:
/// stopping the running container and deleting the associated compose file.
///
/// # Arguments
///
/// * `body` - The JSON body received in the request, expected to contain `app_name`.
///
/// # Returns
///
/// A result containing a Warp reply or a Warp rejection.

async fn handle_stop_app(body: Value) -> Result<impl warp::Reply, warp::Rejection> {

    let app_name = body
        .get("app_name")
        .and_then(Value::as_str)
        .unwrap_or("default-app");

    let _ = stop_container(app_name).await.map_err(|e|{
        warp::reject::custom(CustomError(format!(
            "Failed to stop container for app {}: {}",
            app_name, e
        )))
    })?;

    let _ = remove_app_compose(app_name).map_err(|e|{
        warp::reject::custom(CustomError(format!(
            "Failed to remove app compose for app {}: {}",
            app_name, e
        )))
    })?;

    Ok(warp::reply::with_status(
        format!("stop app: {}.", app_name),
        warp::http::StatusCode::CREATED,
        ))

}

/// Handles the app removal logic.
///
/// Extracts `app_name` from the JSON body and performs the necessary steps to remove the app:
/// stopping the running container, removing the container, and deleting the associated compose file.
///
/// # Arguments
///
/// * `body` - The JSON body received in the request, expected to contain `app_name`.
///
/// # Returns
///
/// A result containing a Warp reply or a Warp rejection.

async fn handle_remove_app(body: Value) -> Result<impl warp::Reply, warp::Rejection> {
    let app_name = body
        .get("app_name")
        .and_then(Value::as_str)
        .unwrap_or("default-app");

    let _ = stop_container(app_name).await.map_err(|e| {
        warp::reject::custom(CustomError(format!(
            "Failed to stop container for app {}: {}",
            app_name, e
        )))
    })?;

    let _ = remove_container(app_name).await.map_err(|e| {
        warp::reject::custom(CustomError(format!(
            "Failed to remove container for app {}: {}",
            app_name, e
        )))
    })?;

    let _ = remove_app_compose(app_name).map_err(|e| {
        warp::reject::custom(CustomError(format!(
            "Failed to remove app compose file for app {}: {}",
            app_name, e
        )))
    })?;

    Ok(warp::reply::with_status(
        format!("Remove app: {}.", app_name),
        warp::http::StatusCode::CREATED,
    ))
}

pub fn get_apps_route() -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    warp::get()
        .and(warp::path("get-apps"))
        .and_then(handle_get_apps)
        .boxed()
}

pub async fn handle_get_apps() -> Result<impl warp::Reply, warp::Rejection> {
    match list_deployed_apps().await {
        Ok(apps) => {
            let response = json!({
                "status": "success",
                "apps": apps,
                "total": apps.len(),
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            let response = json!({
                "status": "error",
                "message": format!("Failed to list apps: {}", e)
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handles the app creation logic.
///
/// Extracts `app_name`, `app_type`, and `github_url` from the JSON body.
/// Performs cloning, Dockerfile generation, image building, and container creation.
///
/// # Arguments
///
/// * `body` - The JSON body received in the POST request.
///
/// # Returns
///
/// A result containing a Warp reply or a Warp rejection.

async fn handle_create_app(body: Value) -> Result<impl warp::Reply, warp::Rejection> {
    let app_name = body
        .get("app_name")
        .and_then(Value::as_str)
        .unwrap_or("default-app");
    let app_type = body
        .get("app_type")
        .and_then(Value::as_str)
        .unwrap_or("nodejs");
    let github_url = body.get("github_url").and_then(Value::as_str);

    if github_url.is_none() || github_url.unwrap().is_empty() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&json!({
                "error": "GitHub URL is required"
            })),
            warp::http::StatusCode::BAD_REQUEST,
        ));
    }

    let github_url = github_url.unwrap();
    let metadata = AppMetadata::new(
        app_name.to_string(),
        app_type.to_string(),
        github_url.to_string(),
    );

    let temp_dir = create_temp_dir(app_name).map_err(|e| {
        warp::reject::custom(CustomError(format!(
            "Failed to create temp directory: {}",
            e
        )))
    })?;

    let temp_dir_path = temp_dir.to_str().ok_or_else(|| {
        warp::reject::custom(CustomError("Temp directory path is invalid".to_string()))
    })?;

    if let Err(e) = clone_repo(github_url, temp_dir_path) {
        let _ = remove_temp_dir(&temp_dir);
        return Err(warp::reject::custom(CustomError(format!(
            "Failed to clone repository: {}",
            e
        ))));
    }

    if let Err(e) = generate_and_write_dockerfile(app_type, temp_dir_path, &metadata) {
        let _ = remove_temp_dir(&temp_dir);
        return Err(warp::reject::custom(CustomError(format!(
            "Failed to generate Dockerfile: {}",
            e
        ))));
    }

    if let Err(e) = build_image(app_name, temp_dir_path, &metadata).await {
        let _ = remove_temp_dir(&temp_dir);
        return Err(warp::reject::custom(CustomError(format!(
            "Failed to build Docker image: {}",
            e
        ))));
    }

    if let Ok(1) = verif_app(app_name) {
        println!(
            "Application {} already deployed, updating it right now.",
            app_name
        );

        if let Err(e) = start_docker_compose() {
            let _ = remove_temp_dir(&temp_dir);
            return Err(warp::reject::custom(CustomError(format!(
                "Failed to execute docker compose: {}",
                e
            ))));
        }
    } else {
        if let Err(e) = add_to_deploy(app_name, "3000") {
            let _ = remove_temp_dir(&temp_dir);
            return Err(warp::reject::custom(CustomError(format!(
                "Failed to add app to deploy file: {}",
                e
            ))));
        }

        if let Err(e) = start_docker_compose() {
            let _ = remove_temp_dir(&temp_dir);
            return Err(warp::reject::custom(CustomError(format!(
                "Failed to execute docker compose: {}",
                e
            ))));
        }
    }

    if let Err(e) = remove_temp_dir(&temp_dir) {
        eprintln!("Warning: Failed to clean up temp directory: {}", e);
    }

    let response = json!({
        "message": "Application created successfully",
        "app_name": app_name,
        "app_type": app_type,
        "github_url": github_url,
        "url": format!("http://{}.localhost", app_name),
        "metadata": {
            "created_at": metadata.created_at,
            "domain": metadata.domain,
        }
    });

    Ok(warp::reply::with_status(
        warp::reply::json(&response),
        warp::http::StatusCode::CREATED,
    ))
}
