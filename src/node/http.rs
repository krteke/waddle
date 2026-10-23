use super::{Decode, Encode, NodeConfig, uri::*};
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Http {
    pub username: String,
    pub password: String,
}

impl Decode for Http {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let uri = Uri::parse(link, &["http"])?;
        let username = uri.username()?;

        Ok(NodeConfig {
            name: uri.name()?,
            server: uri.server(),
            port: uri.port(80),
            protocol: Self {
                username,
                password: uri.password()?,
            },
        })
    }
}

impl Encode for Http {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        build(
            "http",
            server,
            port,
            name,
            &self.username,
            &self.password,
            &[],
        )
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Https {
    pub username: String,
    pub password: String,
    pub sni: String,
    pub insecure: bool,
}

impl Decode for Https {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let uri = Uri::parse(link, &["https"])?;
        let username = uri.username()?;

        Ok(NodeConfig {
            name: uri.name()?,
            server: uri.server(),
            port: uri.port(443),
            protocol: Self {
                username,
                password: uri.password()?,
                sni: uri.get("sni"),
                insecure: uri.insecure()?,
            },
        })
    }
}

impl Encode for Https {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        build(
            "https",
            server,
            port,
            name,
            &self.username,
            &self.password,
            &[("sni", &self.sni), ("allowInsecure", flag(self.insecure))],
        )
    }
}
