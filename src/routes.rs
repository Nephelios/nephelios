use warp::Filter;

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
    let app_name = body["app_name"].as_str().unwrap_or("default-app");
    let app_type = body["app_type"].as_str().unwrap_or("nodejs");
    let github_url = body["github_url"].as_str().unwrap_or("");

    if github_url.is_empty() {
        return Ok(warp::reply::with_status(
            "GitHub URL is required".to_string(),
            warp::http::StatusCode::BAD_REQUEST,
        ));
    }

    let string_response = format!(
        "Created app: {} of type: {} with GitHub URL: {}",
        app_name, app_type, github_url
    );

    Ok(warp::reply::with_status(
        string_response,
        warp::http::StatusCode::CREATED,
    ))
}
