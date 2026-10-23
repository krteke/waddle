use super::{Decode, Encode, NodeConfig, TcpHeader, Transport, V2rayTls, uri::*};
use crate::error::LinkError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Vmess {
    pub uuid: String,
    pub transport: Transport,
    pub tls: V2rayTls,
    pub insecure: bool,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct VmessJson {
    ps: String,
    add: String,
    id: String,
    #[serde(deserialize_with = "number")]
    port: u16,
    #[serde(deserialize_with = "number")]
    aid: u16,
    net: String,
    #[serde(rename = "type")]
    header_type: String,
    host: String,
    path: String,
    tls: String,
    sni: String,
    alpn: String,
    fp: String,
    pbk: String,
    sid: String,
    spx: String,
    #[serde(rename = "allowInsecure", deserialize_with = "json_bool")]
    insecure: bool,
}

fn number<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u16, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Number {
        Number(u16),
        Text(String),
    }

    match Number::deserialize(d)? {
        Number::Number(v) => Ok(v),
        Number::Text(v) => v.parse().map_err(serde::de::Error::custom),
    }
}

fn json_bool<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Boolean {
        Bool(bool),
        Number(u8),
    }

    match Boolean::deserialize(d)? {
        Boolean::Bool(v) => Ok(v),
        Boolean::Number(0) => Ok(false),
        Boolean::Number(1) => Ok(true),
        Boolean::Number(_) => Err(serde::de::Error::custom("expected boolean or 0/1")),
    }
}

impl Decode for Vmess {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let body = link
            .trim()
            .strip_prefix("vmess://")
            .ok_or(LinkError::Unsupported)?;

        let node = if body.split('#').next().unwrap().contains('@') {
            let uri = Uri::parse(link, &["vmess"])?;

            NodeConfig {
                name: uri.name()?,
                server: uri.server(),
                port: uri.port(443),
                protocol: Self {
                    uuid: uri.auth()?,
                    transport: Transport::from_uri(&uri)?,
                    tls: V2rayTls::from_uri(&uri)?,
                    insecure: uri.insecure()?,
                },
            }
        } else {
            let data: VmessJson =
                serde_json::from_str(&base64_decode(body.split('#').next().unwrap())?)?;

            if data.aid != 0 {
                return Err(LinkError::Unsupported);
            }

            let tls = match data.tls.as_str() {
                "" | "none" => V2rayTls::None,
                "tls" => V2rayTls::Tls {
                    sni: data.sni,
                    alpn: data.alpn,
                },
                "reality" => {
                    required(&data.pbk)?;
                    V2rayTls::Reality {
                        sni: data.sni,
                        alpn: data.alpn,
                        fingerprint: data.fp,
                        public_key: data.pbk,
                        short_id: data.sid,
                        spider_x: data.spx,
                    }
                }
                _ => return Err(LinkError::Unsupported),
            };

            NodeConfig {
                name: data.ps,
                server: data.add,
                port: data.port,
                protocol: Self {
                    uuid: data.id,
                    transport: Transport::from_parts(
                        &data.net,
                        &data.header_type,
                        data.host,
                        data.path,
                    )?,
                    tls,
                    insecure: data.insecure,
                },
            }
        };

        required(&node.protocol.uuid)?;
        required(&node.server)?;

        if node.port == 0 {
            return Err(LinkError::Structure);
        }

        Ok(node)
    }
}

impl Encode for Vmess {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        required(&self.uuid)?;
        build("vmess", server, port, "", "", "", &[])?;

        let mut data = VmessJson {
            ps: name.into(),
            add: server.trim_matches(['[', ']']).into(),
            port,
            id: self.uuid.clone(),
            ..Default::default()
        };

        match &self.transport {
            Transport::Tcp { header } => {
                data.net = "tcp".into();

                match header {
                    TcpHeader::None => data.header_type = "none".into(),
                    TcpHeader::Http { host, path } => {
                        data.header_type = "http".into();
                        data.host = host.clone();
                        data.path = path.clone();
                    }
                }
            }
            Transport::WebSocket { host, path } => {
                data.net = "ws".into();
                data.host = host.clone();
                data.path = path.clone();
            }
            Transport::H2 { host, path } => {
                data.net = "h2".into();
                data.host = host.clone();
                data.path = path.clone();
            }
            Transport::Grpc { service_name } => {
                data.net = "grpc".into();
                data.path = service_name.clone();
            }
            Transport::HttpUpgrade { host, path } => {
                data.net = "httpupgrade".into();
                data.host = host.clone();
                data.path = path.clone();
            }
        }

        match &self.tls {
            V2rayTls::None => data.tls = "none".into(),
            V2rayTls::Tls { sni, alpn } => {
                data.tls = "tls".into();
                data.sni = sni.clone();
                data.alpn = alpn.clone();
                data.insecure = self.insecure;
            }
            V2rayTls::Reality {
                sni,
                alpn,
                fingerprint,
                public_key,
                short_id,
                spider_x,
            } => {
                required(public_key)?;
                data.tls = "reality".into();
                data.sni = sni.clone();
                data.alpn = alpn.clone();
                data.fp = fingerprint.clone();
                data.pbk = public_key.clone();
                data.sid = short_id.clone();
                data.spx = spider_x.clone();
                data.insecure = self.insecure;
            }
        }

        Ok(format!(
            "vmess://{}",
            base64_encode(serde_json::to_vec(&data)?)
        ))
    }
}
