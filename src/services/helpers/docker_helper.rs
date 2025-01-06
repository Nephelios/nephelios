use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

/// Generates a Dockerfile for the specified application type and writes it to the app's repository.
///
/// # Arguments
///
/// * `app_type` - The type of the application (e.g., "nodejs" or "python").
/// * `app_path` - The path to the application repository where the Dockerfile will be written.
///
/// # Returns
/// * `Ok(())` if the Dockerfile was created and written successfully.
/// * `Err(String)` if there was an error during the creation or writing of the Dockerfile.
pub fn generate_and_write_dockerfile(app_type: &str, app_path: &str) -> Result<(), String> {
    let dockerfile_content = match app_type {
        "nodejs" => {
            r#"
            FROM node:14
            WORKDIR /app
            COPY package*.json ./
            RUN npm install
            COPY . .
            CMD ["node", "index.js"]
            "#
        }
        "python" => {
            r#"
            FROM python:3.8-slim
            WORKDIR /app
            COPY requirements.txt ./
            RUN pip install --no-cache-dir -r requirements.txt
            COPY . .
            CMD ["python", "app.py"]
            "#
        }
        _ => return Err(format!("Unsupported app type: {}", app_type)),
    };

    let dockerfile_path = Path::new(app_path).join("Dockerfile");
    println!("Writing Dockerfile to {}", dockerfile_path.display());
    let mut file = File::create(&dockerfile_path)
        .map_err(|e| format!("Failed to create Dockerfile: {}", e))?;
    file.write_all(dockerfile_content.as_bytes())
        .map_err(|e| format!("Failed to write Dockerfile: {}", e))?;
    Ok(())
}

/// Builds a Docker image using the local Docker CLI based on the provided Dockerfile.
///
/// # Arguments
///
/// * `app_name` - The name of the application (used as the image name).
/// * `app_path` - The path to the application directory where the Dockerfile is located.
///
/// # Returns
/// * `Ok(())` if the image was built successfully.
/// * `Err(String)` if the Docker build failed or there was an issue invoking the Docker CLI.
pub fn build_image(app_name: &str, app_path: &str) -> Result<(), String> {
    let status = Command::new("docker")
        .args(["build", "-t", &format!("{}:latest", app_name), app_path])
        .status()
        .map_err(|e| format!("Failed to invoke Docker CLI: {}", e))?;

    if !status.success() {
        return Err("Docker build failed. Check Dockerfile and logs.".to_string());
    }
    Ok(())
}

/// Runs a Docker container from the built image using the local Docker CLI.
///
/// # Arguments
///
/// * `app_name` - The name of the application used for creating the container and identifying the image.
///
/// # Returns
/// * `Ok(())` if the container was started successfully.
/// * `Err(String)` if there was an issue starting the Docker container or invoking the Docker CLI.
pub fn create_and_run_container(app_name: &str) -> Result<(), String> {
    let container_name = format!("{}-{}", app_name, uuid::Uuid::new_v4());
    let status = Command::new("docker")
        .args([
            "run",
            "-d",
            "-p",
            "3000:3000",
            "--name",
            &container_name,
            &format!("{}:latest", app_name),
        ])
        .status()
        .map_err(|e| format!("Failed to invoke Docker CLI: {}", e))?;

    if !status.success() {
        return Err("Docker container failed to start. Check image and logs.".to_string());
    }

    println!("Container {} is running.", container_name);
    Ok(())
}
