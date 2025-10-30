use serde::{Deserialize, Serialize};
use std::time::Duration;
#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub client_region: String,
    pub response_region: String,
    pub ip_address: String,
    pub dns_duration: Duration,
    pub dns_duration2: Duration,
    pub tcp_duration: Duration,
    pub tls_duration: Duration,
    pub get_duration: Duration,
    pub total_duration: Duration,
}
