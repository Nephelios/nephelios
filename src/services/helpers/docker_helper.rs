use crate::metrics::{CONTAINER_CPU, CONTAINER_MEM, CONTAINER_NET_IN, CONTAINER_NET_OUT};
use bollard::auth::DockerCredentials;
use bollard::container::ListContainersOptions;
use bollard::image::{BuildImageOptions, PruneImagesOptions, PushImageOptions, TagImageOptions};
// Removed unused service imports
use bollard::Docker;
use chrono::Utc;
use dirs::home_dir;
use futures_util::stream::StreamExt;
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
    pub swarm_task_name: Option<String>,
}

pub async fn list_deployed_apps() -> Result<Vec<AppInfo>, String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    // Create filters to get all containers
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .map_err(|e| format!("Failed to list containers: {}", e))?;

    let mut apps = Vec::new();

    // Iterate over containers and check for nephelios namespace
    for container in containers {
        // Get detailed container information
        if let Some(container_id) = &container.id {
            let inspect_result = docker
                .inspect_container(container_id, None)
                .await
                .map_err(|e| format!("Failed to inspect container: {}", e));

            if let Ok(inspect_data) = inspect_result {
                // Check if the container has the required labels
                if let Some(labels) = inspect_data.config.and_then(|c| c.labels) {
                    // First check if this container belongs to the nephelios stack
                    if let Some(namespace) = labels.get("com.docker.stack.namespace") {
                        if namespace == "nephelios" {
                            // Then check for our app labels
                            if let (
                                Some(name),
                                Some(app_type),
                                Some(url),
                                Some(domain),
                                Some(created),
                            ) = (
                                labels.get("com.myapp.name"),
                                labels.get("com.myapp.type"),
                                labels.get("com.myapp.github_url"),
                                labels.get("com.myapp.domain"),
                                labels.get("com.myapp.created_at"),
                            ) {
                                let app_status = get_app_status(name.to_string()).await;

                                // Use the task ID as container_id if available
                                let task_name = labels
                                    .get("com.docker.swarm.task.name")
                                    .map(|id| id.clone())
                                    .unwrap_or_else(|| container_id.clone());

                                // Collect app info
                                apps.push(AppInfo {
                                    app_name: name.clone(),
                                    app_type: app_type.clone(),
                                    github_url: url.clone(),
                                    domain: domain.clone(),
                                    created_at: created.clone(),
                                    status: app_status,
                                    swarm_task_name: Some(task_name),
                                });
                            }
                        }
                    }
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

pub async fn get_app_details(name: String) -> (String, Option<String>) {
    let status = get_app_status(name.clone()).await;
    let mut swarm_name = None;
    
    // Get container details to extract swarm task name
    let docker = match Docker::connect_with_local_defaults() {
        Ok(docker) => docker,
        Err(_) => return (status, None),
    };

    // Create filters to find the specific container by app name
    let mut filters = HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![format!("com.myapp.name={}", name)],
    );

    let options = Some(ListContainersOptions {
        filters,
        ..Default::default()
    });

    let containers = match docker.list_containers(options).await {
        Ok(containers) => containers,
        Err(_) => return (status, None),
    };

    // Extract swarm task name from labels if available
    for container in containers {
        if let Some(labels) = container.labels {
            if let Some(task_name) = labels.get("com.docker.swarm.task.name") {
                swarm_name = Some(task_name.clone());
                break;
            }
            // Also check for service name which might be more useful
            if let Some(service_name) = labels.get("com.docker.swarm.service.name") {
                swarm_name = Some(service_name.clone());
                break;
            }
        }
    }

    (status, swarm_name)
}

async fn is_app_running(name: String) -> Result<bool, String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    let containers = docker
        .list_containers(Some(ListContainersOptions {
            filters: {
                let mut filters = HashMap::new();
                filters.insert(
                    "label".to_string(),
                    vec![format!("com.myapp.name={}", name.clone())],
                );
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
    let tar_path = home.join(format!(".cache/nephelios/{}.tar", app_name));

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
/// * `metadata` - The application metadata.
/// * `install_command` - Custom install command from the frontend.
/// * `run_command` - Custom run command from the frontend.
/// * `build_command` - Custom build command from the frontend.
/// * `app_workdir` - Working directory for the application in the container.
/// * `additional_inputs` - Optional additional environment variables and settings.
///
/// # Returns
/// * `Ok(())` if successful.
/// * `Err(String)` if an error occurs.
pub fn generate_and_write_dockerfile(
    app_type: &str,
    app_path: &str,
    metadata: &AppMetadata,
    install_command: &str,
    run_command: &str,
    build_command: &str,
    app_workdir: &str,
    additional_inputs: Option<&HashMap<String, String>>,
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

    // Generate environment variables from additional_inputs
    let env_vars = additional_inputs
        .map(|inputs| {
            inputs
                .iter()
                .map(|(k, v)| format!("ENV {}=\"{}\"", k, v))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    let dockerfile_content = match app_type {
        "nodejs" => {
            // Detect which package manager is being used
            let uses_npm = install_command.contains("npm")
                || build_command.contains("npm")
                || run_command.contains("npm");

            let uses_yarn = install_command.contains("yarn")
                || build_command.contains("yarn")
                || run_command.contains("yarn");

            let uses_pnpm = install_command.contains("pnpm")
                || build_command.contains("pnpm")
                || run_command.contains("pnpm");

            // Determine which package manager to use
            let package_manager = if uses_yarn {
                "yarn"
            } else if uses_pnpm {
                "pnpm"
            } else if uses_npm {
                "npm"
            } else {
                "bun"
            };

            // Choose the base image based on the package manager
            let base_image = match package_manager {
                "yarn" => "node:18-alpine".to_string(),
                "pnpm" => "node:18-alpine".to_string(),
                "npm" => "node:18-alpine".to_string(),
                _ => "oven/bun:latest".to_string(),
            };

            // Additional setup commands for package managers
            let setup_cmd = match package_manager {
                "yarn" => {
                    "RUN apk add --no-cache curl && curl -o- -L https://yarnpkg.com/install.sh | sh"
                        .to_string()
                }
                "pnpm" => "RUN npm install -g pnpm".to_string(),
                _ => "".to_string(), // No additional setup for npm or bun
            };

            // Determine the appropriate install command based on the package manager
            let install_cmd = if !install_command.is_empty() {
                install_command.to_string()
            } else {
                match package_manager {
                    "yarn" => "yarn install --production".to_string(),
                    "pnpm" => "pnpm install --prod".to_string(),
                    "npm" => "npm install --production".to_string(),
                    _ => "bun install --production".to_string(),
                }
            };

            let build_cmd = if !build_command.is_empty() {
                format!("RUN {}", build_command)
            } else {
                "".to_string()
            };

            let run_cmd = if !run_command.is_empty() {
                format!("CMD [\"sh\", \"-c\", \"{}\"]", run_command)
            } else {
                match package_manager {
                    "yarn" => "CMD [\"sh\", \"-c\", \"if yarn dev 2>/dev/null; then yarn dev; else yarn start; fi\"]".to_string(),
                    "pnpm" => "CMD [\"sh\", \"-c\", \"if pnpm dev 2>/dev/null; then pnpm dev; else pnpm start; fi\"]".to_string(),
                    "npm" => "CMD [\"sh\", \"-c\", \"if npm run dev 2>/dev/null; then npm run dev; else npm start; fi\"]".to_string(),
                    _ => "CMD [\"sh\", \"-c\", \"if bun dev 2>/dev/null; then bun dev; else bun start; fi\"]".to_string()
                }
            };

            format!(
                r#"FROM {}
WORKDIR {}
{}
{}
{}
COPY package.json ./
RUN {}
COPY . .
{}
EXPOSE {}
{}"#,
                base_image,
                app_workdir,
                labels,
                env_vars,
                setup_cmd,
                install_cmd,
                build_cmd,
                deploy_port,
                run_cmd
            )
        }
        "python" => {
            // Determine the appropriate commands based on provided values
            let install_cmd = if !install_command.is_empty() {
                install_command.to_string()
            } else {
                "pip install --no-cache-dir -r requirements.txt".to_string()
            };

            let build_cmd = if !build_command.is_empty() {
                format!("RUN {}", build_command)
            } else {
                "".to_string()
            };

            let run_cmd = if !run_command.is_empty() {
                format!("CMD [\"sh\", \"-c\", \"{}\"]", run_command)
            } else {
                "CMD [\"python\", \"app.py\"]".to_string()
            };

            format!(
                r#"FROM python:3.8-slim
WORKDIR {}
{}
{}
COPY requirements.txt ./
RUN {}
COPY . .
{}
EXPOSE {}
{}"#,
                app_workdir, labels, env_vars, install_cmd, build_cmd, deploy_port, run_cmd
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

    let tar_path =
        create_docker_context(app_name, app_path).map_err(|e| format!("Error: {}", e))?;
    let mut tar_file =
        File::open(&tar_path).map_err(|e| format!("Failed to open tar file: {}", e))?;

    let mut contents = Vec::new();
    tar_file
        .read_to_end(&mut contents)
        .map_err(|e| format!("Failed to read tar file: {}", e))?;

    let options = BuildImageOptions {
        t: format!("{}:latest", app_name.to_lowercase()),
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

    // Local image name (without registry)
    let local_image = format!("{}:latest", app_name.to_lowercase());
    // Remote image name (with registry)
    let remote_image = format!("registry:5000/{}", app_name.to_lowercase());

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
    let push_options = PushImageOptions { tag: "latest" };

    // Si votre registre nécessite une authentification, fournissez les identifiants
    let credentials = Some(DockerCredentials {
        ..Default::default()
    });

    let mut push_stream = docker.push_image(&remote_image, Some(push_options), credentials);

    while let Some(push_stream) = push_stream.next().await {
        match push_stream {
            Ok(output) => {
                if let Some(stream) = output.progress {
                    match serde_json::from_str::<serde_json::Value>(&stream) {
                        Ok(value) => {
                            if let Some(status) = value.get("status") {
                                println!("Push Image info: {}", status);
                            }
                        }
                        Err(_) => {
                            println!("Push Image info: {}", stream);
                        }
                    }
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
/// Connects the Nephelios container to the overlay network after Swarm initialization
///
/// This function uses the Docker API to:
/// 1. Find the Nephelios container
/// 2. Connect it to the nephelios_overlay network
///
/// # Returns
/// * `Ok(())` if the connection was successful
/// * `Err(String)` if there was an error during the process
/// Disconnects the Nephelios container from the overlay network during cleanup
///
/// This function uses the Docker API to:
/// 1. Find the Nephelios container
/// 2. Disconnect it from the nephelios_overlay network
///
/// # Returns
/// * `Ok(())` if the disconnection was successful
/// * `Err(String)` if there was an error during the process
pub async fn disconnect_from_overlay_network() -> Result<(), String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    // Find the Nephelios container using its unique label
    let mut filters = HashMap::new();
    filters.insert("label", vec!["com.nephelios.name=nephelios"]);

    let options = Some(ListContainersOptions {
        filters,
        ..Default::default()
    });

    let containers = docker
        .list_containers(options)
        .await
        .map_err(|e| format!("Failed to list containers: {}", e))?;

    let container = containers
        .first()
        .ok_or("Nephelios container not found".to_string())?;

    let container_id = container
        .id
        .as_ref()
        .ok_or("Container ID not found".to_string())?;

    // Disconnect from the overlay network
    docker
        .disconnect_network(
            "nephelios_overlay",
            bollard::network::DisconnectNetworkOptions {
                container: container_id.to_string(),
                force: true,
            },
        )
        .await
        .map_err(|e| format!("Failed to disconnect from overlay network: {}", e))?;

    Ok(())
}

/// Connects the Nephelios container to the `nephelios_overlay` network.
///
/// This function locates the Nephelios container using its label and connects it to the
/// specified Docker overlay network.
///
/// # Returns
/// * `Ok(())` if successful.
/// * `Err(String)` if an error occurs during connection or container lookup.

pub async fn connect_to_overlay_network() -> Result<(), String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    // Find the Nephelios container using its unique label
    let mut filters = HashMap::new();
    filters.insert("label", vec!["com.nephelios.name=nephelios"]);

    let options = Some(ListContainersOptions {
        filters,
        ..Default::default()
    });

    let containers = docker
        .list_containers(options)
        .await
        .map_err(|e| format!("Failed to list containers: {}", e))?;

    let container = containers
        .first()
        .ok_or("Nephelios container not found".to_string())?;

    let container_id = container
        .id
        .as_ref()
        .ok_or("Container ID not found".to_string())?;

    // Connect to the overlay network
    docker
        .connect_network(
            "nephelios_overlay",
            bollard::network::ConnectNetworkOptions {
                container: container_id.to_string(),
                endpoint_config: bollard::models::EndpointSettings::default(),
            },
        )
        .await
        .map_err(|e| format!("Failed to connect to overlay network: {}", e))?;

    Ok(())
}

/// Deploys the Nephelios stack using the `docker stack deploy` command.
///
/// This function runs the `docker stack deploy` command with the `nephelios.yml` file
/// to deploy the Nephelios stack.
///
/// # Returns
/// * `Ok(())` if the deployment is successful.
/// * `Err(String)` if the deployment command fails.

pub fn deploy_nephelios_stack() -> Result<(), String> {
    let status = Command::new("docker")
        .current_dir("./")
        .arg("stack")
        .arg("deploy")
        .arg("-c")
        .arg("nephelios.yml")
        .arg("nephelios")
        .status()
        .map_err(|e| format!("Failed to deploy Nephelios Stack : {}", e))?;

    if !status.success() {
        return Err("Deploy stack command failed".to_string());
    }

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
    let addr_parameter = format!(
        "--advertise-addr={}",
        env::var("ADVERTISE_ADDR").unwrap_or_else(|_| {
            // Specify a default IP address if ADVERTISE_ADDR is not set
            ip_addr.to_string()
        })
    );

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
    let options = Some(PruneImagesOptions { filters });

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
                    Some(deleted) => println!("Deleted image: {}", deleted),
                }
            }
        }
    }

    Ok(())
}

/// Scales a Docker service.
///
/// This function modifies the number of replicas for a given Docker service by executing
/// the `docker service scale` command. The service name is dynamically constructed using
/// the provided application name and identifier.
///
/// # Arguments
///
/// * `app_name` - A string slice that represents the application name.
/// * `id` - A string slice that represents the identifier used to scale the service.
///
/// # Returns
///
/// * `Ok(())` if the scaling operation was successful.
/// * `Err(String)` if there was an error executing the Docker command.
///
/// # Errors
///
/// This function returns an error if the `docker` command fails to execute
/// or if the scaling operation does not complete successfully.
pub async fn scale_app(app_name: &str, id: &str) -> Result<(), String> {
    let scale_arg = format!("nephelios_{}={}", app_name, id); // Concaténer le nom et "=0"

    let status = Command::new("docker")
        .current_dir("./")
        .arg("service")
        .arg("scale")
        .arg(&scale_arg) // Passer l'argument correctement
        .status()
        .map_err(|e| format!("Failed to execute docker command: {}", e))?;

    if !status.success() {
        return Err("Docker service scale command failed".to_string());
    }

    Ok(())
}

/// Parses the network I/O string from Docker stats.
///
/// This function takes a string formatted like "42kB / 252B", representing
/// incoming and outgoing network data. It splits the string and converts
/// each part to kilobytes.
///
/// # Arguments
///
/// * `net_io` - A string slice representing the network I/O, e.g., "42kB / 252B".
///
/// # Returns
///
/// A tuple `(f64, f64)` representing `(net_in_kb, net_out_kb)`.

fn parse_network_io(net_io: &str) -> (f64, f64) {
    // Format is typically like "42kB / 252B"
    let parts: Vec<&str> = net_io.split('/').collect();
    if parts.len() != 2 {
        return (0.0, 0.0);
    }

    let in_str = parts[0].trim();
    let out_str = parts[1].trim();

    let net_in = parse_data_size(in_str);
    let net_out = parse_data_size(out_str);

    // Convert to KB for consistent metrics
    (net_in, net_out)
}

/// Parses a human-readable data size string into kilobytes.
///
/// This function supports units such as B, KB, MB, GB, and TB,
/// and converts them to kilobytes for consistent internal usage.
///
/// # Arguments
///
/// * `size_str` - A string slice like "42kB", "1.2MB", etc.
///
/// # Returns
///
/// The size converted to kilobytes (`f64`).
/// 
fn parse_data_size(size_str: &str) -> f64 {
    let re = regex::Regex::new(r"([0-9.]+)\s*([a-zA-Z]+)").unwrap();
    if let Some(caps) = re.captures(size_str) {
        let value: f64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0.0);
        let unit = caps.get(2).unwrap().as_str().to_lowercase();

        // Convert to KB
        match unit.as_str() {
            "b" => value / 1024.0,
            "kb" => value,
            "mb" => value * 1024.0,
            "gb" => value * 1024.0 * 1024.0,
            "tb" => value * 1024.0 * 1024.0 * 1024.0,
            _ => value,
        }
    } else {
        0.0
    }
}

/// Updates Prometheus metrics by parsing `docker stats`, filtering only `nephelios` containers.
///
/// This asynchronous function executes `docker stats --no-stream` to gather
/// live statistics about running Docker containers. It parses the JSON output
/// and updates Prometheus metrics for CPU usage, memory usage, and network I/O
/// — **but only for containers whose names start with `nephelios`**.
///
/// # Behavior
///
/// - Resets all container metrics before collecting new ones.
/// - Filters out any container whose name does not begin with `"nephelios"`.
/// - Parses each stat field and updates the corresponding Prometheus gauges.
///
/// # Returns
///
/// * `Ok(())` on successful metrics update.
/// * `Err` if the command execution or data parsing fails.
///
/// # Errors
///
/// Returns an error if:
/// - The `docker stats` command fails to execute,
/// - The output is not valid UTF-8,
/// - The JSON parsing fails for any line.

pub async fn update_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("docker")
        .arg("stats")
        .arg("--no-stream")
        .arg("--format")
        .arg("{{json .}}")
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let lines = stdout.lines();

    CONTAINER_CPU.reset();
    CONTAINER_MEM.reset();
    CONTAINER_NET_IN.reset();
    CONTAINER_NET_OUT.reset();

    for line in lines {
        let data: serde_json::Value = serde_json::from_str(line)?;
        let name = data["Name"].as_str().unwrap_or("unknown");

        if !name.starts_with("nephelios") {
            continue;
        }

        let cpu = parse_percentage(data["CPUPerc"].as_str().unwrap_or("0%"));
        let mem = parse_memory(data["MemUsage"].as_str().unwrap_or("0MiB / 0MiB"));
        let (net_in, net_out) = parse_network_io(data["NetIO"].as_str().unwrap_or("0kB / 0B"));

        CONTAINER_CPU.with_label_values(&[name]).set(cpu);
        CONTAINER_MEM.with_label_values(&[name]).set(mem);
        CONTAINER_NET_IN.with_label_values(&[name]).set(net_in);
        CONTAINER_NET_OUT.with_label_values(&[name]).set(net_out);
    }

    Ok(())
}

/// Parses a percentage string like "42.5%" into a floating-point value.
///
/// # Arguments
///
/// * `val` - A string slice representing the percentage (e.g., "42.5%").
///
/// # Returns
///
/// A `f64` value of the percentage, or 0.0 if parsing fails.

fn parse_percentage(val: &str) -> f64 {
    val.trim_end_matches('%').parse::<f64>().unwrap_or(0.0)
}

/// Parses memory usage from a Docker-formatted string.
///
/// It extracts the first part of the memory usage string (e.g., "512MiB / 2GiB")
/// and converts it to a floating-point value, currently only stripping the unit.
///
/// # Arguments
///
/// * `val` - A string slice in the format "XMiB / YMiB".
///
/// # Returns
///
/// A `f64` value representing the memory usage in MiB.

fn parse_memory(val: &str) -> f64 {
    val.split('/')
        .next()
        .unwrap_or("0")
        .trim()
        .replace("MiB", "")
        .replace("GiB", "")
        .parse::<f64>()
        .unwrap_or(0.0)
}
