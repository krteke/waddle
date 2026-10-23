mod plugin;
mod protocols;
mod tls;
mod transport;
mod widgets;

use super::editor::EditorMsg;
use crate::node::{NodeConfig, ProtocolKind};
use adw::prelude::*;
use protocols::ProtocolFields;
use relm4::{Sender, adw, gtk};
use widgets::Widgets;

pub struct Fields {
    pub page: adw::PreferencesPage,
    tag: adw::EntryRow,
    name: adw::EntryRow,
    server: adw::EntryRow,
    port: adw::SpinRow,
    protocol_fields: ProtocolFields,
    widgets: Widgets,
}

impl Fields {
    pub fn new(config: NodeConfig, tag: &str, sender: &Sender<EditorMsg>) -> Self {
        let mut widgets = Widgets::new(sender);

        let identity = widgets.group("identity");
        let tag = widgets.entry(&identity, "nodes.tag", tag, true);

        let protocol = adw::ComboRow::builder()
            .model(&gtk::StringList::new(
                &ProtocolKind::ALL.map(ProtocolKind::label),
            ))
            .selected(
                ProtocolKind::ALL
                    .iter()
                    .position(|p| *p == config.protocol.kind())
                    .unwrap() as u32,
            )
            .build();

        widgets.row(&identity, "node_editor.protocol", &protocol);

        let input = sender.clone();

        protocol.connect_selected_notify(move |row| {
            input.emit(EditorMsg::Protocol(
                ProtocolKind::ALL[row.selected() as usize],
            ))
        });

        let endpoint = widgets.group("endpoint");

        let name = widgets.entry(&endpoint, "node_editor.name", &config.name, false);
        let server = widgets.entry(&endpoint, "node_editor.server", &config.server, true);

        let port = adw::SpinRow::with_range(1.0, 65535.0, 1.0);

        port.set_numeric(true);
        port.add_suffix(
            &gtk::Label::builder()
                .label("*")
                .css_classes(["error"])
                .build(),
        );

        port.upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Required(true)]);
        port.set_value(f64::from(config.port));

        widgets.row(&endpoint, "node_editor.port", &port);

        let input = sender.clone();

        port.connect_value_notify(move |_| input.emit(EditorMsg::Changed));

        let protocol_fields = ProtocolFields::new(&config.protocol, &mut widgets);

        let fields = Self {
            page: widgets.page.clone(),
            tag,
            name,
            server,
            port,
            protocol_fields,
            widgets,
        };

        fields.retranslate();
        fields.update_visibility();

        fields
    }

    pub fn config(&self) -> NodeConfig {
        NodeConfig {
            name: self.name.text().into(),
            server: self.server.text().into(),
            port: self.port.value() as u16,
            protocol: self.protocol_fields.read(),
        }
    }

    pub fn tag(&self) -> String {
        self.tag.text().trim().into()
    }

    pub fn update_visibility(&self) {
        self.protocol_fields.update_visibility();
    }

    pub fn validate(&self) -> bool {
        self.widgets.validate()
    }

    pub fn commit_numbers(&self) {
        self.port.update();
    }

    pub fn retranslate(&self) {
        self.widgets.retranslate();
    }
}

#[cfg(test)]
pub mod tests;
