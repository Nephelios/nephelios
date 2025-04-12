use std::collections::HashMap;

use crate::services::helpers::traefik_helper::{add_to_deploy, verif_app};
use futures_util::TryFutureExt;

use crate::services::helpers::docker_helper::{
    build_image, deploy_nephelios_stack, generate_and_write_dockerfile, get_app_status,
    list_deployed_apps, prune_images, push_image, remove_service, scale_app, update_metrics, AppMetadata,
};

use crate::services::helpers::traefik_helper::remove_app_compose;

use crate::services::helpers::github_helper::{clone_repo, create_temp_dir, remove_temp_dir};
use crate::services::websocket::{send_deployment_status, StatusSender};
use serde_json::json;
use serde_json::Value;
use warp::{reject, Filter};
use prometheus::{TextEncoder, Encoder};
use crate::metrics::{REGISTRY};



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
pub fn create_app_route(
    status_tx: StatusSender,
) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    warp::post()
        .and(warp::path("create"))
        .and(warp::body::json())
        .and(warp::any().map(move || status_tx.clone()))
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



pub fn create_metrics_route() -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    warp::path("metrics")
        .and(warp::get())
        .and_then(handle_metrics)
        .boxed()
}


async fn handle_metrics() -> Result<impl warp::Reply, warp::Rejection> {
    if let Err(e) = update_metrics().await {
        eprintln!("Failed to update metrics: {}", e);
    }

    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    let response = String::from_utf8(buffer.clone()).unwrap();
    Ok(warp::reply::with_header(response, "Content-Type", encoder.format_type()))
}

/// Handles the app start logic.
///
/// Extracts `app_name` from the JSON body and performs the necessary steps to start the app:
/// adding the app to the deployment list and scaling the service to 1.
///
/// # Arguments
///
/// * `body` - The JSON body received in the request, expected to contain `app_name`.
///
/// # Returns
///
/// A result containing a Warp reply or a Warp rejection.

async fn handle_start_app(body: Value) -> Result<impl warp::Reply, warp::Rejection> {
    let app_name = body
        .get("app_name")
        .and_then(Value::as_str)
        .unwrap_or("default-app");

    let scale: &str = "1";

    let _ = scale_app(app_name, scale).await.map_err(|e| {
        warp::reject::custom(CustomError(format!(
            "Failed to scale service for app {}: {}",
            app_name, e
        )))
    });

    Ok(warp::reply::with_status(
        format!("start app: {}.", app_name),
        warp::http::StatusCode::CREATED,
    ))
}

