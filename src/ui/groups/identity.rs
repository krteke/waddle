use crate::api::group::Policy;
use adw::prelude::*;
use relm4::{adw, gtk};
use rust_i18n::t;

pub struct Identity {
    pub group: adw::PreferencesGroup,
    pub name: adw::EntryRow,
    pub policy: adw::ComboRow,
}

impl Identity {
    pub fn new(name: &str, policy: Policy) -> Self {
        let group = adw::PreferencesGroup::new();

        let name = adw::EntryRow::builder()
            .text(name)
            .use_markup(false)
            .build();

        let choices: Vec<_> = Policy::ALL.iter().map(|p| p.as_str()).collect();

        let policy = adw::ComboRow::builder()
            .model(&gtk::StringList::new(&choices))
            .selected(Policy::ALL.iter().position(|p| *p == policy).unwrap() as u32)
            .use_markup(false)
            .build();

        group.add(&name);
        group.add(&policy);

        let fields = Self {
            group,
            name,
            policy,
        };

        fields.retranslate();

        fields
    }

    pub fn policy(&self) -> Policy {
        Policy::ALL[self.policy.selected() as usize]
    }

    pub fn retranslate(&self) {
        self.group.set_title(&t!("groups.settings"));

        self.name.set_title(&t!("groups.name"));
        self.name.set_tooltip_text(Some(&t!("groups.name")));

        self.policy.set_title(&t!("groups.policy"));
    }
}

pub fn node_row(name: &str, tag: Option<&str>, protocol: &str) -> adw::ActionRow {
    adw::ActionRow::builder()
        .title(tag.filter(|tag| !tag.is_empty()).unwrap_or(name))
        .subtitle(format!("{} · {}", protocol.to_uppercase(), name))
        .use_markup(false)
        .title_lines(1)
        .subtitle_lines(1)
        .build()
}
