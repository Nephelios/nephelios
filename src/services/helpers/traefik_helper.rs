use crate::services::helpers::docker_helper::AppMetadata;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use regex::Regex;

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
    let path = PathBuf::from("./nephelios.yml");
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
    let path = PathBuf::from("./nephelios.yml");
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;

    let service = app;
    let image = app;
    let replicas = 1;
    let resultat = format!(
        r#"  {}:
    image: registry:5000/{}:latest
    deploy:
        mode: replicated
        replicas: {}
        resources:
            limits:
                cpus: "1.5"      # Maximum 1.5 CPU cores
                memory: 1G       # Maximum 1GB RAM
            reservations:
                cpus: "0.5"      # Reserve at least 0.5 CPU cores
                memory: 256M     # Reserve at least 256MB RAM
        labels:
          - "traefik.enable=true"
          - "traefik.http.routers.{}.rule=Host(`{}.localhost`)"
          - "traefik.http.routers.{}.entrypoints=web,websecure"
          - "traefik.http.routers.{}.tls.certresolver=myresolver"
          - "traefik.http.services.{}.loadbalancer.server.port={}"
          - "com.myapp.name={}"
          - "com.myapp.image={}:latest"
          - "com.myapp.type={}"
          - "com.myapp.github_url={}"
          - "com.myapp.domain={}"
          - "com.myapp.created_at={}"
    networks:
        - nephelios_overlay

"#,
        service, image, replicas, service, app, service, service, service, port, app, image, metadata.app_type, metadata.github_url, metadata.domain, metadata.created_at
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
    let path = PathBuf::from("./nephelios.yml");
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
        if !line.starts_with("  ") {
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

/// Updates the number of replicas for an application in the nephelios.yml file.
///
/// # Arguments
///
/// * `app_name` - The name of the application to update.
/// * `replicas` - The new number of replicas.
///
/// # Returns
///
/// A `Result` indicating success or an I/O error.
pub fn update_app_replicas(app_name: &str, replicas: u32) -> io::Result<()> {
    let path = PathBuf::from("./nephelios.yml");
    
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The file nephelios.yml does not exist"
        ));
    }
    
    let content = fs::read_to_string(&path)?;    
    if !content.contains(&format!("{}:", app_name)) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Application {} not found in the file nephelios.yml", app_name)        ));
    }
    
    let pattern = format!(r"(?m)^(\s*{}:\s*(?:\r?\n.*?)*?\breplicas:\s*)(\d+)", regex::escape(app_name));    
    let re = Regex::new(&pattern).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("Error while creating the regex: {}", e))    })?;
    
    if re.is_match(&content) {
        let new_content = re.replace_all(&content, |caps: &regex::Captures| {
            format!("{}{}", &caps[1], replicas)
        });
        
        fs::write(&path, new_content.as_bytes())?;
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Pattern 'replicas:' not found for the application {}", app_name)        ))
    }
}
