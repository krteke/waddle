use super::{
    plugin::PluginFields,
    tls::TlsFields,
    transport::{TransportFields, TrojanTransportFields},
    widgets::{Choice, Widgets},
    *,
};
use crate::node::*;

pub enum ProtocolFields {
    Vmess(VmessFields),
    Vless(VlessFields),
    Shadowsocks(ShadowsocksFields),
    ShadowsocksR(ShadowsocksRFields),
    Trojan(TrojanFields),
    TrojanGo(TrojanGoFields),
    Tuic(TuicFields),
    Juicity(JuicityFields),
    Hysteria2(Hysteria2Fields),
    AnyTls(AnyTlsFields),
    Http(HttpFields),
    Https(HttpsFields),
    Socks5(Socks5Fields),
}

pub struct VmessFields {
    uuid: adw::EntryRow,
    transport: TransportFields,
    tls: TlsFields,
    insecure: adw::SwitchRow,
}

pub struct VlessFields {
    uuid: adw::EntryRow,
    flow: Choice,
    transport: TransportFields,
    tls: TlsFields,
}

pub struct ShadowsocksFields {
    password: adw::EntryRow,
    method: Choice,
    plugin: PluginFields,
}

pub struct ShadowsocksRFields {
    password: adw::EntryRow,
    method: Choice,
    protocol: Choice,
    protocol_param: adw::EntryRow,
    obfs: Choice,
    obfs_param: adw::EntryRow,
}

pub struct TrojanFields {
    password: adw::EntryRow,
    sni: adw::EntryRow,
    insecure: adw::SwitchRow,
}

pub struct TrojanGoFields {
    password: adw::EntryRow,
    transport: TrojanTransportFields,
    encryption: EncryptionFields,
    sni: adw::EntryRow,
    insecure: adw::SwitchRow,
}

pub struct TuicFields {
    uuid: adw::EntryRow,
    password: adw::EntryRow,
    sni: adw::EntryRow,
    insecure: adw::SwitchRow,
    alpn: adw::EntryRow,
    disable_sni: adw::SwitchRow,
    congestion_control: Choice,
    udp_relay_mode: Choice,
}

pub struct JuicityFields {
    uuid: adw::EntryRow,
    password: adw::EntryRow,
    sni: adw::EntryRow,
    insecure: adw::SwitchRow,
    pinned_certchain_sha256: adw::EntryRow,
    congestion_control: Choice,
}

pub struct Hysteria2Fields {
    auth: adw::EntryRow,
    sni: adw::EntryRow,
    insecure: adw::SwitchRow,
    pin_sha256: adw::EntryRow,
    obfs: Choice,
    obfs_password: adw::EntryRow,
    obfs_password_group: adw::PreferencesGroup,
}

pub struct AnyTlsFields {
    auth: adw::EntryRow,
    sni: adw::EntryRow,
    insecure: adw::SwitchRow,
}

pub struct HttpFields {
    username: adw::EntryRow,
    password: adw::EntryRow,
}

pub struct HttpsFields {
    username: adw::EntryRow,
    password: adw::EntryRow,
    sni: adw::EntryRow,
    insecure: adw::SwitchRow,
}

pub struct Socks5Fields {
    username: adw::EntryRow,
    password: adw::EntryRow,
}

