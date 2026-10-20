use serde::{Deserialize, Serialize};

use crate::{error::CredentialError, settings::APPLICATION_ID};

#[derive(Default, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: Option<String>,
    pub jwt: Option<String>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Credentials")
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

impl Credentials {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: Some(password.into()),
            jwt: None,
        }
    }

    pub async fn save(&self, server: &str) -> Result<(), CredentialError> {
        let keyring = oo7::Keyring::new().await?;

        keyring.unlock().await?;
        keyring
            .create_item(
                &format!("Waddle — {server}"),
                &[("application", APPLICATION_ID), ("server", server)],
                serde_json::to_vec(self)?,
                true,
            )
            .await?;

        Ok(())
    }
}

pub async fn load(server: &str) -> Result<Option<Credentials>, CredentialError> {
    let keyring = oo7::Keyring::new().await?;
    keyring.unlock().await?;

    let items = keyring
        .search_items(&[("application", APPLICATION_ID), ("server", server)])
        .await?;

    match items.first() {
        Some(item) => {
            item.unlock().await?;
            let secret = item.secret().await?;
            Ok(Some(serde_json::from_slice(secret.as_bytes())?))
        }
        None => Ok(None),
    }
}

pub async fn delete(server: &str) -> Result<(), CredentialError> {
    let keyring = oo7::Keyring::new().await?;
    keyring.unlock().await?;
    keyring
        .delete(&[("application", APPLICATION_ID), ("server", server)])
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an isolated Secret Service; run scripts/test-e2e.sh"]
    fn credentials_are_replaced_and_looked_up_by_server_name() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let first = format!("waddle-test-first-{}", std::process::id());
            let second = format!("waddle-test-second-{}", std::process::id());
            assert!(load(&first).await.unwrap().is_none());
            let alice = Credentials {
                username: "alice".into(),
                password: Some("first-secret".into()),
                jwt: Some("first-token".into()),
            };
            alice.save(&first).await.unwrap();
            assert!(load(&second).await.unwrap().is_none());
            let bob = Credentials {
                username: "bob".into(),
                password: None,
                jwt: Some("replacement-token".into()),
            };
            bob.save(&first).await.unwrap();
            let credentials = load(&first).await.unwrap().unwrap();
            assert_eq!(credentials.username, "bob");
            assert!(credentials.password.is_none());
            assert_eq!(credentials.jwt.as_deref(), Some("replacement-token"));
            assert!(!format!("{credentials:?}").contains("replacement-secret"));
            assert!(!format!("{credentials:?}").contains("replacement-token"));
            let mut credentials = credentials;
            credentials.jwt = None;
            credentials.save(&first).await.unwrap();
            assert!(
                load(&first)
                    .await
                    .unwrap()
                    .unwrap()
                    .jwt
                    .as_deref()
                    .is_none()
            );
            let keyring = oo7::Keyring::new().await.unwrap();
            let attributes = [
                ("application", crate::settings::APPLICATION_ID),
                ("server", first.as_str()),
            ];
            assert_eq!(keyring.search_items(&attributes).await.unwrap().len(), 1);
            keyring.delete(&attributes).await.unwrap();
        });
    }
}
