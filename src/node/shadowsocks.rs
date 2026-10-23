use super::{Decode, Encode, NodeConfig, uri::*};
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq)]
pub struct Shadowsocks {
    pub method: String,
    pub password: String,
    pub plugin: Plugin,
}

impl Shadowsocks {
    pub const CIPHERS: &[&str] = &[
        "aes-128-gcm",
        "aes-256-gcm",
        "chacha20-poly1305",
        "chacha20-ietf-poly1305",
        "2022-blake3-aes-128-gcm",
        "2022-blake3-aes-256-gcm",
        "2022-blake3-chacha20-poly1305",
        "aes-128-cfb",
        "aes-192-cfb",
        "aes-256-cfb",
        "aes-128-ctr",
        "aes-192-ctr",
        "aes-256-ctr",
        "aes-128-ofb",
        "aes-192-ofb",
        "aes-256-ofb",
        "des-cfb",
        "bf-cfb",
        "cast5-cfb",
        "rc4-md5",
        "chacha20-ietf",
        "salsa20",
        "camellia-128-cfb",
        "camellia-192-cfb",
        "camellia-256-cfb",
        "idea-cfb",
        "rc2-cfb",
        "seed-cfb",
        "plain",
        "none",
    ];
}

impl Default for Shadowsocks {
    fn default() -> Self {
        Self {
            method: "aes-128-gcm".into(),
            password: String::new(),
            plugin: Plugin::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Plugin {
    #[default]
    None,
    SimpleObfs {
        mode: String,
        host: String,
        path: String,
    },
    V2ray {
        tls: bool,
        host: String,
    },
    ShadowTls {
        version: String,
        host: String,
        password: String,
    },
}

impl Plugin {
    fn parse(value: &str) -> Result<Self, LinkError> {
        let mut parts = value.split(';');

        let name = parts.next().unwrap();

        let options: Vec<_> = parts
            .map(|p| p.split_once('=').unwrap_or((p, "")))
            .collect();

        let get = |key| {
            options
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| *v)
                .unwrap_or("")
        };

        Ok(match name {
            "" => Self::None,
            "simple-obfs" => {
                let mode = get("obfs");

                choice(mode, &["http", "tls"])?;

                Self::SimpleObfs {
                    mode: mode.into(),
                    host: get("obfs-host").into(),
                    path: get("obfs-path").into(),
                }
            }
            "v2ray-plugin" => {
                choice(get("mode"), &["", "websocket"])?;

                Self::V2ray {
                    tls: options.iter().any(|(k, _)| *k == "tls"),
                    host: get("host").into(),
                }
            }
            "shadow-tls" => {
                let version = match get("version") {
                    "" => "3",
                    v => v,
                };

                choice(version, &["2", "3"])?;
                required(get("password"))?;

                Self::ShadowTls {
                    version: version.into(),
                    host: get("host").into(),
                    password: get("password").into(),
                }
            }
            _ => return Err(LinkError::Unsupported),
        })
    }

    fn encode(&self) -> Result<String, LinkError> {
        let options = match self {
            Self::None => return Ok(String::new()),
            Self::SimpleObfs { mode, host, path } => vec![
                ("simple-obfs", ""),
                ("obfs", mode.as_str()),
                ("obfs-host", host),
                ("obfs-path", if mode == "http" { path } else { "" }),
            ],
            Self::V2ray { tls, host } => {
                let mut options = vec![("v2ray-plugin", ""), ("host", host.as_str())];

                if *tls {
                    options.push(("tls", ""));
                }

                options
            }
            Self::ShadowTls {
                version,
                host,
                password,
            } => {
                required(password)?;

                vec![
                    ("shadow-tls", ""),
                    ("version", version.as_str()),
                    ("host", host),
                    ("password", password),
                ]
            }
        };

        if options.iter().any(|(_, v)| v.contains(';')) {
            return Err(LinkError::Structure);
        }

        Ok(options
            .into_iter()
            .filter_map(|(k, v)| {
                if v.is_empty() {
                    matches!(k, "simple-obfs" | "v2ray-plugin" | "shadow-tls" | "tls")
                        .then(|| k.into())
                } else {
                    Some(format!("{k}={v}"))
                }
            })
            .collect::<Vec<String>>()
            .join(";"))
    }
}

impl Decode for Shadowsocks {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let body = link
            .trim()
            .strip_prefix("ss://")
            .ok_or(LinkError::Unsupported)?;

        let normalized;

        let link = if !body.split('#').next().unwrap().contains('@') {
            let (body, name) = body.split_once('#').unwrap_or((body, ""));
            let decoded = base64_decode(body)?;

            let (userinfo, address) = decoded.rsplit_once('@').ok_or(LinkError::Structure)?;
            normalized = format!("ss://{}@{address}#{name}", base64_url(userinfo));

            &normalized
        } else {
            link
        };

        let uri = Uri::parse(link, &["ss"])?;

        let (method, password) = if uri.0.password().is_some() {
            (uri.username()?, uri.password()?)
        } else {
            let auth = base64_decode(&uri.username()?)?;
            let (method, password) = auth.split_once(':').ok_or(LinkError::Structure)?;

            (method.into(), password.into())
        };

        choice(&method, Self::CIPHERS)?;
        required(&password)?;

        Ok(NodeConfig {
            name: uri.name()?,
            server: uri.server(),
            port: uri.0.port().ok_or(LinkError::Structure)?,
            protocol: Self {
                method,
                password,
                plugin: Plugin::parse(&uri.get("plugin"))?,
            },
        })
    }
}

impl Encode for Shadowsocks {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        required(&self.password)?;

        let auth = base64_url(format!("{}:{}", self.method, self.password));

        build(
            "ss",
            server,
            port,
            name,
            &auth,
            "",
            &[("plugin", &self.plugin.encode()?)],
        )
    }
}
