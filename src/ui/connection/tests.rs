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
        let result = prepare(server.api.clone(), "Unhealthy").await;
        assert!(matches!(result, AuthResult::Failed(_)), "{result:?}");
        assert_eq!(*server.requests.lock().unwrap(), ["HealthCheck"]);
    });
}

#[test]
fn health_graphql_error_stops_before_administrator_check() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let server =
            HealthServer::start("200 OK", r#"{"errors":[{"message":"health unavailable"}]}"#).await;
        let result = prepare(server.api.clone(), "GraphQL failure").await;
        assert!(matches!(result, AuthResult::Failed(_)), "{result:?}");
        assert_eq!(*server.requests.lock().unwrap(), ["HealthCheck"]);
    });
}

#[test]
fn health_http_error_stops_before_administrator_check() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let server = HealthServer::start("503 Service Unavailable", "").await;
        let result = prepare(server.api.clone(), "HTTP failure").await;
        assert!(matches!(result, AuthResult::Failed(_)), "{result:?}");
        assert_eq!(*server.requests.lock().unwrap(), ["HealthCheck"]);
    });
}

#[test]
fn healthy_server_checks_administrator_after_health_check() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let server = HealthServer::start("200 OK", r#"{"data":{"healthCheck":1}}"#).await;
        let result = prepare(server.api.clone(), "Healthy").await;
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
