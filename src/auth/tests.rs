use super::*;
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    task::JoinHandle,
};

struct HealthServer {
    api: Api,
    requests: Arc<Mutex<Vec<String>>>,
    task: JoinHandle<()>,
}

impl HealthServer {
    async fn start(status: &str, health_response: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let api = Api::new(listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = requests.clone();
        let status = status.to_owned();
        let health_response = health_response.to_owned();
        let task = tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let mut stream = BufReader::new(stream);
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    stream.read_line(&mut header).await.unwrap();
                    if header == "\r\n" {
                        break;
                    }
                    if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:")
                    {
                        length = value.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                stream.read_exact(&mut body).await.unwrap();
                let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
                let operation = request["operationName"].as_str().unwrap();
                received.lock().unwrap().push(operation.to_owned());
                let (status, body) = if operation == "HealthCheck" {
                    (status.as_str(), health_response.as_str())
                } else {
                    ("200 OK", r#"{"data":{"numberUsers":0}}"#)
                };
                stream
                    .write_all(
                        format!(
                            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
        });
        Self {
            api,
            requests,
            task,
        }
    }
}

impl Drop for HealthServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[test]
fn unhealthy_server_stops_before_administrator_check() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let server = HealthServer::start("200 OK", r#"{"data":{"healthCheck":0}}"#).await;
        let result = prepare(server.api.clone(), "Unhealthy", false).await;
        assert!(matches!(result, AuthResult::Failed(_)), "{result:?}");
        assert_eq!(*server.requests.lock().unwrap(), ["HealthCheck"]);
    });
}

#[test]
fn health_graphql_error_stops_before_administrator_check() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let server =
            HealthServer::start("200 OK", r#"{"errors":[{"message":"health unavailable"}]}"#).await;
        let result = prepare(server.api.clone(), "GraphQL failure", false).await;
        assert!(matches!(result, AuthResult::Failed(_)), "{result:?}");
        assert_eq!(*server.requests.lock().unwrap(), ["HealthCheck"]);
    });
}

#[test]
fn health_http_error_stops_before_administrator_check() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let server = HealthServer::start("503 Service Unavailable", "").await;
        let result = prepare(server.api.clone(), "HTTP failure", false).await;
        assert!(matches!(result, AuthResult::Failed(_)), "{result:?}");
        assert_eq!(*server.requests.lock().unwrap(), ["HealthCheck"]);
    });
}

#[test]
fn healthy_server_checks_administrator_after_health_check() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let server = HealthServer::start("200 OK", r#"{"data":{"healthCheck":1}}"#).await;
        let result = prepare(server.api.clone(), "Healthy", false).await;
        assert!(
            matches!(
                result,
                AuthResult::Prompt {
                    create_admin: true,
                    error: None,
                    ..
                }
            ),
            "{result:?}"
        );
        assert_eq!(
            *server.requests.lock().unwrap(),
            ["HealthCheck", "HasAdmin"]
        );
    });
}

async fn account() -> (crate::test_support::Wing, Api, Credentials, String) {
    let wing = crate::test_support::Wing::start();
    let api = Api::new(wing.address);
    let credentials = Credentials::new("admin", "test-password-42");
    let (_, jwt) = api.authenticate(&credentials, true).await.unwrap();
    (wing, api, credentials, jwt)
}

#[test]
#[ignore = "requires dae-wing and isolated Secret Service; run scripts/test-e2e.sh"]
fn saved_jwt_takes_precedence_over_a_wrong_saved_password() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let (_wing, api, _, jwt) = account().await;
        let wrong = Credentials::new("admin", "wrong-password");
        Credentials {
            username: wrong.username.clone(),
            password: wrong.password.clone(),
            jwt: Some(jwt),
        }
        .save("jwt-first")
        .await
        .unwrap();
        let result = prepare(api, "jwt-first", false).await;
        assert!(
            matches!(result, AuthResult::Connected { warning: None, .. }),
            "{result:?}"
        );
        assert_eq!(
            credentials::load("jwt-first")
                .await
                .unwrap()
                .unwrap()
                .password
                .as_deref(),
            Some("wrong-password")
        );
    });
}

