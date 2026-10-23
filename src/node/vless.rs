use super::{Decode, Encode, NodeConfig, Transport, V2rayTls, uri::*};
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Vless {
    pub uuid: String,
    pub flow: String,
    pub transport: Transport,
    pub tls: V2rayTls,
}

impl Decode for Vless {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
        let uri = Uri::parse(link, &["vless"])?;
        let uuid = uri.auth()?;

        required(&uuid)?;

        choice(&uri.get("encryption"), &["", "none"])?;

        let flow = match uri.get("flow").as_str() {
            "none" => String::new(),
            v => v.into(),
        };

        choice(&flow, &["", "xtls-rprx-vision", "xtls-rprx-vision-udp443"])?;

        Ok(NodeConfig {
            name: uri.name()?,
            server: uri.server(),
            port: uri.port(443),
            protocol: Self {
                uuid,
                flow,
                transport: Transport::from_uri(&uri)?,
                tls: V2rayTls::from_uri(&uri)?,
            },
        })
    }
}

impl Encode for Vless {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
        required(&self.uuid)?;

        if let V2rayTls::Reality { public_key, .. } = &self.tls {
            required(public_key)?;
        }

        let mut params = self.transport.query();

        params.extend(self.tls.query());
        params.extend([("encryption", "none"), ("flow", self.flow.as_str())]);

        build("vless", server, port, name, &self.uuid, "", &params)
    }
}
