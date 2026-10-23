use crate::error::LinkError;
use base64::{
    Engine,
    engine::DecodePaddingMode,
    engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig, STANDARD, URL_SAFE_NO_PAD},
};
use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use url::Url;

const COMPONENT: &percent_encoding::AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

pub fn unescape(value: &str) -> Result<String, LinkError> {
    Ok(String::from_utf8(percent_decode_str(value).collect())?)
}

pub fn base64_decode(value: &str) -> Result<String, LinkError> {
    const CONFIG: GeneralPurposeConfig =
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent);

    let bytes = GeneralPurpose::new(&base64::alphabet::STANDARD, CONFIG)
        .decode(value)
        .or_else(|_| GeneralPurpose::new(&base64::alphabet::URL_SAFE, CONFIG).decode(value))?;

    Ok(String::from_utf8(bytes)?)
}

pub fn base64_encode(value: impl AsRef<[u8]>) -> String {
    STANDARD.encode(value)
}

pub fn base64_url(value: impl AsRef<[u8]>) -> String {
    URL_SAFE_NO_PAD.encode(value)
}

pub struct Uri(pub Url);

impl Uri {
    pub fn parse(link: &str, schemes: &[&str]) -> Result<Self, LinkError> {
        let url = Url::parse(link.trim())?;

        if !schemes.contains(&url.scheme()) {
            return Err(LinkError::Unsupported);
        }

        if url.host_str().is_none() || !matches!(url.path(), "" | "/") || url.port() == Some(0) {
            return Err(LinkError::Structure);
        }

        Ok(Self(url))
    }

    pub fn name(&self) -> Result<String, LinkError> {
        unescape(self.0.fragment().unwrap_or_default())
    }

    pub fn server(&self) -> String {
        self.0.host_str().unwrap().trim_matches(['[', ']']).into()
    }

    pub fn port(&self, default: u16) -> u16 {
        self.0.port_or_known_default().unwrap_or(default)
    }

    pub fn username(&self) -> Result<String, LinkError> {
        unescape(self.0.username())
    }

    pub fn password(&self) -> Result<String, LinkError> {
        unescape(self.0.password().unwrap_or_default())
    }

    pub fn auth(&self) -> Result<String, LinkError> {
        if self.0.password().is_some() {
            return Err(LinkError::Structure);
        }

        self.username()
    }

    pub fn get(&self, key: &str) -> String {
        self.0
            .query_pairs()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
            .unwrap_or_default()
    }

    pub fn flag(&self, key: &str) -> Result<bool, LinkError> {
        boolean(&self.get(key))
    }

    pub fn sni(&self) -> Result<String, LinkError> {
        let sni = self.get("sni");
        let peer = self.get("peer");

        if !sni.is_empty() && !peer.is_empty() && sni != peer {
            return Err(LinkError::Structure);
        }

        Ok(if sni.is_empty() { peer } else { sni })
    }

    pub fn insecure(&self) -> Result<bool, LinkError> {
        let mut enabled = false;

        for key in [
            "allowInsecure",
            "allow_insecure",
            "allowinsecure",
            "skipVerify",
        ] {
            enabled |= self.flag(key)?;
        }

        Ok(enabled)
    }
}

pub fn boolean(value: &str) -> Result<bool, LinkError> {
    match value {
        "" | "0" | "false" | "False" | "FALSE" => Ok(false),
        "1" | "true" | "True" | "TRUE" => Ok(true),
        _ => Err(LinkError::Structure),
    }
}

pub fn required(value: &str) -> Result<(), LinkError> {
    if value.is_empty() {
        Err(LinkError::Structure)
    } else {
        Ok(())
    }
}

pub fn choice(value: &str, supported: &[&str]) -> Result<(), LinkError> {
    if supported.contains(&value) {
        Ok(())
    } else {
        Err(LinkError::Unsupported)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build(
    scheme: &str,
    server: &str,
    port: u16,
    name: &str,
    username: &str,
    password: &str,
    params: &[(&str, &str)],
) -> Result<String, LinkError> {
    let host = if server.contains(':') && !server.starts_with('[') {
        format!("[{server}]")
    } else {
        server.into()
    };

    let mut url = Url::parse(&format!("{scheme}://{host}:{port}"))?;

    if url.host_str().is_none()
        || !url.username().is_empty()
        || !matches!(url.path(), "" | "/")
        || url.query().is_some()
        || url.fragment().is_some()
        || port == 0
    {
        return Err(LinkError::Structure);
    }

    url.set_username(&utf8_percent_encode(username, COMPONENT).to_string())
        .map_err(|_| LinkError::Structure)?;

    url.set_password(
        (!password.is_empty())
            .then(|| utf8_percent_encode(password, COMPONENT).to_string())
            .as_deref(),
    )
    .map_err(|_| LinkError::Structure)?;

    for &(key, value) in params {
        if !value.is_empty() {
            url.query_pairs_mut().append_pair(key, value);
        }
    }

    url.set_fragment(
        (!name.is_empty())
            .then(|| utf8_percent_encode(name, COMPONENT).to_string())
            .as_deref(),
    );

    Ok(url.into())
}

pub fn flag(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}
