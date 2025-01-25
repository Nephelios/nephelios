use std::process::Command;
use std::fs::OpenOptions;
use std::io::{self, Write, Read};
use std::path::PathBuf;
use std::fs::{File};


pub fn docker_compose() -> io::Result<()> {

  Command::new("docker")
  .current_dir("src")
  .arg("compose")
  .arg("up")
  .arg("-d")
  .spawn()
  .expect("Échec lors du lancement de docker compose up");

  Ok(())

}

pub fn verif_app(app: &str) -> io::Result<i32> {

  let path = PathBuf::from("src/docker-compose.yml");
  let mut file = File::open(&path)?;
  let mut content = String::new();
  file.read_to_string(&mut content)?;

  if content.contains(app){
    Ok(1)
  }
  else {
    Ok(0)
  }

}


pub fn add_to_hosts(app: &str) -> io::Result<()> {

  let path = PathBuf::from("/etc/hosts");

  let mut file = OpenOptions::new()
  .append(true)
  .create(true)
  .open(path)?;

  let resultat = format!(
    r#"
    127.0.0.1 {}

  "#,
  app
);

  file.write_all(resultat.as_bytes())?;

  Ok(())

}

pub fn add_to_deploy(app: &str, port: &str) -> io::Result<()> {


    let path = PathBuf::from("src/docker-compose.yml");
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;

    let service = app;
    let container_name = app;
    let image = app;
    let label = "labels";
    let app = app;
    let port= port;
    let resultat = format!(
        r#"
  {}:
    image: {}:latest
    container_name: {}
    {}:
      - "traefik.enable=true"
      - "traefik.http.routers.{}.rule=Host(`{}.local`)"
      - "traefik.http.routers.{}.entryPoints=websecure"
      - "traefik.http.routers.{}.tls=true"
      - "traefik.http.services.{}.loadbalancer.server.port={}"
    networks:
      - traefik-global-proxy
     
    
"#,
        service, 
        image,
        container_name,
        label,
        service,
        app,
        service,
        service,
        service,
        port
    );


    file.write_all(resultat.as_bytes())?;
    println!("Contenu ajouté");

    Ok(())
}
