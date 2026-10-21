use super::*;

type Duration = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/configs.graphql",
    response_derives = "Debug"
)]
struct Configs;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/select_config.graphql"
)]
struct SelectConfig;

pub use configs::ConfigsConfigs as Config;

impl Api {
    pub async fn configs(&self) -> Result<Vec<Config>, ApiError> {
        Ok(self
            .execute::<Configs>(configs::Variables {})
            .await?
            .configs)
    }

    pub async fn select_config(&self, id: String) -> Result<(), ApiError> {
        self.execute::<SelectConfig>(select_config::Variables { id })
            .await?;

        Ok(())
    }
}
