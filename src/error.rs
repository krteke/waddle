use relm4::gtk::gio;
use rust_i18n::t;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Server rejected authentication")]
    Unauthorized,
    #[error("Health check returned status {status}; expected 1")]
    Unhealthy { status: i64 },
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("GraphQL request failed: {0:?}")]
    Graphql(Vec<graphql_client::Error>),
    #[error("GraphQL response contained no data")]
    MissingData,
    #[error("Server returned an invalid authentication token: {0}")]
    InvalidToken(#[from] reqwest::header::InvalidHeaderValue),
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("Could not restore the saved session: {0}")]
    Session(#[source] ApiError),
    #[error("Could not clear the saved session: {0}")]
    SignOut(#[source] CredentialError),
    #[error("Server health check failed: {0}")]
    HealthCheck(#[source] ApiError),
    #[error("Could not check administrator status: {0}")]
    Check(#[source] ApiError),
    #[error("Could not sign in: {0}")]
    Login(#[source] ApiError),
    #[error("Could not create administrator: {0}")]
    Create(#[source] ApiError),
    #[error("Could not read saved credentials: {0}")]
    ReadCredentials(#[source] CredentialError),
    #[error("Signed in, but could not save credentials: {0}")]
    SaveCredentials(#[source] CredentialError),
}

impl ConnectionError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Session(_) => t!("auth.connection_error"),
            Self::SignOut(_) => t!("auth.sign_out_error"),
            Self::HealthCheck(_) => t!("auth.health_error"),
            Self::Check(_) | Self::Login(ApiError::Http(_)) | Self::Create(ApiError::Http(_)) => {
                t!("auth.connection_error")
            }
            Self::Login(_) => t!("auth.login_error"),
            Self::Create(_) => t!("auth.create_error"),
            Self::ReadCredentials(_) => t!("auth.read_error"),
            Self::SaveCredentials(_) => t!("auth.save_error"),
        }
        .to_string()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("Secret Service operation failed: {0}")]
    Keyring(#[source] Box<oo7::Error>),
    #[error("Could not encode or decode saved credentials: {0}")]
    Encoding(#[from] serde_json::Error),
}

impl From<oo7::Error> for CredentialError {
    fn from(error: oo7::Error) -> Self {
        Self::Keyring(Box::new(error))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Could not parse server URL {value:?}: {source}")]
pub struct InvalidServerUrl {
    pub value: String,
    #[source]
    pub source: url::ParseError,
}

#[derive(Debug, thiserror::Error)]
pub enum ServerSettingsError {
    #[error("Server removed, but saved credentials could not be deleted: {0}")]
    DeleteCredentials(#[source] CredentialError),
    #[error("Server name is empty")]
    EmptyName,
    #[error("Server name {name:?} already exists")]
    DuplicateName { name: String },
    #[error(transparent)]
    InvalidUrl(#[from] InvalidServerUrl),
    #[error("Could not save settings: {0}")]
    Save(#[from] gio::glib::BoolError),
}

impl ServerSettingsError {
    pub fn user_message(&self) -> String {
        match self {
            Self::DeleteCredentials(_) => t!("error.delete_credentials").to_string(),
            Self::EmptyName => t!("error.empty_name").to_string(),
            Self::DuplicateName { .. } => t!("error.duplicate_name").to_string(),
            Self::InvalidUrl(_) => t!("error.invalid_url").to_string(),
            Self::Save(_) => t!("error.save").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_url_message_does_not_expose_the_address_or_parse_error() {
        let error = ServerSettingsError::InvalidUrl(InvalidServerUrl {
            value: "not-a-url".into(),
            source: "not-a-url".parse::<reqwest::Url>().unwrap_err(),
        });
        assert_eq!(error.user_message(), "Enter a valid server URL.");
    }

    #[test]
    fn settings_write_message_does_not_expose_the_backend_error() {
        let error = ServerSettingsError::Save(gio::glib::bool_error!("private backend details"));
        assert_eq!(error.user_message(), "Could not save settings.");
    }
}