/// Handles the app stop logic.
///
/// Extracts `app_name` from the JSON body and performs the necessary steps to stop the app:
/// stopping the running service and scaling this serving to 0.
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

    let scale: &str = "0";

    let _ = scale_app(app_name, scale).await.map_err(|e| {
        warp::reject::custom(CustomError(format!(
            "Failed to scale service for app {}: {}",
            app_name, e
        )))
    });

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

    let _ = remove_service(app_name).await.map_err(|e| {
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
async fn handle_create_app(
    body: Value,
    status_tx: StatusSender,
) -> Result<impl warp::Reply, warp::Rejection> {
    let _ = tokio::spawn(async move {
        let app_name = body
            .get("app_name")
            .and_then(Value::as_str)
            .unwrap_or("default-app");
        let app_type = body
            .get("app_type")
            .and_then(Value::as_str)
            .unwrap_or("nodejs");
        let github_url = body.get("github_url").and_then(Value::as_str);

        let install_command = body
            .get("install_command")
            .and_then(Value::as_str)
            .unwrap_or("");
        let run_command = body
            .get("run_command")
            .and_then(Value::as_str)
            .unwrap_or("");
        let build_command = body
            .get("build_command")
            .and_then(Value::as_str)
            .unwrap_or("");
        let app_workdir = body
            .get("app_workdir")
            .and_then(Value::as_str)
            .unwrap_or("/app");
        let additional_inputs = body
            .get("additionalInputs")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let key = item.get("key").and_then(Value::as_str)?;
                        let value = item.get("value").and_then(Value::as_str)?;
                        Some((key.to_string(), value.to_string()))
                    })
                    .collect::<HashMap<String, String>>()
            })
            .unwrap_or_else(HashMap::new);

        if github_url.is_none() || github_url.unwrap().is_empty() {
            send_deployment_status(
                &status_tx,
                app_name,
                "error",
                "GitHub URL is required",
                None,
            )
            .await;
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

        // Clone repository
        send_deployment_status(
            &status_tx,
            app_name,
            "in_progress",
            "Cloning repository",
            None,
        )
        .await;
        let temp_dir = match create_temp_dir(app_name) {
            Ok(dir) => dir,
            Err(e) => {
                send_deployment_status(
                    &status_tx,
                    app_name,
                    "error",
                    &format!("Failed to create temp directory: {}", e),
                    None,
                )
                .await;
                return Err(reject::custom(CustomError(format!(
                    "Failed to create temp directory: {}",
                    e
                ))));
            }
        };

        let temp_dir_path = match temp_dir.to_str() {
            Some(path) => path,
            None => {
                send_deployment_status(
                    &status_tx,
                    app_name,
                    "error",
                    "Invalid temp directory path",
                    None,
                )
                .await;
                return Err(reject::custom(CustomError(
                    "Temp directory path is invalid".to_string(),
                )));
            }
        };

        if let Err(e) = clone_repo(github_url, temp_dir_path) {
            let _ = remove_temp_dir(&temp_dir);
            send_deployment_status(
                &status_tx,
                app_name,
                "error",
                &format!("Failed to clone repository: {}", e),
                None,
            )
            .await;
            return Err(reject::custom(CustomError(format!(
                "Failed to clone repository: {}",
                e
            ))));
        }

        // Generate Dockerfile
        if let Err(e) = generate_and_write_dockerfile(
            app_type,
            temp_dir_path,
            &metadata,
            install_command,
            run_command,
            build_command,
            app_workdir,
            Some(&additional_inputs),
        ) {
            let _ = remove_temp_dir(&temp_dir);
            send_deployment_status(
                &status_tx,
                app_name,
                "error",
                &format!("Failed to generate Dockerfile: {}", e),
                None,
            )
            .await;
            return Err(reject::custom(CustomError(format!(
                "Failed to generate Dockerfile: {}",
                e
            ))));
        }

        send_deployment_status(&status_tx, app_name, "success", "Cloning repository", None).await;

        // Build Docker image
        send_deployment_status(
            &status_tx,
            app_name,
            "in_progress",
            "Building Docker image",
            None,
        )
        .await;
        if let Err(e) = build_image(app_name, temp_dir_path, &metadata).await {
            let _ = remove_temp_dir(&temp_dir);
            send_deployment_status(
                &status_tx,
                app_name,
                "error",
                &format!("Failed to build Docker image: {}", e),
                None,
            )
            .await;
            return Err(reject::custom(CustomError(format!(
                "Failed to build Docker image: {}",
                e
            ))));
        }

        send_deployment_status(
            &status_tx,
            app_name,
            "success",
            "Building Docker image",
            None,
        )
        .await;

        if let Err(e) = push_image(app_name).await {
            return Err(reject::custom(CustomError(format!(
                "Failed to push Docker image: {}",
                e
            ))));
        }

        send_deployment_status(
            &status_tx,
            app_name,
            "in_progress",
            "Starting deployment",
            None,
        )
        .await;
        if let Ok(1) = verif_app(app_name) {
            if let Err(e) = deploy_nephelios_stack() {
                let _ = remove_temp_dir(&temp_dir);
                send_deployment_status(
                    &status_tx,
                    app_name,
                    "error",
                    &format!("Failed to update deployment: {}", e),
                    None,
                )
                .await;
                return Err(reject::custom(CustomError(format!(
                    "Failed to execute docker compose: {}",
                    e
                ))));
            }
        } else {
            if let Err(e) = add_to_deploy(app_name, "3000", &metadata) {
                let _ = remove_temp_dir(&temp_dir);
                send_deployment_status(
                    &status_tx,
                    app_name,
                    "error",
                    &format!("Failed to add app to deploy file: {}", e),
                    None,
                )
                .await;
                return Err(reject::custom(CustomError(format!(
                    "Failed to add app to deploy file: {}",
                    e
                ))));
            }

            if let Err(e) = deploy_nephelios_stack() {
                let _ = remove_temp_dir(&temp_dir);
                send_deployment_status(
                    &status_tx,
                    app_name,
                    "error",
                    &format!("Failed to start deployment: {}", e),
                    None,
                )
                .await;
                return Err(reject::custom(CustomError(format!(
                    "Failed to execute docker compose: {}",
                    e
                ))));
            }
        }

        send_deployment_status(&status_tx, app_name, "success", "Starting deployment", None).await;

        if let Err(e) = remove_temp_dir(&temp_dir) {
            eprintln!("Warning: Failed to clean up temp directory: {}", e);
        }

        tokio::spawn(async move {
            let res_prune_images = prune_images().await;
            match res_prune_images {
                Ok(_) => println!("✅ Docker images pruned successfully"),
                Err(e) => eprintln!("❌ Failed to prune Docker images: {}", e),
            }
        });

        let response = json!({
        "message": "Application created successfully",
        "app_name": app_name,
        "app_type": app_type,
        "github_url": github_url,
        "status": get_app_status(app_name.to_string()).await,
        "domain": metadata.domain,
        "created_at": metadata.created_at,
        });

        send_deployment_status(
            &status_tx,
            app_name,
            "deployed",
            "deployed_info",
            Some(response.clone()),
        )
        .await;

        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            warp::http::StatusCode::CREATED,
        ))
    });

    Ok(warp::reply::with_status(
        "Deployment Job has been created !",
        warp::http::StatusCode::CREATED,
    ))
}
