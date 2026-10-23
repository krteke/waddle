use super::{Decode, Encode, NodeConfig, uri::*};
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Trojan {
    pub password: String,
    pub sni: String,
    pub insecure: bool,
}

impl Decode for Trojan {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let uri = Uri::parse(link, &["trojan"])?;

        if !uri.get("type").is_empty() {
            return Err(LinkError::Unsupported);
        }

        let password = uri.auth()?;
        required(&password)?;

        Ok(NodeConfig {
            name: uri.name()?,
            server: uri.server(),
            port: uri.port(443),
            protocol: Self {
                password,
                sni: uri.sni()?,
                insecure: uri.insecure()?,
            },
        })
    }
}

impl Encode for Trojan {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        required(&self.password)?;

        build(
            "trojan",
            server,
            port,
            name,
            &self.password,
            "",
            &[("sni", &self.sni), ("allowInsecure", flag(self.insecure))],
        )
    }
}

use super::TrojanTransport;

#[derive(Clone, Debug, PartialEq, Default)]
pub enum TrojanEncryption {
    #[default]
    None,
    Shadowsocks {
        cipher: String,
        password: String,
    },
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TrojanGo {
    pub password: String,
    pub sni: String,
    pub insecure: bool,
    pub transport: TrojanTransport,
    pub encryption: TrojanEncryption,
}

impl Decode for TrojanGo {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let uri = Uri::parse(link, &["trojan-go"])?;
        let password = uri.auth()?;

        required(&password)?;

        let encryption = match uri.get("encryption").as_str() {
            "" | "none" => TrojanEncryption::None,
            value => {
                let parts = value.strip_prefix("ss;").ok_or(LinkError::Unsupported)?;
                let (cipher, password) = parts.split_once(';').ok_or(LinkError::Structure)?;

                choice(
                    cipher,
                    &[
                        "aes-128-gcm",
                        "aes-256-gcm",
                        "chacha20-poly1305",
                        "chacha20-ietf-poly1305",
                    ],
                )?;

                required(password)?;

                TrojanEncryption::Shadowsocks {
                    cipher: cipher.into(),
                    password: password.into(),
                }
            }
        };

        Ok(NodeConfig {
            name: uri.name()?,
            server: uri.server(),
            port: uri.port(443),
            protocol: Self {
                password,
                sni: uri.sni()?,
                insecure: uri.insecure()?,
                transport: TrojanTransport::from_uri(&uri)?,
                encryption,
            },
        })
    }
}

impl Encode for TrojanGo {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        required(&self.password)?;

        let encryption = match &self.encryption {
            TrojanEncryption::None => String::new(),
            TrojanEncryption::Shadowsocks { cipher, password } => {
                required(password)?;
                format!("ss;{cipher};{password}")
            }
        };

        let mut params = self.transport.query();

        params.extend([
            ("sni", self.sni.as_str()),
            ("allowInsecure", flag(self.insecure)),
            ("encryption", &encryption),
        ]);

        build("trojan-go", server, port, name, &self.password, "", &params)
    }
}
