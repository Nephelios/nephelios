mod routes;
mod services;

use crate::routes::{create_app_route, health_check_route};
use warp::Filter;
use std::thread;
mod traefik;

/// Entry point for the application.
#[tokio::main]
async fn main() {



    let api_routes = create_app_route().or(health_check_route());
    // warp::serve(api_routes).run(([127, 0, 0, 1], 3000)).await;

    let warp_thread = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            println!("Serveur Warp en cours d'exécution sur http://127.0.0.1:3000");
            warp::serve(api_routes)
                .run(([127, 0, 0, 1], 3000))
                .await;
        });
    });

  
    warp_thread.join().unwrap();

}
