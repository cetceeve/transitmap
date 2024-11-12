use axum::{
    routing::get,
    Router,
};
use tower_http::services::{ServeDir, ServeFile};

mod trip_metadata;
mod realtime;
mod stats;

use trip_metadata::{metadata_handler, init_delay_listener};
use realtime::realtime_handler;
use stats::stats_handler;
use types::init_async_metadata_table;


#[tokio::main]
async fn main() {
    init_async_metadata_table().await;
    init_delay_listener().await;
    let serve_dir = ServeDir::new("website/")
        .not_found_service(ServeFile::new("website/index.html"));

    let router = Router::new()
        .route("/realtime", get(realtime_handler))
        .route("/trip_metadata/:trip_id", get(metadata_handler))
        .route("/stats", get(stats_handler))
        .fallback_service(serve_dir);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("started server");
    axum::serve(listener, router).await.unwrap();
}
