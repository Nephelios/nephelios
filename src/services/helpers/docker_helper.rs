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
    let dockerfile_path = Path::new(app_path).join("Dockerfile");

    // Check if the Dockerfile already exists
    if dockerfile_path.exists() {
        println!("Dockerfile already exists at {}", dockerfile_path.display());
        return Ok(());
    }

    let dockerfile_content = match app_type {
        "nodejs" => {
            r#"
            FROM node:18
            WORKDIR /app
            # Copy package.json and package-lock.json (if available) for dependency caching
            COPY package*.json ./
            RUN npm install --production
            # Copy the rest of the application code
            COPY . .
            # Expose a default port for documentation purposes
            EXPOSE 3000
            # Use npm start as the default entry point
            CMD ["npm", "start"]
        "#
        }
        "python" => {
            r#"
            FROM python:3.8-slim
            WORKDIR /app
            # Copy the requirements file for dependency installation
            COPY requirements.txt ./
            RUN pip install --no-cache-dir -r requirements.txt
            # Copy the rest of the application code
            COPY . .
            # Expose a default port for documentation purposes
            EXPOSE 5000
            # Use a generic command to start the app
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

/// Runs the Docker Compose command to deploy the application.
///
/// # Returns
/// * `Ok(())` if the Docker Compose command was successful.
/// * `Err(String)` if there was an error during execution.
pub fn docker_compose() -> Result<(), String> {
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
