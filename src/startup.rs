use anyhow::Context;
use axum::routing::{get, post};
use axum::Router;
use axum_tracing_opentelemetry::middleware::OtelAxumLayer;

use crate::config::Config;
use crate::history::HistoryBuilder;
use crate::routes;
use crate::state::AppState;
use crate::triton::grpc_inference_service_client::GrpcInferenceServiceClient;
use crate::triton::health_client::HealthClient;

pub async fn run_server(config: Config) -> anyhow::Result<()> {
    tracing::info!("Connecting to triton endpoint: {}", config.triton_endpoint);
    let grpc_client = GrpcInferenceServiceClient::connect(config.triton_endpoint.clone())
        .await
        .context("failed to connect triton endpoint")?;
    let health_client = HealthClient::connect(config.triton_endpoint.clone())
        .await
        .context("failed to connect triton endpoint")?;

    let history_builder =
        HistoryBuilder::new(&config.history_template, &config.history_template_file)?;

    let ignore_upstream_health = config.ignore_upstream_health;

    let state = AppState {
        grpc_client,
        health_client,
        history_builder,
        ignore_upstream_health,
    };

    let app = Router::new()
        .route("/v1/completions", post(routes::compat_completions))
        .route(
            "/v1/chat/completions",
            post(routes::compat_chat_completions),
        )
        .route("/health_check", get(routes::health_check))
        .with_state(state)
        .layer(OtelAxumLayer::default());

    let address = format!("{}:{}", config.host, config.port);
    tracing::info!("Starting server at {}", address);

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");

    opentelemetry::global::shutdown_tracer_provider();
}