impl ProtocolFields {
    pub fn new(value: &Protocol, w: &mut Widgets) -> Self {
        match value {
            Protocol::Vmess(node) => {
                let authentication = w.group("authentication");

                w.description(&authentication, "node_editor.vmess_hint");
                let uuid = w.entry(&authentication, "node_editor.uuid", &node.uuid, true);

                let transport = TransportFields::new(&node.transport, w);
                let tls = TlsFields::new(&node.tls, w);

                let options = w.group("");
                let insecure = w.switch(&options, "node_editor.insecure", node.insecure);

                Self::Vmess(VmessFields {
                    uuid,
                    transport,
                    tls,
                    insecure,
                })
            }
            Protocol::Vless(node) => {
                let authentication = w.group("authentication");

                let uuid = w.entry(&authentication, "node_editor.uuid", &node.uuid, true);

                let flow = w.choice(
                    &authentication,
                    "node_editor.flow",
                    &["", "xtls-rprx-vision", "xtls-rprx-vision-udp443"],
                    &node.flow,
                );

                let transport = TransportFields::new(&node.transport, w);
                let tls = TlsFields::new(&node.tls, w);

                Self::Vless(VlessFields {
                    uuid,
                    flow,
                    transport,
                    tls,
                })
            }
            Protocol::Shadowsocks(node) => {
                let authentication = w.group("authentication");

                let password = w.password(
                    &authentication,
                    "node_editor.password",
                    &node.password,
                    true,
                );

                let method = w.choice(
                    &authentication,
                    "node_editor.cipher",
                    Shadowsocks::CIPHERS,
                    &node.method,
                );

                let plugin = PluginFields::new(&node.plugin, w);

                Self::Shadowsocks(ShadowsocksFields {
                    password,
                    method,
                    plugin,
                })
            }
            Protocol::ShadowsocksR(node) => {
                let authentication = w.group("authentication");

                let password = w.password(
                    &authentication,
                    "node_editor.password",
                    &node.password,
                    true,
                );

                let method = w.choice(
                    &authentication,
                    "node_editor.cipher",
                    ShadowsocksR::CIPHERS,
                    &node.method,
                );

                let obfuscation = w.group("obfuscation");

                let protocol = w.choice(
                    &obfuscation,
                    "node_editor.ssr_protocol",
                    &[
                        "origin",
                        "verify_sha1",
                        "auth_sha1_v4",
                        "auth_aes128_md5",
                        "auth_aes128_sha1",
                        "auth_chain_a",
                        "auth_chain_b",
                    ],
                    &node.protocol,
                );

                let protocol_param = w.entry(
                    &obfuscation,
                    "node_editor.protocol_parameter",
                    &node.protocol_param,
                    false,
                );

                let obfs = w.choice(
                    &obfuscation,
                    "node_editor.obfs",
                    &[
                        "plain",
                        "http_simple",
                        "http_post",
                        "random_head",
                        "tls1.2_ticket_auth",
                    ],
                    &node.obfs,
                );

                let obfs_param = w.entry(
                    &obfuscation,
                    "node_editor.obfs_parameter",
                    &node.obfs_param,
                    false,
                );

                Self::ShadowsocksR(ShadowsocksRFields {
                    password,
                    method,
                    protocol,
                    protocol_param,
                    obfs,
                    obfs_param,
                })
            }
            Protocol::Trojan(node) => {
                let authentication = w.group("authentication");

                let password = w.password(
                    &authentication,
                    "node_editor.password",
                    &node.password,
                    true,
                );

                let tls = w.group("tls");
                let sni = w.entry(&tls, "node_editor.sni", &node.sni, false);
                let insecure = w.switch(&tls, "node_editor.insecure", node.insecure);

                Self::Trojan(TrojanFields {
                    password,
                    sni,
                    insecure,
                })
            }
            Protocol::TrojanGo(node) => {
                let authentication = w.group("authentication");

                let password = w.password(
                    &authentication,
                    "node_editor.password",
                    &node.password,
                    true,
                );

                let transport = TrojanTransportFields::new(&node.transport, w);
                let encryption = EncryptionFields::new(&node.encryption, w);

                let tls = w.group("tls");
                let sni = w.entry(&tls, "node_editor.sni", &node.sni, false);
                let insecure = w.switch(&tls, "node_editor.insecure", node.insecure);

                Self::TrojanGo(TrojanGoFields {
                    password,
                    transport,
                    encryption,
                    sni,
                    insecure,
                })
            }
            Protocol::Tuic(node) => {
                let authentication = w.group("authentication");

                let uuid = w.entry(&authentication, "node_editor.uuid", &node.uuid, true);

                let password = w.password(
                    &authentication,
                    "node_editor.password",
                    &node.password,
                    true,
                );

                let tls = w.group("tls");

                let sni = w.entry(&tls, "node_editor.sni", &node.sni, false);
                let insecure = w.switch(&tls, "node_editor.insecure", node.insecure);
                let alpn = w.entry(&tls, "node_editor.alpn", &node.alpn, false);
                let disable_sni = w.switch(&tls, "node_editor.disable_sni", node.disable_sni);

                let quic = w.group("quic");

                let congestion_control = w.choice(
                    &quic,
                    "node_editor.congestion_control",
                    &["", "bbr", "cubic"],
                    &node.congestion_control,
                );

                let udp_relay_mode = w.choice(
                    &quic,
                    "node_editor.udp_relay_mode",
                    &["", "native", "quic"],
                    &node.udp_relay_mode,
                );

                Self::Tuic(TuicFields {
                    uuid,
                    password,
                    sni,
                    insecure,
                    alpn,
                    disable_sni,
                    congestion_control,
                    udp_relay_mode,
                })
            }
            Protocol::Juicity(node) => {
                let authentication = w.group("authentication");

                let uuid = w.entry(&authentication, "node_editor.uuid", &node.uuid, true);

                let password = w.password(
                    &authentication,
                    "node_editor.password",
                    &node.password,
                    true,
                );

                let tls = w.group("tls");
                let sni = w.entry(&tls, "node_editor.sni", &node.sni, false);
                let insecure = w.switch(&tls, "node_editor.insecure", node.insecure);

                let pinned_certchain_sha256 = w.entry(
                    &tls,
                    "node_editor.certificate_hash",
                    &node.pinned_certchain_sha256,
                    false,
                );

                let quic = w.group("quic");

                let congestion_control = w.choice(
                    &quic,
                    "node_editor.congestion_control",
                    &["", "bbr", "cubic"],
                    &node.congestion_control,
                );

                Self::Juicity(JuicityFields {
                    uuid,
                    password,
                    sni,
                    insecure,
                    pinned_certchain_sha256,
                    congestion_control,
                })
            }
            Protocol::Hysteria2(node) => {
                let authentication = w.group("authentication");

                let auth = w.password(&authentication, "node_editor.password", &node.auth, false);
                let tls = w.group("tls");

                let sni = w.entry(&tls, "node_editor.sni", &node.sni, false);
                let insecure = w.switch(&tls, "node_editor.insecure", node.insecure);

                let pin_sha256 = w.entry(
                    &tls,
                    "node_editor.certificate_hash",
                    &node.pin_sha256,
                    false,
                );

                let obfuscation = w.group("obfuscation");

                let obfs = w.choice(
                    &obfuscation,
                    "node_editor.obfs",
                    &["", "salamander", "gecko"],
                    &node.obfs,
                );

                let obfs_password_group = w.group("");

                let obfs_password = w.password(
                    &obfs_password_group,
                    "node_editor.obfs_password",
                    &node.obfs_password,
                    true,
                );

                Self::Hysteria2(Hysteria2Fields {
                    auth,
                    sni,
                    insecure,
                    pin_sha256,
                    obfs,
                    obfs_password,
                    obfs_password_group,
                })
            }
            Protocol::AnyTls(node) => {
                let authentication = w.group("authentication");

                let auth = w.password(&authentication, "node_editor.password", &node.auth, false);

                let tls = w.group("tls");
                let sni = w.entry(&tls, "node_editor.sni", &node.sni, false);
                let insecure = w.switch(&tls, "node_editor.insecure", node.insecure);

                Self::AnyTls(AnyTlsFields {
                    auth,
                    sni,
                    insecure,
                })
            }
            Protocol::Http(node) => {
                let authentication = w.group("authentication");

                let username = w.entry(
                    &authentication,
                    "node_editor.username",
                    &node.username,
                    false,
                );

                let password = w.password(
                    &authentication,
                    "node_editor.password",
                    &node.password,
                    false,
                );

                Self::Http(HttpFields { username, password })
            }
            Protocol::Https(node) => {
                let authentication = w.group("authentication");

                let username = w.entry(
                    &authentication,
                    "node_editor.username",
                    &node.username,
                    false,
                );

                let password = w.password(
                    &authentication,
                    "node_editor.password",
                    &node.password,
                    false,
                );

                let tls = w.group("tls");
                let sni = w.entry(&tls, "node_editor.sni", &node.sni, false);

                let insecure = w.switch(&tls, "node_editor.insecure", node.insecure);

                Self::Https(HttpsFields {
                    username,
                    password,
                    sni,
                    insecure,
                })
            }
            Protocol::Socks5(node) => {
                let authentication = w.group("authentication");

                let username = w.entry(
                    &authentication,
                    "node_editor.username",
                    &node.username,
                    false,
                );

                let password = w.password(
                    &authentication,
                    "node_editor.password",
                    &node.password,
                    false,
                );

                Self::Socks5(Socks5Fields { username, password })
            }
        }
    }

