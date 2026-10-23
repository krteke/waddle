use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    MinMovingAvg,
    MinAvg10,
    Min,
    Random,
    Fixed,
}

impl Policy {
    pub const ALL: [Self; 5] = [
        Self::MinMovingAvg,
        Self::MinAvg10,
        Self::Min,
        Self::Random,
        Self::Fixed,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MinMovingAvg => "min_moving_avg",
            Self::MinAvg10 => "min_avg10",
            Self::Min => "min",
            Self::Random => "random",
            Self::Fixed => "fixed",
        }
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/groups.graphql",
    extern_enums("Policy"),
    response_derives = "Debug, Clone, PartialEq"
)]
struct Groups;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/create_group.graphql",
    extern_enums("Policy"),
    response_derives = "Debug, Clone, PartialEq"
)]
struct CreateGroup;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/set_group_policy.graphql",
    extern_enums("Policy"),
    response_derives = "Debug, Clone, PartialEq"
)]
struct SetGroupPolicy;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/rename_group.graphql",
    response_derives = "Debug, Clone, PartialEq"
)]
struct RenameGroup;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/remove_group.graphql",
    response_derives = "Debug, Clone, PartialEq"
)]
struct RemoveGroup;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/add_group_nodes.graphql",
    response_derives = "Debug, Clone, PartialEq"
)]
struct AddGroupNodes;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/del_group_nodes.graphql",
    response_derives = "Debug, Clone, PartialEq"
)]
struct DelGroupNodes;

pub use groups::{GroupsGroups as Group, GroupsGroupsNodes as GroupNode};

impl From<node::Node> for GroupNode {
    fn from(node: node::Node) -> Self {
        Self {
            id: node.id,
            name: node.name,
            tag: node.tag,
            protocol: node.protocol,
        }
    }
}

impl Api {
    pub async fn groups(&self) -> Result<Vec<Group>, ApiError> {
        let groups = self.execute::<Groups>(groups::Variables {}).await?.groups;

        Ok(groups)
    }

    pub async fn create_group(&self, name: String, policy: Policy) -> Result<(), ApiError> {
        let params = if policy == Policy::Fixed {
            vec![create_group::PolicyParam {
                key: Some(String::new()),
                val: "0".into(),
            }]
        } else {
            Vec::new()
        };

        self.execute::<CreateGroup>(create_group::Variables {
            name,
            policy,
            params: Some(params),
        })
        .await?;

        Ok(())
    }

    pub async fn remove_group(&self, id: String) -> Result<(), ApiError> {
        self.execute::<RemoveGroup>(remove_group::Variables { id })
            .await?;

        Ok(())
    }

    pub async fn save_group(&self, original: &Group, edited: &Group) -> Result<(), ApiError> {
        let id = &original.id;

        if original.name != edited.name {
            self.execute::<RenameGroup>(rename_group::Variables {
                id: id.clone(),
                name: edited.name.clone(),
            })
            .await?;
        }

        if original.policy != edited.policy {
            let params = if edited.policy == Policy::Fixed {
                vec![set_group_policy::PolicyParam {
                    key: Some(String::new()),
                    val: "0".into(),
                }]
            } else {
                Vec::new()
            };

            self.execute::<SetGroupPolicy>(set_group_policy::Variables {
                id: id.clone(),
                policy: edited.policy,
                params: Some(params),
            })
            .await?;
        }

        let removed: Vec<_> = original
            .nodes
            .iter()
            .filter(|node| !edited.nodes.iter().any(|other| other.id == node.id))
            .map(|node| node.id.clone())
            .collect();

        if !removed.is_empty() {
            self.execute::<DelGroupNodes>(del_group_nodes::Variables {
                id: id.clone(),
                nodes: removed,
            })
            .await?;
        }

        let added: Vec<_> = edited
            .nodes
            .iter()
            .filter(|node| !original.nodes.iter().any(|other| other.id == node.id))
            .map(|node| node.id.clone())
            .collect();

        if !added.is_empty() {
            self.execute::<AddGroupNodes>(add_group_nodes::Variables {
                id: id.clone(),
                nodes: added,
            })
            .await?;
        }

        Ok(())
    }
}
