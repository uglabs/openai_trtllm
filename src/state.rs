use tonic::transport::Channel;

use crate::history::HistoryBuilder;
use crate::triton::grpc_inference_service_client::GrpcInferenceServiceClient;
use crate::triton::health_client::HealthClient;

#[derive(Clone)]
pub struct AppState {
    pub grpc_client: GrpcInferenceServiceClient<Channel>,
    pub health_client: HealthClient<Channel>,
    pub history_builder: HistoryBuilder,
    pub ignore_upstream_health: bool,
}
