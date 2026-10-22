use super::editor::EditorMsg;
use adw::prelude::*;
use relm4::{Sender, adw, gtk};
use rust_i18n::t;

pub struct ChoiceList {
    pub group: adw::PreferencesGroup,
    key: &'static str,
    choices: Vec<(String, adw::ActionRow, gtk::CheckButton)>,
}

impl ChoiceList {
    pub fn new(key: &'static str, values: &[&str], sender: &Sender<EditorMsg>) -> Self {
        let group = adw::PreferencesGroup::new();

        let mut choices = Vec::new();

        values.iter().for_each(|value| {
            let button = gtk::CheckButton::builder()
                .valign(gtk::Align::Center)
                .build();

            if let Some((_, _, first)) = choices.first() {
                button.set_group(Some(first));
            }

            let row = adw::ActionRow::builder()
                .title(*value)
                .use_markup(false)
                .subtitle_lines(0)
                .activatable_widget(&button)
                .build();

            row.add_prefix(&button);

            let sender = sender.clone();
            button.connect_toggled(move |_| sender.emit(EditorMsg::Changed));

            group.add(&row);
            choices.push((value.to_string(), row, button));
        });

        choices[0].2.set_active(true);

        let choice = Self {
            group,
            key,
            choices,
        };

        choice.retranslate();

        choice
    }

    pub fn load(&self, value: &str) {
        self.choices
            .iter()
            .find(|(v, _, _)| v == value)
            .unwrap()
            .2
            .set_active(true);
    }

    pub fn value(&self) -> String {
        self.choices
            .iter()
            .find(|(_, _, button)| button.is_active())
            .unwrap()
            .0
            .clone()
    }

    pub fn retranslate(&self) {
        self.group.set_title(&t!(format!("config.{}", self.key)));

        for (value, row, _) in &self.choices {
            row.set_subtitle(&t!(format!("config_help.{}_{}", self.key, value)));
        }
    }
}