    pub fn read(&self) -> Protocol {
        match self {
            Self::Vmess(f) => Vmess {
                uuid: f.uuid.text().into(),
                transport: f.transport.read(),
                tls: f.tls.read(),
                insecure: f.insecure.is_active() && f.tls.enabled(),
            }
            .into(),

            Self::Vless(f) => Vless {
                uuid: f.uuid.text().into(),
                flow: f.flow.text(),
                transport: f.transport.read(),
                tls: f.tls.read(),
            }
            .into(),

            Self::Shadowsocks(f) => Shadowsocks {
                password: f.password.text().into(),
                method: f.method.text(),
                plugin: f.plugin.read(),
            }
            .into(),

            Self::ShadowsocksR(f) => ShadowsocksR {
                password: f.password.text().into(),
                method: f.method.text(),
                protocol: f.protocol.text(),
                protocol_param: f.protocol_param.text().into(),
                obfs: f.obfs.text(),
                obfs_param: f.obfs_param.text().into(),
            }
            .into(),

            Self::Trojan(f) => Trojan {
                password: f.password.text().into(),
                sni: f.sni.text().into(),
                insecure: f.insecure.is_active(),
            }
            .into(),

            Self::TrojanGo(f) => TrojanGo {
                password: f.password.text().into(),
                transport: f.transport.read(),
                encryption: f.encryption.read(),
                sni: f.sni.text().into(),
                insecure: f.insecure.is_active(),
            }
            .into(),

            Self::Tuic(f) => Tuic {
                uuid: f.uuid.text().into(),
                password: f.password.text().into(),
                sni: f.sni.text().into(),
                insecure: f.insecure.is_active(),
                alpn: f.alpn.text().into(),
                disable_sni: f.disable_sni.is_active(),
                congestion_control: f.congestion_control.text(),
                udp_relay_mode: f.udp_relay_mode.text(),
            }
            .into(),

            Self::Juicity(f) => Juicity {
                uuid: f.uuid.text().into(),
                password: f.password.text().into(),
                sni: f.sni.text().into(),
                insecure: f.insecure.is_active(),
                pinned_certchain_sha256: f.pinned_certchain_sha256.text().into(),
                congestion_control: f.congestion_control.text(),
            }
            .into(),

            Self::Hysteria2(f) => Hysteria2 {
                auth: f.auth.text().into(),
                sni: f.sni.text().into(),
                insecure: f.insecure.is_active(),
                pin_sha256: f.pin_sha256.text().into(),
                obfs: f.obfs.text(),
                obfs_password: f.obfs_password.text().into(),
            }
            .into(),

            Self::AnyTls(f) => AnyTls {
                auth: f.auth.text().into(),
                sni: f.sni.text().into(),
                insecure: f.insecure.is_active(),
            }
            .into(),

            Self::Http(f) => Http {
                username: f.username.text().into(),
                password: f.password.text().into(),
            }
            .into(),

            Self::Https(f) => Https {
                username: f.username.text().into(),
                password: f.password.text().into(),
                sni: f.sni.text().into(),
                insecure: f.insecure.is_active(),
            }
            .into(),

            Self::Socks5(f) => Socks5 {
                username: f.username.text().into(),
                password: f.password.text().into(),
            }
            .into(),
        }
    }

