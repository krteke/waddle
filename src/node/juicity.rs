use super::{Decode, Encode, NodeConfig, uri::*};
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Juicity {
    pub uuid: String,
    pub password: String,
    pub sni: String,
    pub insecure: bool,
    pub congestion_control: String,
    pub pinned_certchain_sha256: String,
}

impl Decode for Juicity {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let uri = Uri::parse(link, &["juicity"])?;
        let uuid = uri.username()?;

        required(&uuid)?;

        choice(&uri.get("congestion_control"), &["", "bbr", "cubic"])?;

        Ok(NodeConfig {
            name: uri.name()?,
            server: uri.server(),
            port: uri.port(443),
            protocol: Self {
                uuid,
                password: uri.password()?,
                sni: uri.sni()?,
                insecure: uri.insecure()?,
                congestion_control: uri.get("congestion_control"),
                pinned_certchain_sha256: uri.get("pinned_certchain_sha256"),
            },
        })
    }
}

impl Encode for Juicity {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        required(&self.uuid)?;

        build(
            "juicity",
            server,
            port,
            name,
            &self.uuid,
            &self.password,
            &[
                ("sni", &self.sni),
                ("allow_insecure", flag(self.insecure)),
                ("congestion_control", &self.congestion_control),
                ("pinned_certchain_sha256", &self.pinned_certchain_sha256),
            ],
        )
    }
}
