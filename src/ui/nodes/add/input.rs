use super::AddMsg;
use adw::prelude::*;
use gtk::glib;
use relm4::{Sender, adw, gtk};
use rust_i18n::t;

pub struct NodeInput {
    pub group: adw::PreferencesGroup,
    pub link: RequiredEntry,
    pub tag: RequiredEntry,
    remove: gtk::Button,
    error: adw::ActionRow,
}

impl NodeInput {
    pub fn new(sender: &Sender<AddMsg>) -> Self {
        let group = adw::PreferencesGroup::new();

        let remove = gtk::Button::builder()
            .icon_name("list-remove-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat", "destructive-action"])
            .build();

        group.set_header_suffix(Some(&remove));

        let input = sender.clone();

        remove.connect_clicked(gtk::glib::clone!(
            #[weak]
            group,
            move |_| {
                input.emit(AddMsg::Remove(group.clone()));
            }
        ));

        let error = adw::ActionRow::builder()
            .use_markup(false)
            .title_lines(0)
            .visible(false)
            .css_classes(["caption", "error"])
            .build();

        let link = RequiredEntry::new("nodes.link", &group, sender);
        let tag = RequiredEntry::new("nodes.tag", &group, sender);

        group.add(&error);

        [&link, &tag].iter().for_each(|entry| {
            entry.row.connect_changed(gtk::glib::clone!(
                #[weak]
                error,
                move |_| {
                    error.set_visible(false);
                }
            ));
        });

        Self {
            group,
            link,
            tag,
            remove,
            error,
        }
    }

    pub fn validate(&self) -> bool {
        let link = self.link.validate();
        let tag = self.tag.validate();

        link && tag
    }

    pub fn show_error(&self, error: &str) {
        self.error.set_title(error);
        self.error.set_visible(true);
    }

    pub fn retranslate(&self, number: usize) {
        self.group.set_title(&t!("nodes.item", number = number));

        self.remove
            .set_tooltip_text(Some(&t!("nodes.remove_input")));
        self.link.retranslate();

        self.tag.retranslate();
    }
}

pub struct RequiredEntry {
    pub row: adw::EntryRow,
    error: adw::ActionRow,
    key: &'static str,
}

impl RequiredEntry {
    fn new(key: &'static str, group: &adw::PreferencesGroup, sender: &Sender<AddMsg>) -> Self {
        let row = adw::EntryRow::builder().use_markup(false).build();

        let mark = gtk::Label::builder()
            .label("*")
            .css_classes(["error"])
            .build();

        row.add_suffix(&mark);
        row.upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Required(true)]);

        let error = adw::ActionRow::builder()
            .use_markup(false)
            .title_lines(0)
            .css_classes(["caption", "error"])
            .visible(false)
            .build();

        row.upcast_ref::<gtk::Widget>()
            .update_relation(&[gtk::accessible::Relation::DescribedBy(&[error
                .upcast_ref::<gtk::Widget>()
                .upcast_ref()])]);

        let sender = sender.clone();

        row.connect_changed(move |_| sender.emit(AddMsg::Changed));

        group.add(&row);
        group.add(&error);

        Self { row, error, key }
    }

    fn validate(&self) -> bool {
        let invalid = self.row.text().trim().is_empty();

        self.error.set_visible(invalid);

        if invalid {
            self.row.add_css_class("error");
        } else {
            self.row.remove_css_class("error");
        }

        self.row
            .upcast_ref::<gtk::Widget>()
            .update_state(&[gtk::accessible::State::Invalid(if invalid {
                gtk::AccessibleInvalidState::True
            } else {
                gtk::AccessibleInvalidState::False
            })]);

        !invalid
    }

    fn retranslate(&self) {
        self.row.set_title(&t!(self.key));
        self.error.set_title(&t!("config.required"));
    }
}
