use super::{Decode, Encode, NodeConfig, uri::*};
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq)]
pub struct ShadowsocksR {
    pub password: String,
    pub method: String,
    pub protocol: String,
    pub protocol_param: String,
    pub obfs: String,
    pub obfs_param: String,
}

impl ShadowsocksR {
    pub const CIPHERS: &[&str] = &[
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

impl Default for ShadowsocksR {
    fn default() -> Self {
        Self {
            password: String::new(),
            method: "aes-128-cfb".into(),
            protocol: "origin".into(),
            protocol_param: String::new(),
            obfs: "plain".into(),
            obfs_param: String::new(),
        }
    }
}

impl Decode for ShadowsocksR {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let body = link
            .trim()
            .strip_prefix("ssr://")
            .ok_or(LinkError::Unsupported)?;

        let decoded = base64_decode(body)?;

        let (address, query) = decoded.split_once("/?").unwrap_or((&decoded, ""));
        let parts: Vec<_> = address.rsplitn(6, ':').collect();

        let [password, obfs, method, protocol, port, server] = parts.as_slice() else {
            return Err(LinkError::Structure);
        };

        let port = port.parse()?;

        required(server)?;

        if port == 0 {
            return Err(LinkError::Structure);
        }

        let get = |key| -> Result<String, LinkError> {
            let value = url::form_urlencoded::parse(query.as_bytes())
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.into_owned())
                .unwrap_or_default();

            base64_decode(&value)
        };

        choice(method, Self::CIPHERS)?;

        let password = base64_decode(password)?;

        required(&password)?;

        choice(
            protocol,
            &[
                "origin",
                "verify_sha1",
                "auth_sha1_v4",
                "auth_aes128_md5",
                "auth_aes128_sha1",
                "auth_chain_a",
                "auth_chain_b",
            ],
        )?;

        choice(
            obfs,
            &[
                "plain",
                "http_simple",
                "http_post",
                "random_head",
                "tls1.2_ticket_auth",
            ],
        )?;

        Ok(NodeConfig {
            name: get("remarks")?,
            server: (*server).into(),
            port,
            protocol: Self {
                password,
                method: (*method).into(),
                protocol: (*protocol).into(),
                protocol_param: get("protoparam")?,
                obfs: (*obfs).into(),
                obfs_param: get("obfsparam")?,
            },
        })
    }
}

impl Encode for ShadowsocksR {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        build("ssr", server, port, "", "", "", &[])?;

        required(&self.password)?;

        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("remarks", &base64_url(name))
            .append_pair("protoparam", &base64_url(&self.protocol_param))
            .append_pair("obfsparam", &base64_url(&self.obfs_param))
            .finish();

        Ok(format!(
            "ssr://{}",
            base64_url(format!(
                "{}:{}:{}:{}:{}:{}/?{}",
                server.trim_matches(['[', ']']),
                port,
                self.protocol,
                self.method,
                self.obfs,
                base64_url(&self.password),
                query
            ))
        ))
    }
}
