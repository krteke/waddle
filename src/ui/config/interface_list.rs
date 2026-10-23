use adw::prelude::*;
use relm4::{Sender, adw, gtk};
use rust_i18n::t;

use super::editor::EditorMsg;
use crate::api::config::Interface;

pub struct InterfaceList {
    pub group: adw::PreferencesGroup,
    row: adw::ExpanderRow,
    key: &'static str,
    choices: Vec<(String, gtk::CheckButton)>,
}

impl InterfaceList {
    pub fn new(key: &'static str) -> Self {
        let group = adw::PreferencesGroup::new();
        let row = adw::ExpanderRow::builder().use_markup(false).build();

        group.add(&row);

        Self {
            group,
            row,
            key,
            choices: Vec::new(),
        }
    }

    pub fn populate(
        &mut self,
        interfaces: &[Interface],
        selected: &[String],
        sender: &Sender<EditorMsg>,
    ) {
        let mut names = selected.to_vec();

        if self.key == "wan_interface" {
            names.push("auto".to_owned());
        }

        for interface in interfaces {
            if !names.contains(&interface.name) {
                names.push(interface.name.clone());
            }
        }

        for name in names {
            let button = gtk::CheckButton::builder()
                .valign(gtk::Align::Center)
                .build();

            let row = adw::ActionRow::builder()
                .title(&name)
                .use_markup(false)
                .subtitle_lines(0)
                .activatable_widget(&button)
                .build();

            if let Some(interface) = interfaces.iter().find(|interface| interface.name == name) {
                row.set_subtitle(&interface.ip.join(", "));
            }

            row.add_prefix(&button);

            let sender = sender.clone();
            button.connect_toggled(move |_| sender.emit(EditorMsg::Changed));

            self.row.add_row(&row);
            self.choices.push((name, button));
        }
    }

    pub fn load(&self, selected: &[String]) {
        for (name, button) in &self.choices {
            button.set_active(selected.contains(name));
        }

        self.update_summary();
    }

    pub fn values(&self) -> Vec<String> {
        self.choices
            .iter()
            .filter(|(_, button)| button.is_active())
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn update_summary(&self) {
        let selected = self.values();

        self.row.set_subtitle(
            if selected.is_empty() {
                t!("config.interfaces_none").into_owned()
            } else {
                selected.join(", ")
            }
            .as_str(),
        );
    }

    pub fn retranslate(&self) {
        self.row.set_title(&t!(format!("config.{}", self.key)));
        self.group
            .set_description(Some(&t!(format!("config_help.{}", self.key))));

        self.update_summary();
    }
}
