use super::uri::Uri;
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq, Default)]
pub enum TcpHeader {
    #[default]
    None,
    Http {
        host: String,
        path: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Transport {
    Tcp { header: TcpHeader },
    WebSocket { host: String, path: String },
    H2 { host: String, path: String },
    Grpc { service_name: String },
    HttpUpgrade { host: String, path: String },
}

impl Default for Transport {
    fn default() -> Self {
        Self::Tcp {
            header: TcpHeader::None,
        }
    }
}

impl Transport {
    pub fn from_uri(uri: &Uri) -> Result<Self, LinkError> {
        Self::from_parts(
            &uri.get("type"),
            &uri.get("headerType"),
            uri.get("host"),
            if uri.get("type") == "grpc" {
                uri.get("serviceName")
            } else {
                uri.get("path")
            },
        )
    }

    pub fn from_parts(
        network: &str,
        header: &str,
        host: String,
        path: String,
    ) -> Result<Self, LinkError> {
        let transport = match network {
            "" | "tcp" => {
                let header = match header {
                    "" | "none" => TcpHeader::None,
                    "http" => TcpHeader::Http { host, path },
                    _ => return Err(LinkError::Unsupported),
                };

                Self::Tcp { header }
            }
            "ws" => Self::WebSocket { host, path },
            "h2" | "http" | "http2" => Self::H2 { host, path },
            "grpc" => Self::Grpc { service_name: path },
            "httpupgrade" => Self::HttpUpgrade { host, path },
            _ => return Err(LinkError::Unsupported),
        };

        Ok(transport)
    }

    pub fn query(&self) -> Vec<(&str, &str)> {
        match self {
            Self::Tcp {
                header: TcpHeader::None,
            } => vec![("type", "tcp"), ("headerType", "none")],
            Self::Tcp {
                header: TcpHeader::Http { host, path },
            } => vec![
                ("type", "tcp"),
                ("headerType", "http"),
                ("host", host),
                ("path", path),
            ],
            Self::WebSocket { host, path } => vec![("type", "ws"), ("host", host), ("path", path)],
            Self::H2 { host, path } => vec![("type", "h2"), ("host", host), ("path", path)],
            Self::Grpc { service_name } => vec![("type", "grpc"), ("serviceName", service_name)],
            Self::HttpUpgrade { host, path } => {
                vec![("type", "httpupgrade"), ("host", host), ("path", path)]
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum TrojanTransport {
    #[default]
    Original,
    WebSocket {
        host: String,
        path: String,
    },
    Grpc {
        service_name: String,
    },
    HttpUpgrade {
        host: String,
        path: String,
    },
}

impl TrojanTransport {
    pub fn from_uri(uri: &Uri) -> Result<Self, LinkError> {
        let trojan = match uri.get("type").as_str() {
            "" | "original" => Self::Original,
            "ws" => Self::WebSocket {
                host: uri.get("host"),
                path: uri.get("path"),
            },
            "httpupgrade" => Self::HttpUpgrade {
                host: uri.get("host"),
                path: uri.get("path"),
            },
            "grpc" => Self::Grpc {
                service_name: match uri.get("serviceName").as_str() {
                    "" => uri.get("path"),
                    value => value.into(),
                },
            },
            _ => return Err(LinkError::Unsupported),
        };

        Ok(trojan)
    }
    pub fn query(&self) -> Vec<(&str, &str)> {
        match self {
            Self::Original => vec![("type", "original")],
            Self::WebSocket { host, path } => vec![("type", "ws"), ("host", host), ("path", path)],
            Self::Grpc { service_name } => vec![("type", "grpc"), ("serviceName", service_name)],
            Self::HttpUpgrade { host, path } => {
                vec![("type", "httpupgrade"), ("host", host), ("path", path)]
            }
        }
    }
}
