use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
};
use bollard::image::BuildImageOptions;
use bollard::models::HostConfigLogConfig;
use bollard::service::{HostConfig, PortBinding};
use bollard::Docker;
use dirs::home_dir;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
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
use warp::hyper::Body;

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
pub fn generate_and_write_dockerfile(app_type: &str, app_path: &str) -> Result<(), String> {
    let dockerfile_path = Path::new(app_path).join("Dockerfile");

    if dockerfile_path.exists() {
        println!("Dockerfile already exists at {}", dockerfile_path.display());
        return Ok(());
    }

    let dockerfile_content = match app_type {
        "nodejs" => {
            r#"
        FROM oven/bun:latest
        WORKDIR /app
        COPY package.json ./
        RUN bun install --production
        COPY . .
        EXPOSE 3000
        CMD ["sh", "-c", "if bun dev 2>/dev/null; then bun dev; else bun start; fi"]
        "#
        }
        "python" => {
            r#"
        FROM python:3.8-slim
        WORKDIR /app
        COPY requirements.txt ./
        RUN pip install --no-cache-dir -r requirements.txt
        COPY . .
        EXPOSE 5000
        CMD ["python", "app.py"]
        "#
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
pub async fn build_image(app_name: &str, app_path: &str) -> Result<(), String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    let tar_path = create_docker_context(app_path).map_err(|e| format!("Error: {}", e))?;
    let tar_file = TokioFile::open(&tar_path)
        .await
        .map_err(|e| format!("Failed to open tar file: {}", e))?;

    let tar_stream =
        FramedRead::new(tar_file, BytesCodec::new()).map(|res| res.map(|b| b.freeze()));
    let body = Body::wrap_stream(tar_stream);

    let options = BuildImageOptions {
        t: app_name,
        rm: true,
        ..Default::default()
    };

    let mut build_stream = docker.build_image(options, None, Some(body));

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

/// Removes a Docker container.
///
/// # Arguments
/// * `container_name` - The name of the container to remove.
///
/// # Returns
/// * `Ok(())` if successful.
/// * `Err(String)` if an error occurs.
pub async fn remove_container(container_name: &str) -> Result<(), String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    docker
        .remove_container(
            container_name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await
        .map_err(|e| format!("Failed to remove container {}: {}", container_name, e))?;

    println!("Container {} removed successfully.", container_name);
    Ok(())
}
