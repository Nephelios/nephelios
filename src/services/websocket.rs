use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use warp::ws::{Message, WebSocket};
use warp::Filter;

use crate::services::helpers::docker_helper::AppMetadata;
use futures_util::SinkExt;

#[derive(Clone, Serialize)]
pub struct DeploymentStatus {
    app_name: String,
    status: String,
    step: String,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    timestamp: DateTime<Utc>,
    metadata: AppMetadata
}

pub type StatusSender = broadcast::Sender<DeploymentStatus>;

/// Handles individual WebSocket connections.
///
/// Splits the WebSocket connection into sender and receiver parts, sets up message
/// forwarding, and maintains the connection until the client disconnects.
///
/// # Arguments
///
/// * `ws` - WebSocket connection
/// * `status_rx` - Receiver for deployment status updates
pub async fn handle_ws_connection(ws: WebSocket, status_rx: broadcast::Receiver<DeploymentStatus>) {
    let (mut ws_sender, mut ws_receiver) = ws.split();
    let (tx, mut rx) = mpsc::channel(32);
    let mut status_rx = status_rx;

    // Forward deployment status updates to WebSocket
    tokio::task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(msg).await {
                eprintln!("WebSocket send error: {}", e);
                break;
            }
        }
    });

    // Handle incoming WebSocket messages and broadcast status updates
    tokio::task::spawn(async move {
        while let Ok(status) = status_rx.recv().await {
            let msg = serde_json::to_string(&status).unwrap();
            if let Err(e) = tx.send(Message::text(msg)).await {
                eprintln!("Failed to forward status update: {}", e);
                break;
            }
        }
    });

    // Keep connection alive until client disconnects
    while let Some(result) = ws_receiver.next().await {
        if let Err(e) = result {
            eprintln!("WebSocket error: {}", e);
            break;
        }
    }
}

/// Creates a WebSocket route for handling real-time deployment status updates.
///
/// # Arguments
///
/// * `status_rx` - Receiver for deployment status updates
///
/// # Returns
///
/// A Filter that handles WebSocket upgrade requests and manages connections
pub fn ws_route(
    status_rx: broadcast::Receiver<DeploymentStatus>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let status_rx = Arc::new(status_rx);

    warp::path("ws")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let status_rx = Arc::clone(&status_rx);
            ws.on_upgrade(move |socket| handle_ws_connection(socket, status_rx.resubscribe()))
        })
}

/// Sends a deployment status update through the broadcast channel.
///
/// # Arguments
///
/// * `sender` - Broadcast channel sender
/// * `app_name` - Name of the application being deployed
/// * `status` - Current deployment status
/// * `step` - Current deployment step
///
/// # Errors
///
/// Prints error message to stderr if sending fails
pub async fn send_deployment_status(
    sender: &StatusSender,
    app_name: &str,
    status: &str,
    step: &str,
    metadata: &AppMetadata
) {
    let status_update = DeploymentStatus {
        app_name: app_name.to_string(),
        status: status.to_string(),
        step: step.to_string(),
        timestamp: chrono::Utc::now(),
        metadata: metadata.clone()
    };

    if let Err(e) = sender.send(status_update) {
        eprintln!("Failed to send status update: {}", e);
    }
}
