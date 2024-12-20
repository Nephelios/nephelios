mod helpers {
    pub mod docker_helper;
    pub mod github_helper;
}
use crate::helpers::docker_helper as docker;
use crate::helpers::github_helper as github;

/// Orchestrates the entire deployment pipeline:
/// 1. Clones a GitHub repository.
/// 2. Generate and write a Dockerfile.
/// 3. Builds a Docker image.
/// 4. Runs a Docker container.
/// 5. Cleans up the temporary directory.
///
/// # Arguments
/// * `github_url` - The URL of the GitHub repository to clone.
/// * `app_type` - The type of the application (e.g., "nodejs", "python").
/// * `app_name` - The name of the application, used for image and container tagging.
///
/// # Returns
/// `Ok(())` on success or `Err(String)` with an error message on failure.
fn pipeline(github_url: &str, app_type: &str, app_name: &str) -> Result<(), String> {
    let temp_dir = github::create_temp_dir(app_name)?;

    let target_dir = format!("{}/{}", temp_dir.display(), uuid::Uuid::new_v4());

    // Step 1: Clone Repository
    github::clone_repo(github_url, &target_dir)?;

    // Step 2: Generate Dockerfile
    docker::generate_and_write_dockerfile(app_type, &target_dir)?;

    // Step 3: Build Docker Image
    docker::build_image(app_name, &target_dir)?;

    // Step 4: Run Docker Container
    docker::create_and_run_container(app_name)?;

    // Step 5: Cleanup
    github::remove_temp_dir(&temp_dir)?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let github_url = "https://github.com/rat9615/simple-nodejs-app";
    let app_type = "nodejs";
    let app_name = "my-app";

    if let Err(e) = pipeline(github_url, app_type, app_name) {
        eprintln!("Pipeline failed: {}", e);
    }
}
