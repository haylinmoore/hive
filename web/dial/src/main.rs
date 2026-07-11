use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

mod config;
mod dial;
mod panel;
use crate::config::Config;
use crate::dial::{dial_doorking_gather, dial_inbound};
use crate::panel::{get_panel_route, post_panel_route};

#[tokio::main]
async fn main() {
    let config = Arc::new(
        Config::new(std::env::var("DB_PATH").unwrap_or_else(|_| String::from("db"))).await,
    );

    let dial = Router::new()
        .route("/inbound", get(dial_inbound))
        .route("/doorking_gather", get(dial_doorking_gather))
        .with_state(config.clone());

    let panel = Router::new()
        .route("/", get(get_panel_route))
        .route("/", post(post_panel_route))
        .with_state(config);

    let dial_listener = tokio::net::TcpListener::bind(
        std::env::var("DIAL_BIND").unwrap_or_else(|_| String::from("0.0.0.0:3000")),
    )
    .await
    .unwrap();
    println!(
        "listening for dial on {}",
        dial_listener.local_addr().unwrap()
    );

    let panel_listener = tokio::net::TcpListener::bind(
        std::env::var("PANEL_BIND").unwrap_or_else(|_| String::from("0.0.0.0:3001")),
    )
    .await
    .unwrap();
    println!(
        "listening for panel on {}",
        panel_listener.local_addr().unwrap()
    );

    let _ = tokio::join!(
        axum::serve(dial_listener, dial),
        axum::serve(panel_listener, panel)
    );
}