#[test]
#[ignore = "requires dae-wing and isolated Secret Service; run scripts/test-e2e.sh"]
fn saved_jwt_restores_session_without_a_saved_password() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let (_wing, api, credentials, jwt) = account().await;
        Credentials {
            username: credentials.username.clone(),
            password: None,
            jwt: Some(jwt),
        }
        .save("jwt-only")
        .await
        .unwrap();
        let result = prepare(api, "jwt-only", false).await;
        let AuthResult::Connected {
            api, warning: None, ..
        } = result
        else {
            panic!("{result:?}");
        };
        assert_eq!(api.current_user().await.unwrap(), "admin");
        assert!(
            credentials::load("jwt-only")
                .await
                .unwrap()
                .unwrap()
                .password
                .as_deref()
                .is_none()
        );
    });
}

#[test]
#[ignore = "requires dae-wing and isolated Secret Service; run scripts/test-e2e.sh"]
fn rejected_jwt_automatically_renews_using_saved_password() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let (_wing, api, credentials, _) = account().await;
        Credentials {
            username: credentials.username.clone(),
            password: credentials.password.clone(),
            jwt: Some("rejected-token".into()),
        }
        .save("renew-jwt")
        .await
        .unwrap();
        let result = prepare(api.clone(), "renew-jwt", false).await;
        assert!(
            matches!(result, AuthResult::Connected { warning: None, .. }),
            "{result:?}"
        );
        let saved = credentials::load("renew-jwt").await.unwrap().unwrap();
        assert_eq!(
            api.with_token(saved.jwt.as_deref().unwrap())
                .unwrap()
                .current_user()
                .await
                .unwrap(),
            "admin"
        );
        assert_eq!(saved.password.as_deref(), Some("test-password-42"));
    });
}

#[test]
#[ignore = "requires dae-wing and isolated Secret Service; run scripts/test-e2e.sh"]
fn rejected_jwt_without_saved_password_prompts_for_credentials() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let (_wing, api, credentials, _) = account().await;
        Credentials {
            username: credentials.username.clone(),
            password: None,
            jwt: Some("rejected-token".into()),
        }
        .save("prompt-jwt")
        .await
        .unwrap();
        let result = prepare(api, "prompt-jwt", false).await;
        let AuthResult::Prompt {
            credentials,
            save_password: false,
            create_admin: false,
            error: None,
        } = result
        else {
            panic!("{result:?}");
        };
        assert_eq!(credentials.username, "admin");
        assert_eq!(credentials.password.as_deref(), None);
    });
}

#[test]
#[ignore = "requires dae-wing and isolated Secret Service; run scripts/test-e2e.sh"]
fn manual_sign_out_clears_only_the_target_jwt_and_prefills_saved_password() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let (_wing, api, credentials, jwt) = account().await;
        Credentials {
            username: credentials.username.clone(),
            password: credentials.password.clone(),
            jwt: Some(jwt.clone()),
        }
        .save("sign-out")
        .await
        .unwrap();
        Credentials {
            username: credentials.username.clone(),
            password: credentials.password.clone(),
            jwt: Some(jwt.clone()),
        }
        .save("other-server")
        .await
        .unwrap();
        let result = sign_out("sign-out").await;
        let AuthResult::Prompt {
            credentials,
            save_password: true,
            create_admin: false,
            error: None,
        } = result
        else {
            panic!("{result:?}");
        };
        assert_eq!(credentials.password.as_deref(), Some("test-password-42"));
        assert!(
            credentials::load("sign-out")
                .await
                .unwrap()
                .unwrap()
                .jwt
                .as_deref()
                .is_none()
        );
        assert_eq!(
            credentials::load("other-server")
                .await
                .unwrap()
                .unwrap()
                .jwt
                .as_deref(),
            Some(jwt.as_str())
        );
        let result = prepare(api, "sign-out", true).await;
        assert!(
            matches!(result, AuthResult::Prompt { error: None, .. }),
            "{result:?}"
        );
    });
}

#[test]
#[ignore = "requires dae-wing and isolated Secret Service; run scripts/test-e2e.sh"]
fn manual_sign_out_flag_prevents_using_a_remaining_valid_jwt() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let (_wing, api, credentials, jwt) = account().await;
        Credentials {
            username: credentials.username.clone(),
            password: credentials.password.clone(),
            jwt: Some(jwt),
        }
        .save("manual-only")
        .await
        .unwrap();
        let result = prepare(api, "manual-only", true).await;
        let AuthResult::Prompt {
            credentials,
            error: None,
            ..
        } = result
        else {
            panic!("{result:?}");
        };
        assert_eq!(credentials.username, "admin");
        assert_eq!(credentials.password.as_deref(), Some("test-password-42"));
    });
}
