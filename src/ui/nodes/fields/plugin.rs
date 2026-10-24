use super::{
    widgets::{Choice, Segments, Widgets},
    *,
};
use crate::node::Plugin;

pub struct PluginFields {
    selection: Choice,
    obfs: adw::PreferencesGroup,
    mode: Segments,
    obfs_host: adw::EntryRow,
    obfs_path: adw::EntryRow,
    v2ray: adw::PreferencesGroup,
    tls: adw::SwitchRow,
    host: adw::EntryRow,
    shadow: adw::PreferencesGroup,
    version: Segments,
    shadow_host: adw::EntryRow,
    password: adw::EntryRow,
}
impl PluginFields {
    pub fn new(value: &Plugin, w: &mut Widgets) -> Self {
        let group = w.group("plugin_group");

        let index = match value {
            Plugin::None => 0,
            Plugin::SimpleObfs { .. } => 1,
            Plugin::V2ray { .. } => 2,
            Plugin::ShadowTls { .. } => 3,
        };

        let selection = w.selection(
            &group,
            "node_editor.plugin",
            &["", "simple-obfs", "v2ray-plugin", "shadow-tls"],
            index,
        );

        let obfs = w.group("");

        let mode = w.segmented_choice(&obfs, "node_editor.obfs", &["http", "tls"], "http");
        let obfs_host = w.entry(&obfs, "node_editor.http_host", "", false);
        let obfs_path = w.entry(&obfs, "node_editor.path", "", false);

        let v2ray = w.group("");

        let tls = w.switch(&v2ray, "node_editor.plugin_tls", false);
        let host = w.entry(&v2ray, "node_editor.http_host", "", false);

        let shadow = w.group("");

        let version =
            w.segmented_choice(&shadow, "node_editor.shadowtls_version", &["2", "3"], "3");
        let shadow_host = w.entry(&shadow, "node_editor.http_host", "", false);
        let password = w.password(&shadow, "node_editor.password", "", true);

        match value {
            Plugin::None => {}
            Plugin::SimpleObfs {
                mode: m,
                host,
                path,
            } => {
                mode.control.set_active(u32::from(m == "tls"));
                obfs_host.set_text(host);
                obfs_path.set_text(path);
            }
            Plugin::V2ray {
                tls: enabled,
                host: h,
            } => {
                tls.set_active(*enabled);
                host.set_text(h);
            }
            Plugin::ShadowTls {
                version: v,
                host,
                password: p,
            } => {
                version.control.set_active(u32::from(v == "3"));
                shadow_host.set_text(host);
                password.set_text(p);
            }
        }

        Self {
            selection,
            obfs,
            mode,
            obfs_host,
            obfs_path,
            v2ray,
            tls,
            host,
            shadow,
            version,
            shadow_host,
            password,
        }
    }

    pub fn read(&self) -> Plugin {
        match self.selection.selected() {
            0 => Plugin::None,
            1 => Plugin::SimpleObfs {
                mode: self.mode.text(),
                host: self.obfs_host.text().into(),
                path: self.obfs_path.text().into(),
            },
            2 => Plugin::V2ray {
                tls: self.tls.is_active(),
                host: self.host.text().into(),
            },
            3 => Plugin::ShadowTls {
                version: self.version.text(),
                host: self.shadow_host.text().into(),
                password: self.password.text().into(),
            },
            _ => unreachable!(),
        }
    }

    pub fn update_visibility(&self) {
        self.obfs_path.set_visible(self.mode.selected() == 0);
        self.obfs.set_visible(self.selection.selected() == 1);
        self.v2ray.set_visible(self.selection.selected() == 2);
        self.shadow.set_visible(self.selection.selected() == 3);
    }
}
