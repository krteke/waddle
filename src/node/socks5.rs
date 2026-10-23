use super::{Decode, Encode, NodeConfig, uri::*};
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Socks5 {
    pub username: String,
    pub password: String,
}

impl Decode for Socks5 {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let uri = Uri::parse(link, &["socks5"])?;
        let username = uri.username()?;

        Ok(NodeConfig {
            name: uri.name()?,
            server: uri.server(),
            port: uri.port(1080),
            protocol: Self {
                username,
                password: uri.password()?,
            },
        })
    }
}

impl Encode for Socks5 {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        build(
            "socks5",
            server,
            port,
            name,
            &self.username,
            &self.password,
            &[],
        )
    }
}
