use bollard::auth::DockerCredentials;
use bollard::container::{ListContainersOptions, StopContainerOptions};
use bollard::image::{BuildImageOptions, PruneImagesOptions, PushImageOptions, TagImageOptions};
use bollard::service::{InspectServiceOptions, ListServicesOptions};
use bollard::Docker;
use chrono::Utc;
use dirs::home_dir;
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::net::IpAddr;
use std::path::Path;
use std::process::Command;
use tar::Builder;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
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

    let filters: HashMap<String, Vec<String>> = HashMap::new();

    let options = Some(ListServicesOptions{
        filters,
        ..Default::default()
    });

    let services = docker
        .list_services(options)
        .await
        .map_err(|e| format!("Failed to list services: {}", e))?;

    let mut apps = Vec::new();



    let inspect_options = Some(InspectServiceOptions{
        insert_defaults: true,
    });


    // Iterate over containers and check for custom labels
    for service in services {
        if let Some(spec) = docker.inspect_service(service.id.as_ref().unwrap(), inspect_options).await.unwrap().spec {
            if let Some(labels) = spec.labels {

                if let (Some(name), Some(app_type), Some(url), Some(domain), Some(created)) = (
                    labels.get("com.myapp.name"),
                    labels.get("com.myapp.type"),
                    labels.get("com.myapp.github_url"),
                    labels.get("com.myapp.domain"),
                    labels.get("com.myapp.created_at"),
                ) {
                    let app_status = get_app_status(name.to_string()).await;

                    // Collect app info, handle Option<String> for container.id
                    apps.push(AppInfo {
                        app_name: name.clone(),
                        app_type: app_type.clone(),
                        github_url: url.clone(),
                        domain: domain.clone(),
                        created_at: created.clone(),
                        status: app_status,
                        container_id: Some(
                            service
                                .id
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string()),
                        ),
                    });
                }
            }
        }
    }

    // Sort by creation date, newest first
    apps.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(apps)
}

pub async fn get_app_status(name: String) -> String {
    let mut app_status: &str = "unknown";

    if let Ok(res) = is_app_running(name).await {
        if res {
            app_status = "running";
        }
    }
    app_status.to_string()
}

