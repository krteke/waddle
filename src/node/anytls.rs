use super::{Decode, Encode, NodeConfig, uri::*};
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct AnyTls {
    pub auth: String,
    pub sni: String,
    pub insecure: bool,
}

impl Decode for AnyTls {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let uri = Uri::parse(link, &["anytls"])?;
        let auth = uri.auth()?;

        Ok(NodeConfig {
            name: uri.name()?,
            server: uri.server(),
            port: uri.port(443),
            protocol: Self {
                auth,
                sni: uri.sni()?,
                insecure: uri.flag("insecure")?,
            },
        })
    }
}

impl Encode for AnyTls {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        build(
            "anytls",
            server,
            port,
            name,
            &self.auth,
            "",
            &[("sni", &self.sni), ("insecure", flag(self.insecure))],
        )
    }
}
