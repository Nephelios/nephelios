use crate::services::helpers::docker_helper::AppMetadata;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::PathBuf;

/// Verifies if the application is already deployed.
///
/// # Arguments
///
/// * `app_name` - The name of the application to verify.
///
/// # Returns
/// * `Ok(1)` if the application is already deployed.
/// * `Ok(0)` if the application is not deployed.
/// * `Err(String)` if there was an error during verification.
pub fn verif_app(app: &str) -> io::Result<i32> {
    let path = PathBuf::from("src/docker-compose.yml");
    let mut file = File::open(&path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    if content.contains(app) {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Adds the application to the Traefik configuration.
///
/// # Arguments
///
/// * `app_name` - The name of the application to be added.
///
/// # Returns
/// * `Ok(())` if the application was successfully added.
/// * `Err(String)` if there was an error during the addition.
pub fn add_to_deploy(app: &str, port: &str, metadata: &AppMetadata) -> io::Result<()> {
    let path = PathBuf::from("src/docker-compose.yml");
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;

    let service = app;
    let image = app;
    let replicas = 1;
    let app = app;
    let port = port;
    let resultat = format!(
        r#"  {}:
    image: localhost:5000/{}:latest
    deploy:
        replicas: {}
        labels:
          - "traefik.enable=true"
          - "traefik.http.routers.{}.rule=Host(`{}.localhost`)"
          - "traefik.http.routers.{}.entryPoints=web"
          - "traefik.http.services.{}.loadbalancer.server.port={}"
          - "com.myapp.name={}"
          - "com.myapp.image={}:latest"
          - "com.myapp.type={}"
          - "com.myapp.github_url={}"
          - "com.myapp.domain={}"
          - "com.myapp.created_at={}"
"#,
        service, image, replicas, service, app, service, service, port, app, image, metadata.app_type, metadata.github_url, metadata.domain, metadata.created_at
    );

    file.write_all(resultat.as_bytes())?;
    println!("Contenu ajouté");

    Ok(())
}


/// Removes the docker-compose configuration for the given application.
///
/// Reads the `docker-compose.yml` file, removes the section corresponding to `app_name`,
/// and writes the updated content back to the file.
///
/// # Arguments
///
/// * `app_name` - The name of the application to remove from the compose file.
///
/// # Returns
///
/// A `Result` indicating success or an I/O error.

pub fn remove_app_compose(app_name: &str) -> io::Result<()> {
    let path = PathBuf::from("src/docker-compose.yml");
    let content = fs::read_to_string(&path)?;


    let mut new_content = String::new();
    let mut in_service = false;

    for line in content.lines() {
        if line.starts_with("  ") && in_service {
            continue;
        }
        if line.starts_with(&format!("  {}:", app_name)) {
            in_service = true;
            continue;
        }
        if line.starts_with("  ") == false {
            in_service = false;
        }
        if !in_service {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }
    
    let mut file = fs::File::create(&path)?;
    file.write_all(new_content.as_bytes())?;
    
    Ok(())

}