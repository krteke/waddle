use super::uri::{Uri, required};
use crate::error::LinkError;

#[derive(Clone, Debug, PartialEq, Default)]
pub enum V2rayTls {
    #[default]
    None,
    Tls {
        sni: String,
        alpn: String,
    },
    Reality {
        sni: String,
        alpn: String,
        fingerprint: String,
        public_key: String,
        short_id: String,
        spider_x: String,
    },
}

impl V2rayTls {
    pub fn from_uri(uri: &Uri) -> Result<Self, LinkError> {
        let v2ray = match uri.get("security").as_str() {
            "" | "none" => Self::None,
            "tls" => Self::Tls {
                sni: uri.get("sni"),
                alpn: uri.get("alpn"),
            },
            "reality" => {
                let public_key = uri.get("pbk");
                required(&public_key)?;

                Self::Reality {
                    sni: uri.get("sni"),
                    alpn: uri.get("alpn"),
                    fingerprint: uri.get("fp"),
                    public_key,
                    short_id: uri.get("sid"),
                    spider_x: uri.get("spx"),
                }
            }
            _ => return Err(LinkError::Unsupported),
        };

        Ok(v2ray)
    }

    pub fn query(&self) -> Vec<(&str, &str)> {
        match self {
            Self::None => vec![("security", "none")],
            Self::Tls { sni, alpn } => vec![("security", "tls"), ("sni", sni), ("alpn", alpn)],
            Self::Reality {
                sni,
                alpn,
                fingerprint,
                public_key,
                short_id,
                spider_x,
            } => vec![
                ("security", "reality"),
                ("sni", sni),
                ("alpn", alpn),
                ("fp", fingerprint),
                ("pbk", public_key),
                ("sid", short_id),
                ("spx", spider_x),
            ],
        }
    }
}
