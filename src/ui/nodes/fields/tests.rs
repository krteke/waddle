use super::*;
use crate::node::{
    DecodeExt, Plugin, Protocol, Transport, TrojanEncryption, TrojanTransport, V2rayTls,
};

fn find<W: IsA<gtk::Widget>>(
    root: &impl IsA<gtk::Widget>,
    predicate: &impl Fn(&W) -> bool,
) -> Option<W> {
    let root = root.as_ref();
    if let Ok(widget) = root.clone().downcast::<W>()
        && predicate(&widget)
    {
        return Some(widget);
    }
    let mut child = root.first_child();
    while let Some(widget) = child {
        if let Some(found) = find(&widget, predicate) {
            return Some(found);
        }
        child = widget.next_sibling();
    }
    None
}

struct Form {
    fields: Fields,
    _receiver: relm4::Receiver<EditorMsg>,
}
impl Form {
    fn from_link(link: &str) -> Self {
        let (sender, receiver) = relm4::channel();
        Self {
            fields: Fields::new(link.decode::<Protocol>().unwrap(), "Test", &sender),
            _receiver: receiver,
        }
    }
    fn entry(&self, title: &str) -> adw::EntryRow {
        find(&self.fields.page, &|row: &adw::EntryRow| {
            row.title() == title && row.is_visible()
        })
        .unwrap_or_else(|| panic!("missing visible entry {title}"))
    }
    fn select(&self, title: &str, index: u32) {
        let row = find::<adw::ComboRow>(&self.fields.page, &|row| {
            row.title() == title && row.is_visible()
        })
        .unwrap();
        row.set_selected(index);
        self.fields.update_visibility();
    }
}

// Run on the same GTK thread as the app's integration scenario.
pub fn transport_switch_preserves_separate_drafts() {
    let form = Form::from_link(
        "vless://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:8443?type=ws&host=cdn.example.com&path=%2Fws#Original",
    );
    form.select("Transport type", 3);
    form.entry("gRPC service name").set_text("grpc-proxy");
    let Protocol::Vless(node) = form.fields.config().protocol else {
        panic!("expected VLESS")
    };
    assert_eq!(
        node.transport,
        Transport::Grpc {
            service_name: "grpc-proxy".into()
        }
    );
    form.select("Transport type", 1);
    assert_eq!(form.entry("HTTP Host").text(), "cdn.example.com");
    assert_eq!(form.entry("Path").text(), "/ws");
    form.select("Transport type", 3);
    assert_eq!(form.entry("gRPC service name").text(), "grpc-proxy");
    let actual = form.fields.config();
    assert_eq!(actual.name, "Original");
    assert_eq!(actual.server, "example.com");
    assert_eq!(actual.port, 8443);
    assert!(
        form.fields.validate(),
        "inactive Reality public key must not block gRPC submission"
    );
}

