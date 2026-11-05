use super::*;
use serde::Deserialize;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/dnss.graphql",
    response_type = "crate::api::dns::Dns"
)]
struct Dnss;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/parse_dns.graphql",
    response_type = "crate::api::dns::DnsDocument"
)]
struct ParseDns;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/create_dns.graphql"
)]
struct CreateDns;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/update_dns.graphql"
)]
struct UpdateDns;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/rename_dns.graphql"
)]
struct RenameDns;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/select_dns.graphql"
)]
struct SelectDns;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/remove_dns.graphql"
)]
struct RemoveDns;

#[derive(Clone, Debug, Deserialize)]
pub struct Dns {
    pub id: String,
    pub name: String,
    pub selected: bool,
    pub dns: DnsDocument,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DnsDocument {
    #[serde(default)] // parsedDns has no raw text; only stored configurations expose it.
    pub string: String,
    pub upstream: Vec<Param>,
    pub routing: DnsRouting,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Param {
    pub key: String,
    pub val: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct DnsRouting {
    pub request: RuleSet,
    pub response: RuleSet,
}
#[derive(Clone, Debug, Deserialize)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
    pub fallback: Fallback,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Rule {
    pub conditions: Conditions,
    pub outbound: Function,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Conditions {
    pub and: Vec<Function>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Function {
    pub name: String,
    pub not: bool,
    pub params: Vec<Param>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "__typename")]
pub enum Fallback {
    Plaintext { val: String },
    Function(Function),
}

impl Api {
    pub async fn dnss(&self) -> Result<Vec<Dns>, ApiError> {
        let response = self.execute::<Dnss>(dnss::Variables {}).await?;
        Ok(response.dnss)
    }
    pub async fn parse_dns(&self, raw: String) -> Result<DnsDocument, ApiError> {
        let response = self
            .execute::<ParseDns>(parse_dns::Variables { raw })
            .await?;
        Ok(response.parsed_dns)
    }
    pub async fn create_dns(&self, name: String, dns: String) -> Result<(), ApiError> {
        self.execute::<CreateDns>(create_dns::Variables { name, dns })
            .await?;
        Ok(())
    }
    pub async fn update_dns(&self, id: String, dns: String) -> Result<(), ApiError> {
        self.execute::<UpdateDns>(update_dns::Variables { id, dns })
            .await?;
        Ok(())
    }
    pub async fn rename_dns(&self, id: String, name: String) -> Result<(), ApiError> {
        self.execute::<RenameDns>(rename_dns::Variables { id, name })
            .await?;
        Ok(())
    }
    pub async fn select_dns(&self, id: String) -> Result<(), ApiError> {
        self.execute::<SelectDns>(select_dns::Variables { id })
            .await?;
        Ok(())
    }
    pub async fn remove_dns(&self, id: String) -> Result<(), ApiError> {
        self.execute::<RemoveDns>(remove_dns::Variables { id })
            .await?;
        Ok(())
    }
}
