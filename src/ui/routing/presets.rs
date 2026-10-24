use super::editor::EditorMsg;
use crate::routing::{MacAction, Mode, Preset};
use adw::prelude::*;
use relm4::{Sender, adw, gtk};
use rust_i18n::t;
use std::{cell::Cell, rc::Rc};

pub struct Presets {
    pub page: adw::PreferencesPage,
    targets: Vec<String>,
    target_group: adw::PreferencesGroup,
    target: adw::ComboRow,
    modes: Vec<(adw::ActionRow, gtk::CheckButton)>,
    mode_group: adw::PreferencesGroup,
    mac_group: adw::PreferencesGroup,
    mac_enabled: adw::SwitchRow,
    mac_action: adw::ToggleGroup,
    action_row: adw::ActionRow,
    addresses: adw::PreferencesGroup,
    entries: Vec<(adw::EntryRow, adw::ActionRow)>,
    add: gtk::Button,
    loading: Rc<Cell<bool>>,
    input: Sender<EditorMsg>,
}

impl Presets {
    pub fn new(targets: Vec<String>, input: &Sender<EditorMsg>) -> Self {
        let page = adw::PreferencesPage::new();
        let target_group = adw::PreferencesGroup::new();

        let labels: Vec<_> = targets.iter().map(String::as_str).collect();

        let target = adw::ComboRow::builder()
            .model(&gtk::StringList::new(&labels))
            .selected(targets.iter().position(|g| g == "proxy").unwrap_or(0) as u32)
            .use_markup(false)
            .build();

        target.set_sensitive(!targets.is_empty());
        target_group.add(&target);

        page.add(&target_group);

        let mode_group = adw::PreferencesGroup::new();

        let mut modes: Vec<(adw::ActionRow, gtk::CheckButton)> = Vec::new();

        let loading = Rc::new(Cell::new(true));

        for _ in Mode::ALL {
            let check = gtk::CheckButton::builder()
                .valign(gtk::Align::Center)
                .build();

            if let Some((_, first)) = modes.first() {
                check.set_group(Some(first));
            }

            let row = adw::ActionRow::builder()
                .use_markup(false)
                .subtitle_lines(0)
                .activatable_widget(&check)
                .build();

            row.add_prefix(&check);
            mode_group.add(&row);

            let input = input.clone();
            let loading = loading.clone();

            check.connect_toggled(move |check| {
                if check.is_active() && !loading.get() {
                    input.emit(EditorMsg::PresetChanged);
                }
            });

            modes.push((row, check));
        }

        modes[0].1.set_active(true);
        page.add(&mode_group);

        let mac_group = adw::PreferencesGroup::new();
        let mac_enabled = adw::SwitchRow::new();

        mac_group.add(&mac_enabled);

        let mac_action = adw::ToggleGroup::new();

        mac_action.add(adw::Toggle::builder().label("Proxy").build());
        mac_action.add(adw::Toggle::builder().label("Direct").build());
        mac_action.set_valign(gtk::Align::Center);

        let action_row = adw::ActionRow::new();
        action_row.add_suffix(&mac_action);

        mac_group.add(&action_row);
        page.add(&mac_group);

        let addresses = adw::PreferencesGroup::new();
        let add = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .css_classes(["flat"])
            .build();

        let sender = input.clone();

        add.connect_clicked(move |_| sender.emit(EditorMsg::AddMac));
        addresses.set_header_suffix(Some(&add));
        page.add(&addresses);

        let sender = input.clone();
        let flag = loading.clone();

        target.connect_selected_notify(move |_| {
            if !flag.get() {
                sender.emit(EditorMsg::PresetChanged);
            }
        });

        let sender = input.clone();
        let flag = loading.clone();

        mac_enabled.connect_active_notify(move |_| {
            if !flag.get() {
                sender.emit(EditorMsg::PresetChanged);
            }
        });

        let sender = input.clone();
        let flag = loading.clone();

        mac_action.connect_active_notify(move |_| {
            if !flag.get() {
                sender.emit(EditorMsg::PresetChanged);
            }
        });

        let form = Self {
            page,
            targets,
            target_group,
            target,
            modes,
            mode_group,
            mac_group,
            mac_enabled,
            mac_action,
            action_row,
            addresses,
            entries: vec![],
            add,
            loading,
            input: input.clone(),
        };

        form.loading.set(false);
        form.retranslate();
        form.update_visibility();

        form
    }

