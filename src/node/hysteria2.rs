use super::{Decode, Encode, NodeConfig, uri::*};
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Hysteria2 {
    pub auth: String,
    pub sni: String,
    pub insecure: bool,
    pub obfs: String,
    pub obfs_password: String,
    pub pin_sha256: String,
}

impl Decode for Hysteria2 {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let uri = Uri::parse(link, &["hysteria2", "hy2"])?;

        let mut auth = uri.username()?;
        let password = uri.password()?;

        if !password.is_empty() {
            auth.push(':');
            auth.push_str(&password);
        }

        choice(&uri.get("obfs"), &["", "salamander", "gecko"])?;

        let obfs_password = match uri.get("obfs-password").as_str() {
            "" => uri.get("obfsPassword"),
            value => value.into(),
        };

        if uri.get("obfs") == "salamander" {
            required(&obfs_password)?;
        }

        Ok(NodeConfig {
            name: uri.name()?,
            server: uri.server(),
            port: uri.port(443),
            protocol: Self {
                auth,
                sni: uri.get("sni"),
                insecure: uri.flag("insecure")?,
                obfs: uri.get("obfs"),
                obfs_password,
                pin_sha256: uri.get("pinSHA256"),
            },
        })
    }
}

impl Encode for Hysteria2 {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        if self.obfs == "salamander" {
            required(&self.obfs_password)?;
        }

        build(
            "hysteria2",
            server,
            port,
            name,
            &self.auth,
            "",
            &[
                ("sni", &self.sni),
                ("insecure", flag(self.insecure)),
                ("obfs", &self.obfs),
                ("obfs-password", &self.obfs_password),
                ("pinSHA256", &self.pin_sha256),
            ],
        )
    }
}
