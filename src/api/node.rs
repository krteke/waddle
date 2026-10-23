use super::*;

pub use import_nodes::{ImportArgument, ImportNodesImportNodes as NodeImportResult};
pub use nodes::NodesNodesEdges as Node;
pub use test_node_latencies::TestNodeLatenciesTestNodeLatencies as NodeLatency;

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

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/test_node_latencies.graphql",
    response_derives = "Debug"
)]
struct TestNodeLatencies;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/remove_nodes.graphql",
    response_derives = "Debug"
)]
struct RemoveNodes;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/update_node.graphql",
    response_derives = "Debug"
)]
struct UpdateNode;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/tag_node.graphql",
    response_derives = "Debug"
)]
struct TagNode;

impl Api {
    pub async fn update_node(&self, id: String, new_link: String) -> Result<(), ApiError> {
        self.execute::<UpdateNode>(update_node::Variables { id, new_link })
            .await?;

        Ok(())
    }

    pub async fn tag_node(&self, id: String, tag: String) -> Result<(), ApiError> {
        self.execute::<TagNode>(tag_node::Variables { id, tag })
            .await?;

        Ok(())
    }

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

    pub async fn test_node_latencies(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<NodeLatency>, ApiError> {
        Ok(self
            .execute::<TestNodeLatencies>(test_node_latencies::Variables { ids: Some(ids) })
            .await?
            .test_node_latencies)
    }

    pub async fn remove_nodes(&self, ids: Vec<String>) -> Result<(), ApiError> {
        self.execute::<RemoveNodes>(remove_nodes::Variables { ids })
            .await?;
        Ok(())
    }
}
