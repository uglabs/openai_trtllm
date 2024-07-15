use anyhow::Context;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use tonic::transport::Channel;

use crate::error::AppError;
use crate::state::AppState;
use crate::triton::health_check_response::ServingStatus;
use crate::triton::health_client::HealthClient;
use crate::triton::telemetry::propagate_context;
use crate::triton::HealthCheckRequest;

fn build_health_check_request() -> HealthCheckRequest {
    HealthCheckRequest {
        // TODO: Does this have any effect on the health check?
        service: "".to_string(),
    }
}

async fn upstream_health_check(
    headers: HeaderMap,
    mut health_client: HealthClient<Channel>,
) -> Result<(), AppError> {
    let request = build_health_check_request();
    let mut request = tonic::Request::new(request);

    propagate_context(&mut request, &headers);
    let response = health_client
        .check(request)
        .await
        .context("failed to call grpc health check on upstream")?
        .into_inner();

    let upstream_status = ServingStatus::try_from(response.status)
        .context(format!("invalid upstream status: {}", response.status))?;

    match upstream_status {
        ServingStatus::Serving => Ok(()),
        _ => Err(anyhow::anyhow!(
            "unhealthy upstream. health status: {}[{:?}]",
            upstream_status.as_str_name(),
            upstream_status
        )
        .into()),
    }
}

fn no_op_health_check() -> Result<(), AppError> {
    Ok(())
}

pub async fn health_check(
    headers: HeaderMap,
    State(AppState {
        health_client,
        ignore_upstream_health,
        ..
    }): State<AppState>,
) -> Response {
    tracing::info!("health_check");
    match ignore_upstream_health {
        false => upstream_health_check(headers, health_client)
            .await
            .into_response(),
        true => no_op_health_check().into_response(),
    }
}
