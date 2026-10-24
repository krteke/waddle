use super::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/routings.graphql",
    response_derives = "Debug, Clone"
)]
struct Routings;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/create_routing.graphql"
)]
struct CreateRouting;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/update_routing.graphql"
)]
struct UpdateRouting;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/select_routing.graphql"
)]
struct SelectRouting;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/rename_routing.graphql"
)]
struct RenameRouting;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/remove_routing.graphql"
)]
struct RemoveRouting;

pub use routings::{ResponseData as RoutingCatalog, RoutingsRoutings as Routing};
impl Api {
    pub async fn routings(&self) -> Result<RoutingCatalog, ApiError> {
        self.execute::<Routings>(routings::Variables {}).await
    }

    pub async fn create_routing(&self, name: String, routing: String) -> Result<(), ApiError> {
        self.execute::<CreateRouting>(create_routing::Variables { name, routing })
            .await?;

        Ok(())
    }

    pub async fn update_routing(&self, id: String, routing: String) -> Result<(), ApiError> {
        self.execute::<UpdateRouting>(update_routing::Variables { id, routing })
            .await?;

        Ok(())
    }

    pub async fn rename_routing(&self, id: String, name: String) -> Result<(), ApiError> {
        self.execute::<RenameRouting>(rename_routing::Variables { id, name })
            .await?;

        Ok(())
    }

    pub async fn select_routing(&self, id: String) -> Result<(), ApiError> {
        self.execute::<SelectRouting>(select_routing::Variables { id })
            .await?;

        Ok(())
    }

    pub async fn remove_routing(&self, id: String) -> Result<(), ApiError> {
        self.execute::<RemoveRouting>(remove_routing::Variables { id })
            .await?;

        Ok(())
    }
}