async fn is_app_running(name: String) -> Result<bool, String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    let containers = docker
        .list_containers(Some(ListContainersOptions {
            filters: {
                let mut filters = HashMap::new();
                filters.insert("label".to_string(), vec![format!("com.myapp.name={}", name.clone())]);
                filters
            },
            ..Default::default()
        }))
        .await
        .map_err(|e| format!("Failed to list containers: {}", e))?;

    for container in containers {
        if let Some(state) = container.state {
            if state == "running" {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Creates a Docker context tarball for the specified application path.
///
/// # Arguments
/// * `app_path` - The path to the application directory.
///
/// # Returns
/// * `Ok(String)` containing the path to the created tarball.
/// * `Err(String)` if there is an error.
fn create_docker_context(app_name: &str, app_path: &str) -> Result<String, String> {
    let app_dir = Path::new(app_path)
        .canonicalize()
        .map_err(|e| format!("Invalid application path: {}", e))?;

    if !app_dir.exists() || !app_dir.is_dir() {
        return Err(format!("Invalid application path: {}", app_path));
    }

    let home = home_dir().ok_or("Failed to find home directory")?;
    let tar_path = home.join(format!(
        ".cache/nephelios/{}.tar",
        app_name
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

    let tar_path = create_docker_context(app_name, app_path).map_err(|e| format!("Error: {}", e))?;
    let mut tar_file = File::open(&tar_path)
        .map_err(|e| format!("Failed to open tar file: {}", e))?;

    let mut contents = Vec::new();
    tar_file.read_to_end(&mut contents)
        .map_err(|e| format!("Failed to read tar file: {}", e))?;

    let options = BuildImageOptions {
        t: format!("localhost:5000/{}:latest",app_name.to_lowercase()),
        rm: true,
        labels: metadata.to_labels(),
        ..Default::default()
    };

    let mut build_stream = docker.build_image(options, None, Some(contents.into()));

    while let Some(build_result) = build_stream.next().await {
        match build_result {

            Ok(output) => {
                if let Some(stream) = output.stream {
                    println!("Build Info: {}", stream);
                }
                if let Some(error) = output.error {
                    eprintln!("Error: {}", error);
                }
            }
            Err(e) => {
                eprintln!("Error during build: {}", e);
            }
        }
    }

    if let Err(e) = std::fs::remove_file(&tar_path) {
        eprintln!("Warning: Failed to clean up tar file: {}", e);
    } else {
        println!("Successfully cleaned up tar file: {}", tar_path);
    }

    Ok(())
}
/// Pushes a Docker image to a remote registry.
///
/// # Arguments
///
/// * `app_name` - The name of the Docker image to push.
///
/// # Returns
///
/// * `Ok(())` if the image was successfully pushed.
/// * `Err(String)` if there was an error during the push process.
pub async fn push_image(app_name: &str) -> Result<(), String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    // Nom de l'image locale
    let local_image = format!("localhost:5000/{}:latest", app_name.to_lowercase());
    // Nom de l'image avec le registre
    let remote_image = format!("localhost:5000/{}:latest", app_name.to_lowercase());

    // Taguer l'image pour le registre
    let tag_options = TagImageOptions {
        repo: remote_image.clone(),
        tag: "latest".parse().unwrap(),
    };
    docker
        .tag_image(&local_image, Some(tag_options))
        .await
        .map_err(|e| format!("Failed to tag image: {}", e))?;

    // Pousser l'image vers le registre
    let push_options = PushImageOptions {
        tag: "latest",
        ..Default::default()
    };

    // Si votre registre nécessite une authentification, fournissez les identifiants
    let credentials = Some(DockerCredentials {
        ..Default::default()
    });

    let mut push_stream = docker.push_image(&*remote_image, Some(push_options), credentials);

    while let Some(push_stream) = push_stream.next().await {
        match push_stream {
            Ok(output) => {
                if let Some(stream) = output.progress {
                    println!("Push Image info: {}", stream);
                }
                if let Some(error) = output.error {
                    eprintln!("Error: {}", error);
                }
            }
            Err(e) => {
                eprintln!("Error pushing image: {}", e);
            }
        }
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
pub fn deploy_nephelios_stack() -> Result<(), String> {
    let status = Command::new("docker")
        .current_dir("./")
        .arg("stack")
        .arg("deploy")
        .arg("-c")
        .arg("docker-compose.yml")
        .arg("nephelios")
        .status()
        .map_err(|e| format!("Failed to execute docker compose: {}", e))?;

    if !status.success() {
        return Err("Docker Compose command failed".to_string());
    }

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
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;
    let options = Some(StopContainerOptions { t: 30 });
    docker
        .stop_container(container_name, options)
        .await
        .map_err(|e| format!("Failed to start container: {}", e))?;

    Ok(())
}

/// Removes the container for the given application.
///
/// Executes the `docker rm` command to remove the container with the given name.
///
/// # Arguments
///
/// * `app_name` - The name of the container to remove.
///
/// # Returns
///
/// A `Result` indicating success or an error message in case of failure.

pub async fn remove_service(app_name: &str) -> Result<(), String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    let service_name: &str = &format!("nephelios_{}", app_name);

    println!("Removing service: {}", service_name);

    docker
        .delete_service(service_name)
        .await
        .map_err(|e| format!("Failed to start container: {}", e))?;
    Ok(())
}

/// Leaves the Docker Swarm.
///
/// Executes the `docker swarm leave -f` command to forcefully leave the Docker Swarm.
///
/// # Returns
///
/// * `Ok(())` if the command was successful.
/// * `Err(String)` if there was an error during execution.
pub fn leave_swarm() -> Result<(), String> {
    let status = Command::new("docker")
        .arg("swarm")
        .arg("leave")
        .arg("-f")
        .status()
        .map_err(|e| format!("Failed to execute leave swarm: {}", e))?;

    if !status.success() {
        return Err("Docker Compose command failed".to_string());
    }

    Ok(())
}


/// Stops the Nephelios stack by removing the Docker stack.
///
/// # Returns
///
/// * `Ok(())` if the stack was successfully stopped.
/// * `Err(String)` if there was an error during the process.
pub fn stop_nephelios_stack() -> Result<(), String> {

    let status = Command::new("docker")
        .arg("stack")
        .arg("rm")
        .arg("nephelios")
        .status()
        .map_err(|e| format!("Failed to execute remove Nephelios: {}", e))?;

    if !status.success() {
        return Err("Docker Compose command failed".to_string());
    }

    Ok(())
}


/// Initializes Docker Swarm with the given IP address.
///
/// # Arguments
///
/// * `ip_addr` - The IP address to advertise for the Docker Swarm.
///
/// # Returns
///
/// * `Ok(())` if the Docker Swarm was successfully initialized.
/// * `Err(String)` if there was an error during initialization.
pub fn init_swarm(ip_addr: IpAddr) -> Result<(), String> {
    let addr_parameter = format!("--advertise-addr={}", env::var("ADVERTISE_ADDR").unwrap_or_else(|_| {
        // Specify a default IP address if ADVERTISE_ADDR is not set
        ip_addr.to_string()
    }));

    println!("Init swarm with address: {}", addr_parameter);
    let status = Command::new("docker")
        .arg("swarm")
        .arg("init")
        .arg(addr_parameter)
        .status()
        .map_err(|e| format!("Failed to execute init swarm: {}", e))?;

    if !status.success() {
        return Err("Docker Compose command failed".to_string());
    }

    Ok(())
}

/// Checks if Docker Swarm is active.
///
/// Executes the `docker info` command and checks the output for the presence of "Swarm: active".
///
/// # Returns
///
/// * `Ok(true)` if Docker Swarm is active.
/// * `Ok(false)` if Docker Swarm is not active.
/// * `Err(String)` if there was an error during execution.
pub fn check_swarm() -> Result<bool, String> {
    let swarm_info = Command::new("docker")
        .arg("info")
        .output()
        .map_err(|e| format!("Failed to execute docker info: {}", e))?;

    Ok(String::from_utf8_lossy(&swarm_info.stdout).contains("Swarm: active"))
}
/// Prunes unused Docker images.
///
/// Connects to the local Docker daemon and removes all dangling images.
///
/// # Returns
///
/// * `Ok(())` if the images were successfully pruned.
/// * `Err(String)` if there was an error during the pruning process.
pub async fn prune_images() -> Result<(), String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    let filters: HashMap<String, Vec<String>> = HashMap::new();
    let options = Some(PruneImagesOptions {
        filters
    });

    let result = docker
        .prune_images(options)
        .await
        .map_err(|e| format!("Failed to prune images: {}", e))?;

    match &result.images_deleted {
        None => println!("No images deleted"),
        Some(images_deleted) => {
            for image in images_deleted {
                match &image.deleted {
                    None => {}
                    Some(deleted) => println!("Deleted image: {}", deleted)
                }
            }
        }
    }

    Ok(())
}