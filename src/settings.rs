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
    pub logged_out: bool,
}

pub struct ServerSettings {
    settings: gio::Settings,
    servers: Vec<Server>,
}

impl ServerSettings {
    pub fn load(settings: gio::Settings) -> Self {
        let records = settings.get::<Vec<(String, String, u16, bool, bool)>>("servers");

        let servers = records
            .into_iter()
            .map(|(name, ip, port, is_default, logged_out)| {
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
                    logged_out,
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

    pub fn save_server(
        &mut self,
        index: Option<usize>,
        name: &str,
        ip: &str,
        port: &str,
        make_default: bool,
    ) -> Result<(), ServerSettingsError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(ServerSettingsError::EmptyName);
        }

        if self
            .servers
            .iter()
            .enumerate()
            .any(|(other_index, server)| Some(other_index) != index && server.name == name)
        {
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

        let is_default = self.servers.is_empty()
            || make_default
            || index.is_some_and(|index| self.servers[index].is_default);
        let mut servers = self.servers.clone();
        if is_default {
            servers.iter_mut().for_each(|s| s.is_default = false);
        }

        let server = Server {
            name: name.into(),
            ip: Ok(ip),
            port,
            is_default,
            logged_out: index.is_some_and(|index| self.servers[index].logged_out),
        };

        match index {
            Some(index) => servers[index] = server,
            None => servers.push(server),
        }

        self.save(servers)
    }

    pub fn set_default(&mut self, index: usize) -> Result<(), ServerSettingsError> {
        let mut servers = self.servers.clone();
        servers.iter_mut().for_each(|s| s.is_default = false);
        servers[index].is_default = true;
        self.save(servers)
    }

    pub fn delete_server(&mut self, index: usize) -> Result<(), ServerSettingsError> {
        let mut servers = self.servers.clone();
        let removed = servers.remove(index);

        if removed.is_default
            && let Some(server) = servers.first_mut()
        {
            server.is_default = true;
        }

        self.save(servers)
    }

    pub fn login(&mut self, index: usize) -> Result<(), ServerSettingsError> {
        self.set_logged_out(index, false)
    }

    pub fn logout(&mut self, index: usize) -> Result<(), ServerSettingsError> {
        self.set_logged_out(index, true)
    }

    fn set_logged_out(
        &mut self,
        index: usize,
        logged_out: bool,
    ) -> Result<(), ServerSettingsError> {
        let mut servers = self.servers.clone();
        servers[index].logged_out = logged_out;
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
                    server.logged_out,
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
    fn renaming_server_preserves_manual_sign_out() {
        let backend = settings();
        let mut config = ServerSettings::load(backend.clone());
        config
            .save_server(None, "Home", "127.0.0.1", "2023", false)
            .unwrap();
        config.set_logged_out(0, true).unwrap();
        config
            .save_server(Some(0), "Renamed", "::1", "8080", false)
            .unwrap();
        assert!(ServerSettings::load(backend).servers()[0].logged_out);
    }

    #[test]
    fn manual_sign_out_is_persistent_and_specific_to_a_server() {
        let backend = settings();
        let mut config = ServerSettings::load(backend.clone());
        config
            .save_server(None, "Home", "127.0.0.1", "2023", false)
            .unwrap();
        config
            .save_server(None, "Office", "::1", "8080", false)
            .unwrap();
        assert!(!config.servers()[0].logged_out);
        config.set_logged_out(0, true).unwrap();
        let mut reopened = ServerSettings::load(backend.clone());
        assert!(reopened.servers()[0].logged_out);
        assert!(!reopened.servers()[1].logged_out);
        reopened.set_logged_out(1, true).unwrap();
        reopened.set_logged_out(0, false).unwrap();
        let reopened = ServerSettings::load(backend);
        assert!(!reopened.servers()[0].logged_out);
        assert!(reopened.servers()[1].logged_out);
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
    fn deleting_non_default_server_preserves_remaining_records() {
        let backend = settings();
        backend
            .set(
                "servers",
                vec![
                    ("Remove", "127.0.0.1", 2023u16, false, true),
                    ("Keep", "::1", 8080u16, true, true),
                ],
            )
            .unwrap();
        let mut config = ServerSettings::load(backend.clone());
        let remaining = config.servers()[1].clone();
        config.delete_server(0).unwrap();
        assert_eq!(config.servers(), std::slice::from_ref(&remaining));
        assert_eq!(ServerSettings::load(backend).servers(), &[remaining]);
    }

    #[test]
    fn deleting_default_server_promotes_first_remaining_server() {
        let backend = settings();
        backend
            .set(
                "servers",
                vec![
                    ("Keep", "bad-ip", 2023u16, false, true),
                    ("Remove", "::1", 8080u16, true, false),
                ],
            )
            .unwrap();
        let mut config = ServerSettings::load(backend.clone());
        let mut remaining = config.servers()[0].clone();
        remaining.is_default = true;
        config.delete_server(1).unwrap();
        assert_eq!(ServerSettings::load(backend).servers(), &[remaining]);
    }

    #[test]
    fn deleting_last_server_leaves_no_default() {
        let backend = settings();
        backend
            .set("servers", vec![("Last", "bad-ip", 2023u16, true, true)])
            .unwrap();
        let mut config = ServerSettings::load(backend.clone());
        config.delete_server(0).unwrap();
        let reopened = ServerSettings::load(backend);
        assert!(reopened.servers().is_empty());
        assert_eq!(reopened.default_server_index(), None);
    }

    #[test]
    fn failed_deletion_keeps_the_server_record() {
        let source =
            gio::SettingsSchemaSource::from_directory(env!("OUT_DIR"), None, false).unwrap();
        let schema = source.lookup(APPLICATION_ID, false).unwrap();
        let backend =
            gio::Settings::new_full(&schema, Some(&gio::null_settings_backend_new()), None);
        let mut config = ServerSettings {
            settings: backend,
            servers: vec![Server {
                name: "Keep".into(),
                ip: Ok("127.0.0.1".parse().unwrap()),
                port: 2023,
                is_default: true,
                logged_out: true,
            }],
        };
        let before = config.servers().to_vec();
        assert!(matches!(
            config.delete_server(0),
            Err(ServerSettingsError::Save(_))
        ));
        assert_eq!(config.servers(), before);
    }

    #[test]
    fn new_installation_has_no_server() {
        let config = ServerSettings::load(settings());
        assert!(config.servers().is_empty());
        assert_eq!(config.default_server_index(), None);
    }

    #[test]
    fn editing_repairs_an_address_and_preserves_other_servers() {
        let settings = settings();
        settings
            .set(
                "servers",
                vec![
                    ("Broken", "bad-ip", 2023u16, true, false),
                    ("Office", "::1", 8080u16, false, false),
                ],
            )
            .unwrap();
        let mut config = ServerSettings::load(settings.clone());

        config
            .save_server(Some(0), " Repaired ", " 127.0.0.1 ", " 9090 ", false)
            .unwrap();

        assert_eq!(
            ServerSettings::load(settings).servers(),
            &[
                Server {
                    name: "Repaired".into(),
                    ip: Ok("127.0.0.1".parse().unwrap()),
                    port: 9090,
                    is_default: true,
                    logged_out: false,
                },
                Server {
                    name: "Office".into(),
                    ip: Ok("::1".parse().unwrap()),
                    port: 8080,
                    is_default: false,
                    logged_out: false,
                },
            ]
        );
    }

    #[test]
    fn editing_keeps_its_name_and_can_become_default() {
        let settings = settings();
        settings
            .set(
                "servers",
                vec![
                    ("Home", "127.0.0.1", 2023u16, true, false),
                    ("Office", "::1", 8080u16, false, false),
                ],
            )
            .unwrap();
        let mut config = ServerSettings::load(settings.clone());

        config
            .save_server(Some(1), "Office", "::1", "9090", true)
            .unwrap();

        assert_eq!(
            settings.get::<Vec<(String, String, u16, bool, bool)>>("servers"),
            vec![
                ("Home".into(), "127.0.0.1".into(), 2023, false, false),
                ("Office".into(), "::1".into(), 9090, true, false),
            ]
        );
    }

    #[test]
    fn editing_rejects_another_servers_name_without_changing_settings() {
        let settings = settings();
        settings
            .set(
                "servers",
                vec![
                    ("Home", "127.0.0.1", 2023u16, true, false),
                    ("Office", "::1", 8080u16, false, false),
                ],
            )
            .unwrap();
        let mut config = ServerSettings::load(settings.clone());
        let before = config.servers().to_vec();

        assert!(matches!(
            config.save_server(Some(1), " Home ", "::1", "9090", true),
            Err(ServerSettingsError::DuplicateName { .. })
        ));

        assert_eq!(config.servers(), before);
        assert_eq!(ServerSettings::load(settings).servers(), before);
    }

    #[test]
    fn editing_rejects_invalid_addresses_without_changing_settings() {
        let settings = settings();
        settings
            .set("servers", vec![("Home", "127.0.0.1", 2023u16, true, false)])
            .unwrap();
        let mut config = ServerSettings::load(settings.clone());
        let before = config.servers().to_vec();

        assert!(matches!(
            config.save_server(Some(0), "Renamed", "bad-ip", "9090", true),
            Err(ServerSettingsError::InvalidIp(_))
        ));

        assert_eq!(config.servers(), before);
        assert_eq!(ServerSettings::load(settings).servers(), before);
    }

    #[test]
    fn first_server_becomes_default_without_requesting_it() {
        let settings = settings();
        let mut config = ServerSettings::load(settings.clone());
        config
            .save_server(None, "Home", "192.168.1.2", "2023", false)
            .unwrap();

        let reopened = ServerSettings::load(settings);
        assert_eq!(
            reopened.servers(),
            &[Server {
                name: "Home".into(),
                ip: Ok("192.168.1.2".parse().unwrap()),
                port: 2023,
                is_default: true,
                logged_out: false,
            }]
        );
    }

    #[test]
    fn adding_another_server_preserves_existing_default() {
        let mut config = ServerSettings::load(settings());
        config
            .save_server(None, "Home", "192.168.1.2", "2023", false)
            .unwrap();
        config
            .save_server(None, "Office", "10.0.0.2", "8080", false)
            .unwrap();

        assert_eq!(config.default_server_index(), Some(0));
        assert!(!config.servers()[1].is_default);
    }

    #[test]
    fn adding_an_explicit_default_replaces_previous_default() {
        let mut config = ServerSettings::load(settings());
        config
            .save_server(None, "Home", "192.168.1.2", "2023", false)
            .unwrap();
        config
            .save_server(None, "Office", "10.0.0.2", "8080", true)
            .unwrap();

        assert_eq!(config.default_server_index(), Some(1));
        assert!(!config.servers()[0].is_default);
    }

    #[test]
    fn changing_default_survives_reopening_settings() {
        let settings = settings();
        let mut config = ServerSettings::load(settings.clone());
        config
            .save_server(None, "Home", "192.168.1.2", "2023", false)
            .unwrap();
        config
            .save_server(None, "Office", "10.0.0.2", "8080", false)
            .unwrap();
        config.set_logged_out(0, true).unwrap();
        config.set_default(1).unwrap();

        let reopened = ServerSettings::load(settings);
        assert_eq!(reopened.default_server_index(), Some(1));
        assert!(!reopened.servers()[0].is_default);
        assert!(reopened.servers()[0].logged_out);
    }

    #[test]
    fn duplicate_trimmed_name_is_rejected_without_changing_saved_servers() {
        let settings = settings();
        let mut config = ServerSettings::load(settings.clone());
        config
            .save_server(None, "Home", "192.168.1.2", "2023", false)
            .unwrap();
        let before = config.servers().to_vec();

        assert!(
            matches!(config.save_server(None, " Home ", "10.0.0.2", "8080", true), Err(ServerSettingsError::DuplicateName { name }) if name == "Home")
        );
        assert_eq!(ServerSettings::load(settings).servers(), before);
    }

    #[test]
    fn empty_name_is_rejected() {
        let mut config = ServerSettings::load(settings());
        assert!(matches!(
            config.save_server(None, "  ", "127.0.0.1", "2023", false),
            Err(ServerSettingsError::EmptyName)
        ));
        assert!(config.servers().is_empty());
    }

    #[test]
    fn non_ip_address_is_rejected() {
        let mut config = ServerSettings::load(settings());
        assert!(
            matches!(config.save_server(None, "Home", "https://example.com", "2023", false), Err(ServerSettingsError::InvalidIp(InvalidIpAddress { value, .. })) if value == "https://example.com")
        );
        assert!(config.servers().is_empty());
    }

    #[test]
    fn zero_port_is_rejected() {
        let mut config = ServerSettings::load(settings());
        assert!(
            matches!(config.save_server(None, "Home", "127.0.0.1", "0", false), Err(ServerSettingsError::InvalidPort { value }) if value == "0")
        );
    }

    #[test]
    fn out_of_range_port_is_rejected() {
        let mut config = ServerSettings::load(settings());
        assert!(
            matches!(config.save_server(None, "Home", "127.0.0.1", "65536", false), Err(ServerSettingsError::InvalidPort { value }) if value == "65536")
        );
    }

    #[test]
    fn ipv6_address_can_be_saved() {
        let mut config = ServerSettings::load(settings());
        config
            .save_server(None, " IPv6 ", " ::1 ", " 65535 ", false)
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
                    ("Broken", "not-an-ip", 2023u16, true, false),
                    ("Home", "127.0.0.1", 8080u16, false, false),
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
            .set(
                "servers",
                vec![("Broken", "not-an-ip", 2023u16, true, false)],
            )
            .unwrap();
        let mut config = ServerSettings::load(settings.clone());
        config
            .save_server(None, "Home", "127.0.0.1", "8080", false)
            .unwrap();

        assert_eq!(
            settings.get::<Vec<(String, String, u16, bool, bool)>>("servers"),
            vec![
                ("Broken".into(), "not-an-ip".into(), 2023, true, false),
                ("Home".into(), "127.0.0.1".into(), 8080, false, false),
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
                    ("Broken", "not-an-ip", 2023u16, true, false),
                    ("Home", "127.0.0.1", 8080u16, false, false),
                ],
            )
            .unwrap();
        let mut config = ServerSettings::load(settings.clone());
        config.set_default(1).unwrap();

        assert_eq!(
            settings.get::<Vec<(String, String, u16, bool, bool)>>("servers"),
            vec![
                ("Broken".into(), "not-an-ip".into(), 2023, false, false),
                ("Home".into(), "127.0.0.1".into(), 8080, true, false),
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
            config.save_server(None, "Home", "127.0.0.1", "2023", false),
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
            .save_server(None, "Home", "192.168.1.2", "2023", false)
            .unwrap();
        config
            .save_server(None, "Office", "::1", "8080", true)
            .unwrap();
        config.set_logged_out(0, true).unwrap();
        gio::Settings::sync();
        drop(config);
        drop(backend);

        let backend = gio::keyfile_settings_backend_new(path.to_str().unwrap(), "/", None);
        let reopened = ServerSettings::load(gio::Settings::new_full(&schema, Some(&backend), None));
        assert_eq!(reopened.servers().len(), 2);
        assert!(reopened.servers()[0].logged_out);
        assert_eq!(
            &reopened.servers()[reopened.default_server_index().unwrap()],
            &Server {
                name: "Office".into(),
                ip: Ok("::1".parse().unwrap()),
                port: 8080,
                is_default: true,
                logged_out: false,
            }
        );
        std::fs::remove_file(path).unwrap();
    }
}
