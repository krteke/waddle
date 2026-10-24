use super::{
    widgets::{Segments, Widgets},
    *,
};
use crate::node::V2rayTls;

pub struct TlsFields {
    security: Segments,
    tls: adw::PreferencesGroup,
    sni: adw::EntryRow,
    alpn: adw::EntryRow,
    reality: adw::PreferencesGroup,
    reality_sni: adw::EntryRow,
    reality_alpn: adw::EntryRow,
    fingerprint: adw::EntryRow,
    public_key: adw::EntryRow,
    short_id: adw::EntryRow,
    spider_x: adw::EntryRow,
}

impl TlsFields {
    pub fn new(value: &V2rayTls, w: &mut Widgets) -> Self {
        let group = w.group("tls");

        let index = match value {
            V2rayTls::None => 0,
            V2rayTls::Tls { .. } => 1,
            V2rayTls::Reality { .. } => 2,
        };

        let security = w.segmented(
            &group,
            "node_editor.security",
            &["", "TLS", "Reality"],
            index,
        );

        let tls = w.group("");

        let sni = w.entry(&tls, "node_editor.sni", "", false);
        let alpn = w.entry(&tls, "node_editor.alpn", "", false);

        let reality = w.group("");

        let reality_sni = w.entry(&reality, "node_editor.sni", "", false);
        let reality_alpn = w.entry(&reality, "node_editor.alpn", "", false);

        let fingerprint = w.entry(&reality, "node_editor.fingerprint", "", false);
        let public_key = w.entry(&reality, "node_editor.public_key", "", true);

        let short_id = w.entry(&reality, "node_editor.short_id", "", false);
        let spider_x = w.entry(&reality, "node_editor.spider_x", "", false);

        match value {
            V2rayTls::None => {}
            V2rayTls::Tls {
                sni: server,
                alpn: protocols,
            } => {
                sni.set_text(server);
                alpn.set_text(protocols);
            }
            V2rayTls::Reality {
                sni,
                alpn,
                fingerprint: fp,
                public_key: key,
                short_id: id,
                spider_x: path,
            } => {
                reality_sni.set_text(sni);
                reality_alpn.set_text(alpn);
                fingerprint.set_text(fp);
                public_key.set_text(key);
                short_id.set_text(id);
                spider_x.set_text(path);
            }
        }

        Self {
            security,
            tls,
            sni,
            alpn,
            reality,
            reality_sni,
            reality_alpn,
            fingerprint,
            public_key,
            short_id,
            spider_x,
        }
    }

    pub fn read(&self) -> V2rayTls {
        match self.security.selected() {
            0 => V2rayTls::None,
            1 => V2rayTls::Tls {
                sni: self.sni.text().into(),
                alpn: self.alpn.text().into(),
            },
            2 => V2rayTls::Reality {
                sni: self.reality_sni.text().into(),
                alpn: self.reality_alpn.text().into(),
                fingerprint: self.fingerprint.text().into(),
                public_key: self.public_key.text().into(),
                short_id: self.short_id.text().into(),
                spider_x: self.spider_x.text().into(),
            },
            _ => unreachable!(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.security.selected() != 0
    }

    pub fn update_visibility(&self) {
        self.tls.set_visible(self.security.selected() == 1);
        self.reality.set_visible(self.security.selected() == 2);
    }
}