    pub fn update_visibility(&self) {
        match self {
            Self::Vmess(f) => {
                f.transport.update_visibility();
                f.tls.update_visibility();
                f.insecure.set_visible(f.tls.enabled());
            }
            Self::Vless(f) => {
                f.transport.update_visibility();
                f.tls.update_visibility();
            }
            Self::Shadowsocks(f) => {
                f.plugin.update_visibility();
            }
            Self::TrojanGo(f) => {
                f.transport.update_visibility();
                f.encryption.update_visibility();
            }
            Self::Hysteria2(f) => {
                f.obfs_password_group.set_visible(f.obfs.selected() == 1);
            }
            _ => {}
        }
    }
}

struct EncryptionFields {
    cipher: Choice,
    password_group: adw::PreferencesGroup,
    password: adw::EntryRow,
}
impl EncryptionFields {
    fn new(value: &TrojanEncryption, w: &mut Widgets) -> Self {
        let group = w.group("encryption");
        let (cipher, password) = match value {
            TrojanEncryption::None => ("", ""),
            TrojanEncryption::Shadowsocks { cipher, password } => {
                (cipher.as_str(), password.as_str())
            }
        };
        let cipher = w.choice(
            &group,
            "node_editor.cipher",
            &[
                "",
                "aes-128-gcm",
                "aes-256-gcm",
                "chacha20-poly1305",
                "chacha20-ietf-poly1305",
            ],
            cipher,
        );
        let password_group = w.group("");
        let password = w.password(&password_group, "node_editor.ss_password", password, true);
        Self {
            cipher,
            password_group,
            password,
        }
    }
    fn read(&self) -> TrojanEncryption {
        if self.cipher.selected() == 0 {
            TrojanEncryption::None
        } else {
            TrojanEncryption::Shadowsocks {
                cipher: self.cipher.text(),
                password: self.password.text().into(),
            }
        }
    }
    fn update_visibility(&self) {
        self.password_group.set_visible(self.cipher.selected() != 0);
    }
}
