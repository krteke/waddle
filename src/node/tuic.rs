use super::{Decode, Encode, NodeConfig, uri::*};
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Tuic {
    pub uuid: String,
    pub password: String,
    pub sni: String,
    pub insecure: bool,
    pub alpn: String,
    pub disable_sni: bool,
    pub congestion_control: String,
    pub udp_relay_mode: String,
}

impl Decode for Tuic {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let uri = Uri::parse(link, &["tuic"])?;
        let uuid = uri.username()?;

        required(&uuid)?;

        choice(&uri.get("congestion_control"), &["", "bbr", "cubic"])?;
        choice(&uri.get("udp_relay_mode"), &["", "native", "quic"])?;

        Ok(NodeConfig {
            name: uri.name()?,
            server: uri.server(),
            port: uri.port(443),
            protocol: Self {
                uuid,
                password: uri.password()?,
                sni: uri.sni()?,
                insecure: uri.insecure()?,
                alpn: uri.get("alpn"),
                disable_sni: uri.flag("disable_sni")?,
                congestion_control: uri.get("congestion_control"),
                udp_relay_mode: uri.get("udp_relay_mode"),
            },
        })
    }
}

impl Encode for Tuic {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        required(&self.uuid)?;

        build(
            "tuic",
            server,
            port,
            name,
            &self.uuid,
            &self.password,
            &[
                ("sni", &self.sni),
                ("allow_insecure", flag(self.insecure)),
                ("alpn", &self.alpn),
                ("disable_sni", flag(self.disable_sni)),
                ("congestion_control", &self.congestion_control),
                ("udp_relay_mode", &self.udp_relay_mode),
            ],
        )
    }
}
