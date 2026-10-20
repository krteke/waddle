use std::net::IpAddr;

use gio::prelude::*;
use relm4::gtk::gio;

use crate::error::{InvalidIpAddress, ServerSettingsError};

pub const APPLICATION_ID: &str = "io.github.krteke.Waddle";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Server {
    pub name: String,
    pub ip: Result<IpAddr, InvalidIpAddress>,
    pub port: u16,
    pub is_default: bool,
}

pub struct ServerSettings {
    settings: gio::Settings,
    servers: Vec<Server>,
}

impl ServerSettings {
    pub fn load(settings: gio::Settings) -> Self {
        let records = settings.get::<Vec<(String, String, u16, bool)>>("servers");

        let servers = records
            .into_iter()
            .map(|(name, ip, port, is_default)| {
                let ip = ip.parse().map_err(|source| {
                    let error = InvalidIpAddress { value: ip, source };
                    tracing::warn!(server = %name, error = %error, "Could not parse saved server address");
                    error
                });

                Server {
                    name,
                    ip,
                    port,
                    is_default,
                }
            })
            .collect();

        Self { settings, servers }
    }

    pub fn servers(&self) -> &[Server] {
        &self.servers
    }

    pub fn default_server_index(&self) -> Option<usize> {
        self.servers.iter().position(|server| server.is_default)
    }

    pub fn language(&self) -> gio::glib::GString {
        self.settings.string("language")
    }

    pub fn set_language(&self, language: &str) -> Result<(), ServerSettingsError> {
        self.settings
            .set_string("language", language)
            .inspect_err(|error| tracing::error!(%error, "Could not save language preference"))?;
        Ok(())
    }