pub fn tls_switch_validates_only_active_branch() {
    let form = Form::from_link(
        "vless://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?security=tls&sni=tls.example.com&alpn=h2",
    );
    form.select("Security", 2);
    assert!(!form.fields.validate());
    assert!(form.entry("Reality public key").has_css_class("error"));
    form.entry("Reality public key")
        .set_text("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
    form.entry("Server name (SNI)")
        .set_text("reality.example.com");
    assert!(form.fields.validate());
    form.select("Security", 1);
    assert_eq!(form.entry("Server name (SNI)").text(), "tls.example.com");
    let Protocol::Vless(node) = form.fields.config().protocol else {
        panic!("expected VLESS")
    };
    assert_eq!(
        node.tls,
        V2rayTls::Tls {
            sni: "tls.example.com".into(),
            alpn: "h2".into()
        }
    );
    form.select("Security", 2);
    assert_eq!(
        form.entry("Server name (SNI)").text(),
        "reality.example.com"
    );
    form.select("Security", 0);
    assert!(form.fields.validate());
    let Protocol::Vless(node) = form.fields.config().protocol else {
        panic!("expected VLESS")
    };
    assert_eq!(node.tls, V2rayTls::None);
}

pub fn plugin_switch_constructs_only_selected_plugin() {
    let form = Form::from_link(
        "ss://YWVzLTEyOC1nY206cGFzcw@example.com:8388?plugin=v2ray-plugin%3Btls%3Bhost%3Dcdn.example.com",
    );
    form.select("Plugin", 3);
    assert!(!form.fields.validate());
    form.select("Plugin", 2);
    assert!(form.fields.validate());
    let Protocol::Shadowsocks(node) = form.fields.config().protocol else {
        panic!("expected Shadowsocks")
    };
    assert_eq!(
        node.plugin,
        Plugin::V2ray {
            tls: true,
            host: "cdn.example.com".into()
        }
    );
    form.select("Plugin", 0);
    let Protocol::Shadowsocks(node) = form.fields.config().protocol else {
        panic!("expected Shadowsocks")
    };
    assert_eq!(node.plugin, Plugin::None);
}

pub fn trojan_go_prefills_transport_and_encryption() {
    let form = Form::from_link(
        "trojan-go://password@example.com:443?type=grpc&serviceName=my-service&encryption=ss%3Baes-128-gcm%3Bss-password",
    );
    assert_eq!(form.entry("gRPC service name").text(), "my-service");
    assert_eq!(form.entry("Shadowsocks password").text(), "ss-password");
    assert!(form.fields.validate());
    let Protocol::TrojanGo(node) = form.fields.config().protocol else {
        panic!("expected Trojan-Go")
    };
    assert_eq!(
        node.transport,
        TrojanTransport::Grpc {
            service_name: "my-service".into()
        }
    );
    assert_eq!(
        node.encryption,
        TrojanEncryption::Shadowsocks {
            cipher: "aes-128-gcm".into(),
            password: "ss-password".into()
        }
    );
}

pub fn form_links_import_into_backend(api: &crate::api::Api, runtime: &tokio::runtime::Runtime) {
    // The batch itself is the result under test: every form-generated link must import.
    let vmess = Form::from_link(
        "vmess://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?type=grpc&serviceName=vmess-service&security=tls",
    );
    let vless = Form::from_link(
        "vless://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?type=tcp&security=reality&sni=www.example.com&pbk=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA&fp=chrome",
    );
    let trojan = Form::from_link(
        "trojan-go://password@example.com:443?type=ws&host=cdn.example.com&path=%2Fproxy&encryption=ss%3Baes-128-gcm%3Bss-password",
    );
    let imports = [vmess, vless, trojan]
        .into_iter()
        .map(|form| crate::api::node::ImportArgument {
            link: form.fields.config().encode().unwrap(),
            tag: None,
        })
        .collect();
    let imported = runtime.block_on(api.import_nodes(imports)).unwrap();
    assert_eq!(imported.len(), 3);
    for result in imported {
        assert!(
            result.error.is_none(),
            "backend rejected form link: {:?}",
            result.error
        );
    }
}

pub fn inspect_transport_and_tls_forms(capture: impl Fn(&adw::ApplicationWindow, &str)) {
    let form = Form::from_link(
        "vless://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?type=grpc&serviceName=proxy&security=reality&sni=www.example.com&pbk=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA&fp=chrome",
    );
    let window = adw::ApplicationWindow::builder()
        .content(&form.fields.page)
        .default_width(700)
        .default_height(720)
        .build();
    window.present();
    let context = gtk::glib::MainContext::default();
    while context.pending() {
        context.iteration(false);
    }
    let scroll = find::<gtk::ScrolledWindow>(&form.fields.page, &|_| true).unwrap();
    let manager = adw::StyleManager::default();
    let original = manager.color_scheme();
    for (name, scheme) in [
        ("light", adw::ColorScheme::ForceLight),
        ("dark", adw::ColorScheme::ForceDark),
    ] {
        manager.set_color_scheme(scheme);
        scroll.vadjustment().set_value(scroll.vadjustment().upper());
        capture(&window, &format!("node-form-reality-{name}"));
    }
    form.select("Security", 0);
    form.select("Transport type", 0);
    form.select("Header type", 1);
    form.entry("HTTP Host").set_text("cdn.example.com");
    form.entry("Path").set_text("/proxy");
    for (name, scheme) in [
        ("dark", adw::ColorScheme::ForceDark),
        ("light", adw::ColorScheme::ForceLight),
    ] {
        manager.set_color_scheme(scheme);
        while context.pending() {
            context.iteration(false);
        }
        scroll.vadjustment().set_value(scroll.vadjustment().upper());
        capture(&window, &format!("node-form-tcp-http-{name}"));
    }
    manager.set_color_scheme(original);
    window.close();
}
