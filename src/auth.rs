use crate::{
    api::Api,
    credentials::{self, Credentials},
    error::{ApiError, ConnectionError},
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub enum AuthResult {
    Prompt {
        create_admin: bool,
        credentials: Credentials,
        save_password: bool,
        error: Option<ConnectionError>,
    },
    Connected {
        api: Api,
        username: String,
        warning: Option<ConnectionError>,
    },
    Failed(ConnectionError),
}

pub async fn prepare(api: Api, server: &str, logged_out: bool) -> AuthResult {
    if let Err(error) = api.health_check().await {
        return AuthResult::Failed(ConnectionError::HealthCheck(error));
    }

    match api.has_admin().await {
        Ok(false) => {
            return AuthResult::Prompt {
                create_admin: true,
                credentials: Credentials::default(),
                save_password: false,
                error: None,
            };
        }
        Err(error) => return AuthResult::Failed(ConnectionError::Check(error)),
        _ => {}
    }

    let saved = match credentials::load(server).await {
        Ok(saved) => saved,
        Err(error) => return login_prompt(None, Some(ConnectionError::ReadCredentials(error))),
    };

    if logged_out {
        return login_prompt(saved, None);
    }

    let Some(saved) = saved else {
        return login_prompt(None, None);
    };

    if let Some(jwt) = saved.jwt.as_deref() {
        let result = async {
            let session = api.with_token(jwt)?;
            let username = session.current_user().await?;
            Ok::<_, ApiError>((session, username))
        }
        .await;
        match result {
            Ok((api, username)) => {
                return AuthResult::Connected {
                    api,
                    username,
                    warning: None,
                };
            }
            Err(error @ (ApiError::Unauthorized | ApiError::InvalidToken(_))) => {
                tracing::info!(server, %error, "Saved session is no longer valid");
            }
            Err(error) => return AuthResult::Failed(ConnectionError::Session(error)),
        }
    }

    if saved.password.is_some() {
        authenticate(api, server, saved, false, true).await
    } else {
        login_prompt(Some(saved), None)
    }
}

pub async fn authenticate(
    api: Api,
    server: &str,
    mut credentials: Credentials,
    create_admin: bool,
    save_password: bool,
) -> AuthResult {
    let result = async {
        let (api, jwt) = api.authenticate(&credentials, create_admin).await?;
        let username = api.current_user().await?;
        Ok::<_, ApiError>((api, jwt, username))
    }
    .await;

    match result {
        Ok((api, jwt, username)) => {
            credentials.jwt = Some(jwt);
            if !save_password {
                credentials.password = None;
            }
            let warning = credentials
                .save(server)
                .await
                .err()
                .map(ConnectionError::SaveCredentials);

            AuthResult::Connected {
                api,
                username,
                warning,
            }
        }
        Err(error) => AuthResult::Prompt {
            create_admin,
            credentials: Credentials {
                username: credentials.username,
                ..Credentials::default()
            },
            save_password,
            error: Some(if create_admin {
                ConnectionError::Create(error)
            } else {
                ConnectionError::Login(error)
            }),
        },
    }
}

pub async fn sign_out(server: &str) -> AuthResult {
    match credentials::load(server).await {
        Ok(Some(mut saved)) => {
            saved.jwt = None;
            let error = saved.save(server).await.err().map(ConnectionError::SignOut);
            login_prompt(Some(saved), error)
        }
        Ok(None) => login_prompt(None, None),
        Err(error) => login_prompt(None, Some(ConnectionError::SignOut(error))),
    }
}

fn login_prompt(saved: Option<Credentials>, error: Option<ConnectionError>) -> AuthResult {
    let credentials = saved.unwrap_or_default();

    AuthResult::Prompt {
        create_admin: false,
        save_password: credentials.password.is_some(),
        credentials,
        error,
    }
}
