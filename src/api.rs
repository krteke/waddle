use std::time::Duration;

use graphql_client::{GraphQLQuery, Response};
use reqwest::{Client, header::HeaderValue};
use url::Url;

use crate::{credentials::Credentials, error::ApiError};

pub mod config;
pub mod group;
pub mod node;
pub mod overview;
pub mod routing;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/health_check.graphql"
)]
struct HealthCheck;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/has_admin.graphql"
)]
struct HasAdmin;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/login.graphql"
)]
struct Login;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/create_admin.graphql"
)]
struct CreateAdmin;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/current_user.graphql"
)]
struct CurrentUser;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/proxy_status.graphql"
)]
struct ProxyStatus;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/run_proxy.graphql"
)]
struct RunProxy;

#[derive(Clone, Debug)]
pub struct Api {
    client: Client,
    endpoint: Url,
    auth: Option<HeaderValue>,
}

impl Api {
    pub fn new(mut address: Url) -> Self {
        address.set_path("/graphql");
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("HTTP client initialization failed"),
            endpoint: address,
            auth: None,
        }
    }

    pub fn endpoint(&self) -> &str {
        self.endpoint.as_str()
    }

    async fn execute<Q: GraphQLQuery>(
        &self,
        variables: Q::Variables,
    ) -> Result<Q::ResponseData, ApiError> {
        let mut request = self
            .client
            .post(self.endpoint.clone())
            .json(&Q::build_query(variables));

        if let Some(auth) = &self.auth {
            request = request.header(reqwest::header::AUTHORIZATION, auth);
        }

        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ApiError::Unauthorized);
        }
        // Failed operations may omit required data fields. Read their errors before decoding data.
        let response: Response<serde_json::Value> = response.error_for_status()?.json().await?;

        if let Some(errors) = response.errors
            && !errors.is_empty()
        {
            if errors.iter().any(|error| error.message == "access denied") {
                return Err(ApiError::Unauthorized);
            }
            return Err(ApiError::Graphql(errors));
        }

        Ok(serde_json::from_value(
            response.data.ok_or(ApiError::MissingData)?,
        )?)
    }

    pub async fn health_check(&self) -> Result<(), ApiError> {
        let status = self
            .execute::<HealthCheck>(health_check::Variables {})
            .await?
            .health_check;

        if status != 1 {
            return Err(ApiError::Unhealthy { status });
        }

        Ok(())
    }

    pub async fn has_admin(&self) -> Result<bool, ApiError> {
        Ok(self
            .execute::<HasAdmin>(has_admin::Variables {})
            .await?
            .number_users
            > 0)
    }

    pub async fn proxy_running(&self) -> Result<bool, ApiError> {
        Ok(self
            .execute::<ProxyStatus>(proxy_status::Variables {})
            .await?
            .general
            .dae
            .running)
    }

    pub async fn set_proxy_running(&self, running: bool) -> Result<(), ApiError> {
        self.execute::<RunProxy>(run_proxy::Variables { dry: !running })
            .await?;

        Ok(())
    }

    pub async fn authenticate(
        &self,
        credentials: &Credentials,
        create: bool,
    ) -> Result<(Self, String), ApiError> {
        let password = credentials
            .password
            .as_deref()
            .expect("Password authentication requires a password");

        let token = if create {
            self.execute::<CreateAdmin>(create_admin::Variables {
                username: credentials.username.clone(),
                password: password.to_owned(),
            })
            .await?
            .create_user
        } else {
            self.execute::<Login>(login::Variables {
                username: credentials.username.clone(),
                password: password.to_owned(),
            })
            .await?
            .token
        };

        Ok((self.with_token(&token)?, token))
    }

    pub fn with_token(&self, token: &str) -> Result<Self, ApiError> {
        let mut auth = HeaderValue::from_str(&format!("Bearer {token}"))?;
        auth.set_sensitive(true);

        Ok(Self {
            auth: Some(auth),
            ..self.clone()
        })
    }

    pub async fn current_user(&self) -> Result<String, ApiError> {
        Ok(self
            .execute::<CurrentUser>(current_user::Variables {})
            .await?
            .user
            .username)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Wing;

    #[test]
    #[ignore = "requires DAE_WING_BIN; run scripts/test-e2e.sh"]
    fn administrator_creation_and_login_authorize_graphql_requests() {
        let wing = Wing::start();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let api = Api::new(format!("http://{}", wing.address).parse().unwrap());
                api.health_check().await.unwrap();
                assert!(!api.has_admin().await.unwrap());
                let credentials = crate::credentials::Credentials::new("admin", "test-password-42");
                let (session, _) = api.authenticate(&credentials, true).await.unwrap();
                assert!(!session.proxy_running().await.unwrap());
                session.set_proxy_running(false).await.unwrap();
                assert!(!session.proxy_running().await.unwrap());
                assert!(matches!(
                    session.set_proxy_running(true).await,
                    Err(ApiError::Graphql(_))
                ));
                assert!(!session.proxy_running().await.unwrap());
                assert_eq!(session.current_user().await.unwrap(), "admin");
                assert!(api.has_admin().await.unwrap());
                assert!(matches!(
                    api.authenticate(
                        &crate::credentials::Credentials::new("admin", "wrong-password"),
                        false
                    )
                    .await,
                    Err(ApiError::Graphql(_))
                ));
                let (session, _) = api.authenticate(&credentials, false).await.unwrap();
                assert_eq!(session.current_user().await.unwrap(), "admin");
            });
    }
}
