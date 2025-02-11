use bollard::container::ListContainersOptions;
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
};
use bollard::image::BuildImageOptions;
use bollard::models::HostConfigLogConfig;
use bollard::service::{HostConfig, PortBinding};
use bollard::Docker;
use chrono::Utc;
use dirs::home_dir;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tar::Builder;
use tokio::fs::File as TokioFile;
use tokio_util::codec::{BytesCodec, FramedRead};
use uuid::Uuid;
use walkdir::WalkDir;
use warp::hyper::body::to_bytes;
use warp::hyper::Body;

#[derive(Debug, Clone)]
pub struct AppMetadata {
    pub app_name: String,
    pub app_type: String,
    pub github_url: String,
    pub domain: String,
    pub created_at: String,
}

impl AppMetadata {
    pub fn new(app_name: String, app_type: String, github_url: String) -> Self {
        Self {
            app_name: app_name.clone(),
            app_type,
            github_url,
            domain: format!("{}.localhost", app_name),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Converts the metadata to a HashMap of labels for Docker.
    ///
    /// # Returns
    /// A HashMap with String keys and values.
    fn to_labels(&self) -> HashMap<String, String> {
        let mut labels = HashMap::new();
        labels.insert("com.myapp.name".to_string(), self.app_name.clone());
        labels.insert("com.myapp.type".to_string(), self.app_type.clone());
        labels.insert("com.myapp.github_url".to_string(), self.github_url.clone());
        labels.insert("com.myapp.domain".to_string(), self.domain.clone());
        labels.insert("com.myapp.created_at".to_string(), self.created_at.clone());
        labels
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    pub app_name: String,
    pub app_type: String,
    pub github_url: String,
    pub domain: String,
    pub created_at: String,
    pub status: String,
    #[serde(default)]
    pub container_id: Option<String>,
}

pub async fn list_deployed_apps() -> Result<Vec<AppInfo>, String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    let container_filters: HashMap<String, Vec<String>> = HashMap::new();
    let container_options = ListContainersOptions {
        all: true,
        filters: container_filters,
        ..Default::default()
    };

    let containers = docker
        .list_containers(Some(container_options))
        .await
        .map_err(|e| format!("Failed to list containers: {}", e))?;

    let mut apps = Vec::new();

    // Iterate over containers and check for custom labels
    for container in containers {
        if let Some(labels) = container.labels {
            if let (Some(name), Some(app_type), Some(url), Some(domain), Some(created)) = (
                labels.get("com.myapp.name"),
                labels.get("com.myapp.type"),
                labels.get("com.myapp.github_url"),
                labels.get("com.myapp.domain"),
                labels.get("com.myapp.created_at"),
            ) {
                // Get container's state/status
                let status = match container.state {
                    Some(ref state) => state.clone(),
                    None => container
                        .status
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                };

                // Collect app info, handle Option<String> for container.id
                apps.push(AppInfo {
                    app_name: name.clone(),
                    app_type: app_type.clone(),
                    github_url: url.clone(),
                    domain: domain.clone(),
                    created_at: created.clone(),
                    status,
                    container_id: Some(
                        container
                            .id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                    ),
                });
            }
        }
    }

    // Sort by creation date, newest first
    apps.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(apps)
}

/// Creates a Docker context tarball for the specified application path.
///
/// # Arguments
/// * `app_path` - The path to the application directory.
///
/// # Returns
/// * `Ok(String)` containing the path to the created tarball.
/// * `Err(String)` if there is an error.
fn create_docker_context(app_path: &str) -> Result<String, String> {
    let app_dir = Path::new(app_path)
        .canonicalize()
        .map_err(|e| format!("Invalid application path: {}", e))?;

    if !app_dir.exists() || !app_dir.is_dir() {
        return Err(format!("Invalid application path: {}", app_path));
    }

    let home = home_dir().ok_or("Failed to find home directory")?;
    let tar_path = home.join(format!(
        "{}.tar",
        app_dir.file_name().unwrap().to_string_lossy()
    ));

    let tar_file =
        fs::File::create(&tar_path).map_err(|e| format!("Failed to create tar file: {}", e))?;
    let mut tar_builder = Builder::new(tar_file);

    for entry in WalkDir::new(&app_dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name() {
                if name == ".git" || name == "node_modules" {
                    continue;
                }
            }
        }

        // Add files to the tarball
        if path.is_file() && !path.is_symlink() {
            let file_name = path.strip_prefix(&app_dir).unwrap(); // Use the relative path
            tar_builder
                .append_path_with_name(path, file_name)
                .map_err(|e| format!("Failed to add file {}: {}", path.display(), e))?;
        }
    }

    tar_builder
        .finish()
        .map_err(|e| format!("Failed to finalize tarball: {}", e))?;
    println!("Docker context created at {}", tar_path.display());

    Ok(tar_path.to_string_lossy().to_string())
}

/// Generates and writes a Dockerfile for the given application type.
///
/// # Arguments
/// * `app_type` - The type of the application ("nodejs", "python", etc.).
/// * `app_path` - The path to the application directory.
///
/// # Returns
/// * `Ok(())` if successful.
/// * `Err(String)` if an error occurs.
pub fn generate_and_write_dockerfile(
    app_type: &str,
    app_path: &str,
    metadata: &AppMetadata,
) -> Result<(), String> {
    let dockerfile_path = Path::new(app_path).join("Dockerfile");

    if dockerfile_path.exists() {
        println!("Dockerfile already exists at {}", dockerfile_path.display());
        return Ok(());
    }

    let deploy_port: String =
        env::var("NEPHELIOS_APPS_PORT").unwrap_or_else(|_| "3000".to_string());

    let labels = metadata
        .to_labels()
        .iter()
        .map(|(k, v)| format!("LABEL {}=\"{}\"", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    let dockerfile_content = match app_type {
        "nodejs" => {
            format!(
                r#"FROM oven/bun:latest
WORKDIR /app
{}
COPY package.json ./
RUN bun install --production
COPY . .
EXPOSE {}
CMD ["sh", "-c", "if bun dev 2>/dev/null; then bun dev; else bun start; fi"]"#,
                labels, deploy_port
            )
        }
        "python" => {
            format!(
                r#"FROM python:3.8-slim
WORKDIR /app
{}
COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt
COPY . .
EXPOSE {}
CMD ["python", "app.py"]"#,
                labels, deploy_port
            )
        }
        _ => return Err(format!("Unsupported app type: {}", app_type)),
    };

    println!("Writing Dockerfile to {}", dockerfile_path.display());
    let mut file = File::create(&dockerfile_path)
        .map_err(|e| format!("Failed to create Dockerfile: {}", e))?;
    file.write_all(dockerfile_content.as_bytes())
        .map_err(|e| format!("Failed to write Dockerfile: {}", e))?;
    Ok(())
}

/// Builds a Docker image using the tarball created from the application directory.
///
/// # Arguments
/// * `app_name` - The name of the Docker image.
/// * `app_path` - The application directory.
///
/// # Returns
/// * `Ok(())` if successful.
/// * `Err(String)` if there is an error.
pub async fn build_image(
    app_name: &str,
    app_path: &str,
    metadata: &AppMetadata,
) -> Result<(), String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    let tar_path = create_docker_context(app_path).map_err(|e| format!("Error: {}", e))?;
    let tar_file = TokioFile::open(&tar_path)
        .await
        .map_err(|e| format!("Failed to open tar file: {}", e))?;

    let tar_stream =
        FramedRead::new(tar_file, BytesCodec::new()).map(|res| res.map(|b| b.freeze()));
    let body = Body::wrap_stream(tar_stream);

    let body_bytes = to_bytes(body)
        .await
        .map_err(|e| format!("Failed to convert Body to Bytes: {}", e))?;

    let options = BuildImageOptions {
        t: app_name.to_lowercase(),
        rm: true,
        labels: metadata.to_labels(),
        ..Default::default()
    };

    let mut build_stream = docker.build_image(options, None, Some(body_bytes));

    while let Some(log) = build_stream.next().await {
        match log {
            Ok(output) => println!("{:?}", output),
            Err(e) => return Err(format!("Docker build failed: {}", e)),
        }
    }

    if let Err(e) = std::fs::remove_file(&tar_path) {
        eprintln!("Warning: Failed to clean up tar file: {}", e);
    } else {
        println!("Successfully cleaned up tar file: {}", tar_path);
    }

    Ok(())
}

/// Runs the Docker Compose command to deploy the application.
/// Creates and runs a Docker container from the specified image.
///
/// # Arguments
/// * `app_name` - The name of the Docker image.
///
/// # Returns
/// * `Ok(())` if the Docker Compose command was successful.
/// * `Err(String)` if there was an error during execution.
pub fn start_docker_compose() -> Result<(), String> {
    let status = Command::new("docker")
        .current_dir("src")
        .arg("compose")
        .arg("up")
        .arg("-d")
        .status()
        .map_err(|e| format!("Failed to execute docker compose: {}", e))?;

    if !status.success() {
        return Err("Docker Compose command failed".to_string());
    }

    Ok(())
}

/// * `Ok(())` if successful.
/// * `Err(String)` if an error occurs.
pub async fn create_and_run_container(app_name: &str) -> Result<(), String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    let container_name = format!("{}-{}", app_name, Uuid::new_v4());

    let mut exposed_ports = HashMap::new();
    exposed_ports.insert("3000/tcp".to_string(), HashMap::new());

    let mut port_bindings = HashMap::new();
    port_bindings.insert(
        "3000/tcp".to_string(),
        Some(vec![PortBinding {
            host_ip: None,
            host_port: Some("3000".to_string()),
        }]),
    );

    let mut log_opts = HashMap::new();
    log_opts.insert("max-size".to_string(), "10m".to_string());
    log_opts.insert("max-file".to_string(), "3".to_string());

    let log_config = HostConfigLogConfig {
        typ: Some("json-file".to_string()), // Log driver type
        config: Some(log_opts),             // Log options
    };

    let config = Config::<String> {
        image: Some(format!("{}:latest", app_name)),
        exposed_ports: Some(exposed_ports),
        host_config: Some(HostConfig {
            port_bindings: Some(port_bindings),
            log_config: Some(log_config), // Use HostConfigLogConfig here
            ..Default::default()
        }),
        ..Default::default()
    };

    docker
        .create_container(
            Some(CreateContainerOptions {
                name: &container_name,
                platform: Some(&"linux/amd64".to_string()),
            }),
            config,
        )
        .await
        .map_err(|e| format!("Failed to create container: {}", e))?;

    docker
        .start_container(&container_name, None::<StartContainerOptions<String>>)
        .await
        .map_err(|e| format!("Failed to start container: {}", e))?;

    Ok(())
}


/// Stops the running container for the given application.
///
/// Executes the `docker stop` command to stop the container with the given name.
///
/// # Arguments
///
/// * `container_name` - The name of the container to stop.
///
/// # Returns
///
/// A `Result` indicating success or an error message in case of failure.

pub async fn stop_container(container_name: &str) -> Result<(), String> {

    let output = Command::new("docker")
    .args(&["stop", container_name])
    .output()
    .map_err(|e| format!("Failed to execute docker stop: {}", e))?;
    Ok(())
 }


/// Removes the container for the given application.
///
/// Executes the `docker rm` command to remove the container with the given name.
///
/// # Arguments
///
/// * `container_name` - The name of the container to remove.
///
/// # Returns
///
/// A `Result` indicating success or an error message in case of failure.

pub async fn remove_container(container_name: &str) -> Result<(), String> {
    
    let output = Command::new("docker")
    .args(&["rm", container_name])
    .output()
    .map_err(|e| format!("Failed to execute docker stop: {}", e))?;
    Ok(())
 }