    pub fn read(&self) -> Option<Preset> {
        let mode = Mode::ALL[self.modes.iter().position(|(_, c)| c.is_active()).unwrap()];
        let group = self.targets.get(self.target.selected() as usize)?.clone();

        let macs = if self.mac_enabled.is_active() {
            self.entries
                .iter()
                .map(|(e, _)| e.text().trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            vec![]
        };

        let mac_action = if self.mac_action.active() == 0 {
            MacAction::Proxy
        } else {
            MacAction::Direct
        };

        Some(Preset {
            mode,
            group,
            macs,
            mac_action,
        })
    }

    pub fn detect(&self, text: &str) -> Option<Preset> {
        self.targets.iter().find_map(|g| Preset::detect(text, g))
    }

    pub fn load(&mut self, preset: &Preset) {
        self.loading.set(true);

        self.target.set_selected(
            self.targets
                .iter()
                .position(|g| *g == preset.group)
                .unwrap() as u32,
        );

        self.modes[Mode::ALL.iter().position(|m| *m == preset.mode).unwrap()]
            .1
            .set_active(true);

        self.mac_enabled.set_active(!preset.macs.is_empty());
        self.mac_action
            .set_active(u32::from(preset.mac_action == MacAction::Direct));

        for (entry, error) in self.entries.drain(..) {
            self.addresses.remove(&entry);
            self.addresses.remove(&error);
        }

        for mac in &preset.macs {
            self.add_mac(mac);
        }

        self.loading.set(false);

        self.update_visibility();
    }

    pub fn add_mac(&mut self, value: &str) {
        let entry = adw::EntryRow::builder().title("MAC").text(value).build();

        let error = adw::ActionRow::builder()
            .title(t!("routing.mac_invalid").as_ref())
            .css_classes(["caption", "error"])
            .visible(false)
            .build();

        let remove = gtk::Button::builder()
            .icon_name("list-remove-symbolic")
            .css_classes(["flat", "error"])
            .valign(gtk::Align::Center)
            .tooltip_text(t!("config.remove_item").as_ref())
            .build();

        let input = self.input.clone();
        let weak = entry.downgrade();

        remove.connect_clicked(move |_| input.emit(EditorMsg::RemoveMac(weak.upgrade().unwrap())));

        entry.add_suffix(&remove);

        let input = self.input.clone();
        let flag = self.loading.clone();

        entry.connect_changed(move |_| {
            if !flag.get() {
                input.emit(EditorMsg::PresetChanged);
            }
        });

        self.addresses.add(&entry);
        self.addresses.add(&error);

        self.entries.push((entry, error));
    }

    pub fn remove_mac(&mut self, row: &adw::EntryRow) {
        let index = self.entries.iter().position(|(e, _)| e == row).unwrap();
        let (entry, error) = self.entries.remove(index);

        self.addresses.remove(&entry);
        self.addresses.remove(&error);
    }

    pub fn validate(&self) -> bool {
        let mut valid = self.targets.get(self.target.selected() as usize).is_some();

        for (entry, error) in &self.entries {
            let invalid = self.mac_enabled.is_active() && !entry.text().trim().is_empty();

            error.set_visible(invalid);

            if invalid {
                entry.add_css_class("error");
            } else {
                entry.remove_css_class("error");
            }

            valid &= !invalid;
        }

        valid
    }

    pub fn update_visibility(&self) {
        self.action_row.set_visible(self.mac_enabled.is_active());
        self.addresses.set_visible(self.mac_enabled.is_active());
    }

    pub fn retranslate(&self) {
        self.target.set_title(&t!("routing.proxy_group"));

        self.target_group.set_description(
            self.targets
                .is_empty()
                .then(|| t!("routing.no_groups"))
                .as_deref(),
        );

        self.mode_group.set_title(&t!("routing.mode"));

        let target = self
            .targets
            .get(self.target.selected() as usize)
            .map(String::as_str)
            .unwrap_or("—");

        for ((row, _), key) in self
            .modes
            .iter()
            .zip(["gfw", "non_cn", "cn_only", "global", "mac_only"])
        {
            row.set_title(&t!(format!("routing.{key}")));
            row.set_subtitle(&t!(format!("routing.{key}_hint"), group = target));
        }

        self.mac_group.set_title(&t!("routing.mac_control"));
        self.mac_enabled.set_title(&t!("routing.mac_enable"));
        self.mac_enabled.set_subtitle(&t!("routing.mac_hint"));

        self.action_row.set_title(&t!("routing.mac_action"));

        self.mac_action
            .toggle(0)
            .unwrap()
            .set_label(Some(&t!("routing.proxy")));

        self.mac_action
            .toggle(1)
            .unwrap()
            .set_label(Some(&t!("routing.direct")));

        self.addresses.set_title(&t!("routing.mac_addresses"));
        self.add.set_tooltip_text(Some(&t!("routing.add_mac")));

        for (entry, error) in &self.entries {
            error.set_title(&t!("routing.mac_invalid"));
            entry.set_tooltip_text(Some("AA:BB:CC:DD:EE:FF"));
        }
    }
}
