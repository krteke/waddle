use super::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/nodes.graphql",
    response_derives = "Debug, Clone"
)]
struct Nodes;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/import_nodes.graphql",
    response_derives = "Debug"
)]
struct ImportNodes;

pub use import_nodes::{ImportArgument, ImportNodesImportNodes as NodeImportResult};
pub use nodes::NodesNodesEdges as Node;

impl Api {
    pub async fn nodes(&self) -> Result<Vec<Node>, ApiError> {
        Ok(self
            .execute::<Nodes>(nodes::Variables {})
            .await?
            .nodes
            .edges)
    }

    pub async fn import_nodes(
        &self,
        args: Vec<ImportArgument>,
    ) -> Result<Vec<NodeImportResult>, ApiError> {
        Ok(self
            .execute::<ImportNodes>(import_nodes::Variables { args })
            .await?
            .import_nodes)
    }
}
