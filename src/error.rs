use std::net::AddrParseError;

use relm4::gtk::gio;
use rust_i18n::t;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Could not parse IP address {value:?}: {source}")]
pub struct InvalidIpAddress {
    pub value: String,
    #[source]
    pub source: AddrParseError,
}

#[derive(Debug, thiserror::Error)]
pub enum ServerSettingsError {
    #[error("Server name is empty")]
    EmptyName,
    #[error("Server name {name:?} already exists")]
    DuplicateName { name: String },
    #[error(transparent)]
    InvalidIp(#[from] InvalidIpAddress),
    #[error("Invalid port {value:?}; expected an integer from 1 to 65535")]
    InvalidPort { value: String },
    #[error("Could not save settings: {0}")]
    Save(#[from] gio::glib::BoolError),
}

impl ServerSettingsError {
    pub fn user_message(&self) -> String {
        match self {
            Self::EmptyName => t!("error.empty_name").to_string(),
            Self::DuplicateName { .. } => t!("error.duplicate_name").to_string(),
            Self::InvalidIp(_) => t!("error.invalid_ip").to_string(),
            Self::InvalidPort { .. } => t!("error.invalid_port").to_string(),
            Self::Save(_) => t!("error.save").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_ip_message_does_not_expose_the_address_or_parse_error() {
        let error = ServerSettingsError::InvalidIp(InvalidIpAddress {
            value: "not-an-ip".into(),
            source: "not-an-ip".parse::<std::net::IpAddr>().unwrap_err(),
        });
        assert_eq!(error.user_message(), "Enter a valid IP address.");
    }

    #[test]
    fn settings_write_message_does_not_expose_the_backend_error() {
        let error = ServerSettingsError::Save(gio::glib::bool_error!("private backend details"));
        assert_eq!(error.user_message(), "Could not save settings.");
    }
}
