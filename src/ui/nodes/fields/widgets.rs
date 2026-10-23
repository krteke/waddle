use super::*;
use rust_i18n::t;

pub struct Widgets {
    pub page: adw::PreferencesPage,
    sender: Sender<EditorMsg>,
    rows: Vec<(&'static str, adw::PreferencesRow)>,
    groups: Vec<(&'static str, adw::PreferencesGroup)>,
    required: Vec<(adw::EntryRow, adw::ActionRow)>,
    choices: Vec<Choice>,
    descriptions: Vec<(&'static str, adw::PreferencesGroup)>,
}

pub struct Choice {
    pub row: adw::ComboRow,
    values: &'static [&'static str],
}

impl Choice {
    pub fn text(&self) -> String {
        self.values[self.row.selected() as usize].into()
    }

    pub fn selected(&self) -> u32 {
        self.row.selected()
    }
}

impl Widgets {
    pub fn new(sender: &Sender<EditorMsg>) -> Self {
        Self {
            page: adw::PreferencesPage::new(),
            sender: sender.clone(),
            rows: vec![],
            groups: vec![],
            required: vec![],
            choices: vec![],
            descriptions: vec![],
        }
    }

    pub fn group(&mut self, label: &'static str) -> adw::PreferencesGroup {
        let group = adw::PreferencesGroup::new();

        self.page.add(&group);
        self.groups.push((label, group.clone()));

        group
    }

    pub fn description(&mut self, group: &adw::PreferencesGroup, key: &'static str) {
        self.descriptions.push((key, group.clone()));
    }

    pub fn row(
        &mut self,
        group: &adw::PreferencesGroup,
        label: &'static str,
        row: &impl IsA<adw::PreferencesRow>,
    ) {
        row.as_ref().set_use_markup(false);
        group.add(row.as_ref());

        self.rows.push((label, row.as_ref().clone()));
    }

    pub fn entry(
        &mut self,
        group: &adw::PreferencesGroup,
        label: &'static str,
        value: &str,
        required: bool,
    ) -> adw::EntryRow {
        self.editable(group, label, value, required, adw::EntryRow::new())
    }

    pub fn password(
        &mut self,
        group: &adw::PreferencesGroup,
        label: &'static str,
        value: &str,
        required: bool,
    ) -> adw::EntryRow {
        self.editable(
            group,
            label,
            value,
            required,
            adw::PasswordEntryRow::new().upcast(),
        )
    }

    fn editable(
        &mut self,
        group: &adw::PreferencesGroup,
        label: &'static str,
        value: &str,
        required: bool,
        row: adw::EntryRow,
    ) -> adw::EntryRow {
        row.set_text(value);
        self.row(group, label, &row);

        if required {
            row.add_suffix(
                &gtk::Label::builder()
                    .label("*")
                    .css_classes(["error"])
                    .build(),
            );

            row.upcast_ref::<gtk::Widget>()
                .update_property(&[gtk::accessible::Property::Required(true)]);

            let error = adw::ActionRow::builder()
                .visible(false)
                .css_classes(["caption", "error"])
                .use_markup(false)
                .build();

            row.upcast_ref::<gtk::Widget>().update_relation(&[
                gtk::accessible::Relation::DescribedBy(&[error
                    .upcast_ref::<gtk::Widget>()
                    .upcast_ref()]),
            ]);

            group.add(&error);

            self.required.push((row.clone(), error));
        }

        let input = self.sender.clone();

        row.connect_changed(move |_| input.emit(EditorMsg::Changed));

        row
    }

    pub fn switch(
        &mut self,
        group: &adw::PreferencesGroup,
        label: &'static str,
        active: bool,
    ) -> adw::SwitchRow {
        let row = adw::SwitchRow::builder().active(active).build();
        self.row(group, label, &row);

        let input = self.sender.clone();
        row.connect_active_notify(move |_| input.emit(EditorMsg::Changed));

        row
    }

    pub fn choice(
        &mut self,
        group: &adw::PreferencesGroup,
        label: &'static str,
        values: &'static [&'static str],
        value: &str,
    ) -> Choice {
        let selected = values
            .iter()
            .position(|v| *v == value)
            .expect("domain value is a supported choice") as u32;

        self.selection(group, label, values, selected)
    }

    pub fn selection(
        &mut self,
        group: &adw::PreferencesGroup,
        label: &'static str,
        values: &'static [&'static str],
        selected: u32,
    ) -> Choice {
        let row = adw::ComboRow::builder()
            .model(&gtk::StringList::new(values))
            .selected(selected)
            .build();

        self.row(group, label, &row);

        let input = self.sender.clone();

        row.connect_selected_notify(move |_| input.emit(EditorMsg::Changed));

        self.choices.push(Choice {
            row: row.clone(),
            values,
        });

        Choice { row, values }
    }

    pub fn validate(&self) -> bool {
        let mut valid = true;

        for (row, error) in &self.required {
            let invalid = row.is_visible() && row.text().trim().is_empty();
            error.set_visible(invalid);

            if invalid {
                row.add_css_class("error");

                if valid {
                    row.grab_focus();
                }
            } else {
                row.remove_css_class("error");
            }

            row.upcast_ref::<gtk::Widget>()
                .update_state(&[gtk::accessible::State::Invalid(if invalid {
                    gtk::AccessibleInvalidState::True
                } else {
                    gtk::AccessibleInvalidState::False
                })]);

            valid &= !invalid;
        }

        valid
    }

    pub fn retranslate(&self) {
        for (label, row) in &self.rows {
            row.set_title(&t!(*label));

            if *label == "node_editor.insecure" {
                row.downcast_ref::<adw::SwitchRow>()
                    .unwrap()
                    .set_subtitle(&t!("node_editor.insecure_hint"));
            }
        }

        for (label, group) in &self.groups {
            if !label.is_empty() {
                group.set_title(&t!(format!("node_editor.{label}")));
            }

            if matches!(*label, "endpoint" | "tls" | "plugin_group") {
                group.set_description(Some(&t!(format!("node_editor.{label}_hint"))));
            }
        }

        for (key, group) in &self.descriptions {
            group.set_description(Some(&t!(*key)));
        }

        for (_, error) in &self.required {
            error.set_title(&t!("config.required"));
        }

        for choice in &self.choices {
            if choice.values.contains(&"") {
                let model = choice
                    .row
                    .model()
                    .unwrap()
                    .downcast::<gtk::StringList>()
                    .unwrap();

                let index = choice.values.iter().position(|v| v.is_empty()).unwrap() as u32;
                let selected = choice.selected();

                model.splice(index, 1, &[&t!("node_editor.none")]);
                choice.row.set_selected(selected);
            }
        }
    }
}
