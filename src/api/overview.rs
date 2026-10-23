use super::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/overview.graphql",
    response_derives = "Debug"
)]
struct OverviewSummary;

pub use overview_summary::ResponseData as Summary;

impl Api {
    pub async fn overview(&self) -> Result<Summary, ApiError> {
        self.execute::<OverviewSummary>(overview_summary::Variables {})
            .await
    }
}
