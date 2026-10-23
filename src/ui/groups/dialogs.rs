use super::{GroupMsg, identity::Identity};
use crate::api::group::{Group, Policy};
use adw::prelude::*;
use gtk::glib;
use relm4::{RelmWidgetExt, Sender, adw, gtk};
use rust_i18n::t;

pub struct NewGroup {
    pub dialog: adw::Dialog,
    fields: Identity,
    create: gtk::Button,
    cancel: gtk::Button,
}

impl NewGroup {
    pub fn new(input: Sender<GroupMsg>) -> Self {
        let dialog = adw::Dialog::builder().content_width(420).build();
        let fields = Identity::new("", Policy::MinMovingAvg);

        let create = gtk::Button::builder()
            .css_classes(["suggested-action"])
            .sensitive(false)
            .build();

        let cancel = gtk::Button::new();

        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();

        header.pack_start(&cancel);
        header.pack_end(&create);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);

        content.set_margin_all(18);
        content.append(&fields.group);

        let toolbar = adw::ToolbarView::new();

        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&content));

        dialog.set_child(Some(&toolbar));

        cancel.connect_clicked(gtk::glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));

        fields.name.connect_changed(gtk::glib::clone!(
            #[weak]
            create,
            move |entry| {
                create.set_sensitive(!entry.text().trim().is_empty());
            }
        ));

        let name = fields.name.clone();
        let policy = fields.policy.clone();

        create.connect_clicked(move |_| {
            input.emit(GroupMsg::Create(
                name.text().trim().to_owned(),
                Policy::ALL[policy.selected() as usize],
            ));
        });

        let form = Self {
            dialog,
            fields,
            create,
            cancel,
        };

        form.retranslate();

        form
    }

    pub fn set_busy(&self, busy: bool) {
        self.dialog.set_can_close(!busy);
        self.fields.group.set_sensitive(!busy);

        self.create
            .set_sensitive(!busy && !self.fields.name.text().trim().is_empty());

        self.cancel.set_sensitive(!busy);
    }

    pub fn retranslate(&self) {
        self.dialog.set_title(&t!("groups.add"));

        self.fields.retranslate();

        self.create.set_label(&t!("groups.create"));
        self.cancel.set_label(&t!("dialog.cancel"));
    }
}

pub fn delete(group: &Group, input: Sender<GroupMsg>) -> adw::AlertDialog {
    let dialog = adw::AlertDialog::builder()
        .heading(t!("groups.delete_title").as_ref())
        .body(t!("groups.delete_hint", name = &group.name).as_ref())
        .body_use_markup(false)
        .build();

    dialog.add_response("cancel", &t!("dialog.cancel"));
    dialog.add_response("delete", &t!("groups.delete"));

    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    let id = group.id.clone();

    dialog.connect_response(Some("delete"), move |_, _| {
        input.emit(GroupMsg::Delete(id.clone()))
    });

    dialog
}
