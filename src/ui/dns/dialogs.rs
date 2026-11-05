use adw::prelude::*;
use gtk::glib;
use relm4::{Sender, adw, gtk};
use rust_i18n::t;

use super::DnsMsg;
use crate::api::dns::Dns;

impl Dns {
    pub fn rename(&self, input: Sender<DnsMsg>) -> adw::AlertDialog {
        let entry = gtk::Entry::builder()
            .text(&self.name)
            .activates_default(true)
            .build();

        entry.update_property(&[gtk::accessible::Property::Label(t!("config.name").as_ref())]);
        entry.select_region(0, -1);

        let dialog = adw::AlertDialog::builder()
            .heading(t!("dns.rename_title").as_ref())
            .extra_child(&entry)
            .build();

        dialog.add_response("cancel", t!("dialog.cancel").as_ref());
        dialog.add_response("rename", t!("config.rename").as_ref());
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);

        dialog.set_default_response(Some("rename"));

        dialog.set_close_response("cancel");
        dialog.set_response_enabled("rename", !entry.text().trim().is_empty());

        entry.connect_changed(glib::clone!(
            #[weak]
            dialog,
            move |entry| {
                dialog.set_response_enabled("rename", !entry.text().trim().is_empty());
            }
        ));

        let id = self.id.clone();

        dialog.connect_response(Some("rename"), move |_, _| {
            input.emit(DnsMsg::Rename(id.clone(), entry.text().trim().to_owned()));
        });

        dialog
    }

    pub fn delete(&self, input: Sender<DnsMsg>) -> adw::AlertDialog {
        let dialog = adw::AlertDialog::builder()
            .heading(t!("dns.delete_title").as_ref())
            .body(t!("dns.delete_hint", name = &self.name).as_ref())
            .body_use_markup(false)
            .build();

        dialog.add_response("cancel", t!("dialog.cancel").as_ref());
        dialog.add_response("delete", t!("config.delete").as_ref());

        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");

        let id = self.id.clone();

        dialog.connect_response(Some("delete"), move |_, _| {
            input.emit(DnsMsg::Delete(id.clone()))
        });

        dialog
    }
}

impl Dns {
    pub fn details(&self) -> adw::Dialog {
        let dialog = adw::Dialog::builder()
            .title(&self.name)
            .content_width(740)
            .content_height(560)
            .build();

        let buffer = gtk::TextBuffer::new(None);
        buffer.set_text(&self.dns.string);

        let content = super::editor::code_view(&buffer, false);
        content.set_margin_start(18);
        content.set_margin_end(18);
        content.set_margin_bottom(18);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.set_content(Some(&content));

        dialog.set_child(Some(&toolbar));

        dialog
    }
}