    pub fn add_server(
        &mut self,
        name: &str,
        ip: &str,
        port: &str,
        make_default: bool,
    ) -> Result<(), ServerSettingsError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(ServerSettingsError::EmptyName);
        }

        if self.servers.iter().any(|server| server.name == name) {
            return Err(ServerSettingsError::DuplicateName { name: name.into() });
        }

        let ip = ip.trim();
        let ip = ip.parse().map_err(|source| InvalidIpAddress {
            value: ip.into(),
            source,
        })?;

        let port = port
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or_else(|| ServerSettingsError::InvalidPort { value: port.into() })?;

        let is_default = self.servers.is_empty() || make_default;
        let mut servers = self.servers.clone();
        if is_default {
            servers.iter_mut().for_each(|s| s.is_default = false);
        }

        servers.push(Server {
            name: name.into(),
            ip: Ok(ip),
            port,
            is_default,
        });

        self.save(servers)
    }

    pub fn set_default(&mut self, index: usize) -> Result<(), ServerSettingsError> {
        let mut servers = self.servers.clone();
        servers.iter_mut().for_each(|s| s.is_default = false);
        servers[index].is_default = true;
        self.save(servers)
    }

    fn save(&mut self, servers: Vec<Server>) -> Result<(), ServerSettingsError> {
        let records: Vec<_> = servers
            .iter()
            .map(|server| {
                (
                    server.name.as_str(),
                    match &server.ip {
                        Ok(ip) => ip.to_string(),
                        Err(error) => error.value.clone(),
                    },
                    server.port,
                    server.is_default,
                )
            })
            .collect();

        self.settings
            .set("servers", records)
            .inspect_err(|error| tracing::error!(%error, "Could not save server settings"))?;
        self.servers = servers;

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn settings() -> gio::Settings {
        let source =
            gio::SettingsSchemaSource::from_directory(env!("OUT_DIR"), None, false).unwrap();
        let schema = source.lookup(APPLICATION_ID, false).unwrap();
        gio::Settings::new_full(&schema, Some(&gio::memory_settings_backend_new()), None)
    }

    #[test]
    fn unset_language_follows_the_system() {
        let settings = settings();
        assert!(settings.user_value("language").is_none());
        assert_eq!(ServerSettings::load(settings).language(), "");
    }

    #[test]
    fn language_selection_survives_reopening_settings() {
        let settings = settings();
        let config = ServerSettings::load(settings.clone());
        config.set_language("zh_CN").unwrap();
        assert_eq!(ServerSettings::load(settings.clone()).language(), "zh_CN");
        config.set_language("").unwrap();
        assert_eq!(ServerSettings::load(settings).language(), "");
    }

    #[test]
    fn new_installation_has_no_server() {
        let config = ServerSettings::load(settings());
        assert!(config.servers().is_empty());
        assert_eq!(config.default_server_index(), None);
    }

    #[test]
    fn first_server_becomes_default_without_requesting_it() {
        let settings = settings();
        let mut config = ServerSettings::load(settings.clone());
        config
            .add_server("Home", "192.168.1.2", "2023", false)
            .unwrap();

        let reopened = ServerSettings::load(settings);
        assert_eq!(
            reopened.servers(),
            &[Server {
                name: "Home".into(),
                ip: Ok("192.168.1.2".parse().unwrap()),
                port: 2023,
                is_default: true,
            }]
        );
    }

    #[test]
    fn adding_another_server_preserves_existing_default() {
        let mut config = ServerSettings::load(settings());
        config
            .add_server("Home", "192.168.1.2", "2023", false)
            .unwrap();
        config
            .add_server("Office", "10.0.0.2", "8080", false)
            .unwrap();

        assert_eq!(config.default_server_index(), Some(0));
        assert!(!config.servers()[1].is_default);
    }

    #[test]
    fn adding_an_explicit_default_replaces_previous_default() {
        let mut config = ServerSettings::load(settings());
        config
            .add_server("Home", "192.168.1.2", "2023", false)
            .unwrap();
        config
            .add_server("Office", "10.0.0.2", "8080", true)
            .unwrap();

        assert_eq!(config.default_server_index(), Some(1));
        assert!(!config.servers()[0].is_default);
    }

    #[test]
    fn changing_default_survives_reopening_settings() {
        let settings = settings();
        let mut config = ServerSettings::load(settings.clone());
        config
            .add_server("Home", "192.168.1.2", "2023", false)
            .unwrap();
        config
            .add_server("Office", "10.0.0.2", "8080", false)
            .unwrap();
        config.set_default(1).unwrap();

        let reopened = ServerSettings::load(settings);
        assert_eq!(reopened.default_server_index(), Some(1));
        assert!(!reopened.servers()[0].is_default);
    }

    #[test]
    fn duplicate_trimmed_name_is_rejected_without_changing_saved_servers() {
        let settings = settings();
        let mut config = ServerSettings::load(settings.clone());
        config
            .add_server("Home", "192.168.1.2", "2023", false)
            .unwrap();
        let before = config.servers().to_vec();

        assert!(
            matches!(config.add_server(" Home ", "10.0.0.2", "8080", true), Err(ServerSettingsError::DuplicateName { name }) if name == "Home")
        );
        assert_eq!(ServerSettings::load(settings).servers(), before);
    }

    #[test]
    fn empty_name_is_rejected() {
        let mut config = ServerSettings::load(settings());
        assert!(matches!(
            config.add_server("  ", "127.0.0.1", "2023", false),
            Err(ServerSettingsError::EmptyName)
        ));
        assert!(config.servers().is_empty());
    }

    #[test]
    fn non_ip_address_is_rejected() {
        let mut config = ServerSettings::load(settings());
        assert!(
            matches!(config.add_server("Home", "https://example.com", "2023", false), Err(ServerSettingsError::InvalidIp(InvalidIpAddress { value, .. })) if value == "https://example.com")
        );
        assert!(config.servers().is_empty());
    }

    #[test]
    fn zero_port_is_rejected() {
        let mut config = ServerSettings::load(settings());
        assert!(
            matches!(config.add_server("Home", "127.0.0.1", "0", false), Err(ServerSettingsError::InvalidPort { value }) if value == "0")
        );
    }

    #[test]
    fn out_of_range_port_is_rejected() {
        let mut config = ServerSettings::load(settings());
        assert!(
            matches!(config.add_server("Home", "127.0.0.1", "65536", false), Err(ServerSettingsError::InvalidPort { value }) if value == "65536")
        );
    }

    #[test]
    fn ipv6_address_can_be_saved() {
        let mut config = ServerSettings::load(settings());
        config
            .add_server(" IPv6 ", " ::1 ", " 65535 ", false)
            .unwrap();
        let server = &config.servers()[config.default_server_index().unwrap()];
        assert_eq!(server.name, "IPv6");
        assert_eq!(server.ip, Ok("::1".parse::<IpAddr>().unwrap()));
        assert_eq!(server.port, 65535);
    }

    #[test]
    fn invalid_ip_keeps_its_server_record_and_does_not_stop_loading() {
        let settings = settings();
        settings
            .set(
                "servers",
                vec![
                    ("Broken", "not-an-ip", 2023u16, true),
                    ("Home", "127.0.0.1", 8080u16, false),
                ],
            )
            .unwrap();
        let config = ServerSettings::load(settings);

        assert_eq!(config.servers().len(), 2);
        let broken = &config.servers()[0];
        assert_eq!(broken.name, "Broken");
        assert_eq!(broken.port, 2023);
        assert!(broken.is_default);
        assert_eq!(broken.ip.as_ref().unwrap_err().value, "not-an-ip");
        assert_eq!(config.servers()[1].name, "Home");
        assert_eq!(config.servers()[1].ip, Ok("127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn adding_a_server_preserves_unparseable_saved_addresses() {
        let settings = settings();
        settings
            .set("servers", vec![("Broken", "not-an-ip", 2023u16, true)])
            .unwrap();
        let mut config = ServerSettings::load(settings.clone());
        config
            .add_server("Home", "127.0.0.1", "8080", false)
            .unwrap();

        assert_eq!(
            settings.get::<Vec<(String, String, u16, bool)>>("servers"),
            vec![
                ("Broken".into(), "not-an-ip".into(), 2023, true),
                ("Home".into(), "127.0.0.1".into(), 8080, false),
            ]
        );
    }

    #[test]
    fn changing_default_preserves_unparseable_saved_addresses() {
        let settings = settings();
        settings
            .set(
                "servers",
                vec![
                    ("Broken", "not-an-ip", 2023u16, true),
                    ("Home", "127.0.0.1", 8080u16, false),
                ],
            )
            .unwrap();
        let mut config = ServerSettings::load(settings.clone());
        config.set_default(1).unwrap();

        assert_eq!(
            settings.get::<Vec<(String, String, u16, bool)>>("servers"),
            vec![
                ("Broken".into(), "not-an-ip".into(), 2023, false),
                ("Home".into(), "127.0.0.1".into(), 8080, true),
            ]
        );
    }

    #[test]
    fn failed_settings_write_leaves_the_server_list_unchanged() {
        let source =
            gio::SettingsSchemaSource::from_directory(env!("OUT_DIR"), None, false).unwrap();
        let schema = source.lookup(APPLICATION_ID, false).unwrap();
        let settings =
            gio::Settings::new_full(&schema, Some(&gio::null_settings_backend_new()), None);
        let mut config = ServerSettings::load(settings);

        assert!(matches!(
            config.add_server("Home", "127.0.0.1", "2023", false),
            Err(ServerSettingsError::Save(_))
        ));
        assert!(config.servers().is_empty());
    }

    #[test]
    fn servers_survive_recreating_the_settings_backend_from_disk() {
        let path = std::env::temp_dir().join(format!("waddle-settings-{}.ini", std::process::id()));
        let source =
            gio::SettingsSchemaSource::from_directory(env!("OUT_DIR"), None, false).unwrap();
        let schema = source.lookup(APPLICATION_ID, false).unwrap();
        let backend = gio::keyfile_settings_backend_new(path.to_str().unwrap(), "/", None);
        let mut config =
            ServerSettings::load(gio::Settings::new_full(&schema, Some(&backend), None));
        config
            .add_server("Home", "192.168.1.2", "2023", false)
            .unwrap();
        config.add_server("Office", "::1", "8080", true).unwrap();
        gio::Settings::sync();
        drop(config);
        drop(backend);

        let backend = gio::keyfile_settings_backend_new(path.to_str().unwrap(), "/", None);
        let reopened = ServerSettings::load(gio::Settings::new_full(&schema, Some(&backend), None));
        assert_eq!(reopened.servers().len(), 2);
        assert_eq!(
            &reopened.servers()[reopened.default_server_index().unwrap()],
            &Server {
                name: "Office".into(),
                ip: Ok("::1".parse().unwrap()),
                port: 8080,
                is_default: true,
            }
        );
        std::fs::remove_file(path).unwrap();
    }
}
