use super::*;
use relm4::gtk::glib;
use serde::{Deserialize, Deserializer, de::Error};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/traffic_overview.graphql",
    response_type = "crate::api::traffic::GeneralTraffic"
)]
struct TrafficOverview;

type Time = String;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralTraffic {
    runtime_overview: TrafficStats,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficStats {
    #[serde(deserialize_with = "timestamp")]
    pub updated_at: f64,
    pub upload_rate: f64,
    pub download_rate: f64,
    #[serde(deserialize_with = "byte_total")]
    pub upload_total: u64,
    #[serde(deserialize_with = "byte_total")]
    pub download_total: u64,
    pub active_connections: i64,
    pub udp_sessions: i64,
    pub samples: Vec<TrafficSample>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficSample {
    #[serde(deserialize_with = "timestamp")]
    pub timestamp: f64,
    pub upload_rate: f64,
    pub download_rate: f64,
}

// Normalize wire timestamps to Unix seconds, preserving subsecond chart spacing.
fn timestamp<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let text = String::deserialize(deserializer)?;
    let time = glib::DateTime::from_iso8601(&text, None).map_err(D::Error::custom)?;
    Ok(time.to_unix_usec() as f64 / 1_000_000.0)
}

fn byte_total<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    String::deserialize(deserializer)?
        .parse()
        .map_err(D::Error::custom)
}

impl Api {
    pub async fn traffic(
        &self,
        window_sec: i64,
        max_points: i64,
    ) -> Result<TrafficStats, ApiError> {
        Ok(self
            .execute::<TrafficOverview>(traffic_overview::Variables {
                window_sec,
                max_points,
            })
            .await?
            .general
            .runtime_overview)
    }
}
