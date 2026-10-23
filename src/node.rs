mod anytls;
mod http;
mod hysteria2;
mod juicity;
mod shadowsocks;
mod shadowsocksr;
mod socks5;
mod tls;
mod transport;
mod trojan;
mod tuic;
mod uri;
mod vless;
mod vmess;

use crate::error::LinkError;

pub use anytls::AnyTls;
pub use http::{Http, Https};
pub use hysteria2::Hysteria2;
pub use juicity::Juicity;
pub use shadowsocks::{Plugin, Shadowsocks};
pub use shadowsocksr::ShadowsocksR;
pub use socks5::Socks5;
pub use tls::V2rayTls;
pub use transport::{TcpHeader, Transport, TrojanTransport};
pub use trojan::{Trojan, TrojanEncryption, TrojanGo};
pub use tuic::Tuic;
pub use vless::Vless;
pub use vmess::Vmess;

#[derive(Clone, Debug, PartialEq)]
pub struct NodeConfig<P = Protocol> {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub protocol: P,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            server: String::new(),
            port: 443,
            protocol: Vmess::default().into(),
        }
    }
}

impl<P: Encode> NodeConfig<P> {
    pub fn encode(&self) -> Result<String, LinkError> {
        self.protocol.encode(&self.name, &self.server, self.port)
    }
}

pub trait Decode: Sized {
    fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError>;
}

pub trait Encode {
    fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError>;
}

pub trait DecodeExt {
    fn decode<T: Decode>(&self) -> Result<NodeConfig<T>, LinkError>;
}

impl DecodeExt for str {
    fn decode<T: Decode>(&self) -> Result<NodeConfig<T>, LinkError> {
        T::decode(self)
    }
}

macro_rules! protocols {
    ($($variant:ident => $scheme:literal, $label:literal);+ $(;)?) => {
        #[derive(Clone, Debug, PartialEq)]
        pub enum Protocol { $($variant($variant)),+ }

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum ProtocolKind { $($variant),+ }

        impl ProtocolKind {
            pub const ALL: [Self; 13] = [$(Self::$variant),+];

            pub const fn label(self) -> &'static str {
                match self { $(Self::$variant => $label),+ }
            }

            pub fn empty(self) -> Protocol {
                match self { $(Self::$variant => $variant::default().into()),+ }
            }
        }

        impl Protocol {
            pub fn kind(&self) -> ProtocolKind {
                match self { $(Self::$variant(_) => ProtocolKind::$variant),+ }
            }
        }

        impl Encode for Protocol {
            fn encode(&self, name: &str, server: &str, port: u16) -> Result<String, LinkError> {
                match self { $(Self::$variant(node) => node.encode(name, server, port)),+ }
            }
        }

        impl Decode for Protocol {
            fn decode(link: &str) -> Result<NodeConfig<Self>, LinkError> {
                let link = link.trim();

                match link.split_once("://").ok_or(LinkError::Structure)?.0 {
                    $($scheme => Ok($variant::decode(link)?.into())),+,
                    "hy2" => Ok(Hysteria2::decode(link)?.into()),
                    _ => Err(LinkError::Unsupported),
                }
            }
        }

        $(impl From<$variant> for Protocol {
            fn from(node: $variant) -> Self { Self::$variant(node) }
        }
        impl From<NodeConfig<$variant>> for NodeConfig {
            fn from(node: NodeConfig<$variant>) -> Self {
                Self { name: node.name, server: node.server, port: node.port, protocol: node.protocol.into() }
            }
        })+
    };
}

protocols! {
    Vmess => "vmess", "VMess";
    Vless => "vless", "VLESS";
    Shadowsocks => "ss", "Shadowsocks";
    ShadowsocksR => "ssr", "ShadowsocksR";
    Trojan => "trojan", "Trojan";
    TrojanGo => "trojan-go", "Trojan-Go";
    Tuic => "tuic", "TUIC";
    Juicity => "juicity", "Juicity";
    Hysteria2 => "hysteria2", "Hysteria2";
    AnyTls => "anytls", "AnyTLS";
    Http => "http", "HTTP";
    Https => "https", "HTTPS";
    Socks5 => "socks5", "SOCKS5";
}

#[cfg(test)]
mod tests;
