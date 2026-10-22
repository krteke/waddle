use adw::prelude::*;
use relm4::{adw, gtk};
use rust_i18n::t;
use std::cell::Cell;

pub struct FieldNote {
    pub row: adw::PreferencesRow,
    note: adw::ActionRow,
    error: adw::ActionRow,
    pub editable: gtk::Editable,
    pub key: &'static str,
    required_mark: gtk::Label,
    required: Cell<bool>,
}

impl FieldNote {
    pub fn new(
        input: &(impl IsA<adw::PreferencesRow> + IsA<gtk::Editable>),
        key: &'static str,
        required: bool,
    ) -> Self {
        let row = input.upcast_ref::<adw::PreferencesRow>().clone();
        row.set_use_markup(false);

        let editable = input.upcast_ref::<gtk::Editable>().clone();

        let note = adw::ActionRow::builder()
            .use_markup(false)
            .title_lines(0)
            .subtitle_lines(0)
            .activatable(false)
            .build();

        let error = adw::ActionRow::builder()
            .use_markup(false)
            .title_lines(0)
            .activatable(false)
            .visible(false)
            .css_classes(["caption", "error"])
            .build();

        row.bind_property("sensitive", &note, "sensitive")
            .sync_create()
            .build();

        let required_mark = gtk::Label::builder()
            .label("*")
            .css_classes(["error"])
            .visible(required)
            .build();

        if let Some(entry) = row.downcast_ref::<adw::EntryRow>() {
            entry.add_suffix(&required_mark);
        } else {
            let spin = row.downcast_ref::<adw::SpinRow>().unwrap();

            spin.set_title_lines(0);
            spin.set_subtitle_lines(0);
            spin.add_suffix(&required_mark);
        }

        editable.update_property(&[gtk::accessible::Property::Required(required)]);

        editable.update_relation(&[gtk::accessible::Relation::DescribedBy(&[
            note.upcast_ref::<gtk::Widget>().upcast_ref(),
            error.upcast_ref::<gtk::Widget>().upcast_ref(),
        ])]);

        let note = Self {
            row,
            note,
            error,
            editable,
            key,
            required_mark,
            required: Cell::new(required),
        };

        note.retranslate();

        note
    }

    pub fn add_to(&self, group: &adw::PreferencesGroup) {
        group.add(&self.row);
        group.add(&self.error);
        group.add(&self.note);
    }

    pub fn set_required(&self, required: bool) {
        self.required.set(required);

        self.editable
            .update_property(&[gtk::accessible::Property::Required(required)]);

        self.required_mark.set_visible(required);
    }

    pub fn retranslate(&self) {
        let title = t!(format!("config.{}", self.key));

        self.row.set_title(&title);
        self.row.set_tooltip_text(Some(&title));

        let help = t!(format!("config_help.{}", self.key));

        if let Some(spin) = self.row.downcast_ref::<adw::SpinRow>() {
            spin.set_subtitle(&help);
        } else {
            self.note.set_subtitle(&help);
        }

        self.note
            .set_visible(self.note.subtitle().is_some_and(|s| !s.is_empty()));

        if self.row.has_css_class("error") {
            self.validate();
        } else {
            self.clear_error();
        }
    }

    pub fn validate(&self) -> bool {
        let text = self.editable.text();

        let message =
            (self.required.get() && text.trim().is_empty()).then(|| t!("config.required"));

        self.error.set_visible(message.is_some());
        self.error.set_title(message.as_deref().unwrap_or(""));

        if message.is_some() {
            self.row.add_css_class("error");
        } else {
            self.row.remove_css_class("error");
        }

        self.row
            .upcast_ref::<gtk::Widget>()
            .update_state(&[gtk::accessible::State::Invalid(if message.is_some() {
                gtk::AccessibleInvalidState::True
            } else {
                gtk::AccessibleInvalidState::False
            })]);

        message.is_none()
    }

    pub fn clear_error(&self) {
        self.error.set_visible(false);
        self.row.remove_css_class("error");

        self.row
            .upcast_ref::<gtk::Widget>()
            .update_state(&[gtk::accessible::State::Invalid(
                gtk::AccessibleInvalidState::False,
            )]);
    }

    pub fn focus(&self) {
        self.row.grab_focus();
    }
}
