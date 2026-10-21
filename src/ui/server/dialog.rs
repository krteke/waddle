use adw::prelude::*;
use gtk::glib;
use relm4::{adw, gtk, prelude::*};
use rust_i18n::t;

use crate::{
    app::{App, AppMsg},
    error::{Message, ServerSettingsError},
    settings::ServerSettings,
};

pub struct ServerDialog {
    pub editing: Option<usize>,
    pub dialog: adw::Dialog,
    pub name: adw::EntryRow,
    pub host: gtk::Entry,
    pub port: gtk::Entry,
    pub make_default: gtk::CheckButton,
    error: gtk::Label,
    pub validation_error: Option<ServerSettingsError>,
    cancel: gtk::Button,
    save: gtk::Button,
}

impl ServerDialog {
    pub fn new(sender: &ComponentSender<App>) -> Self {
        let dialog = adw::Dialog::builder()
            .content_width(440)
            .content_height(306)
            .build();

        let cancel = gtk::Button::new();
        cancel.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));

        let save = gtk::Button::new();
        save.add_css_class("suggested-action");
        let input = sender.input_sender().clone();
        save.connect_clicked(move |_| input.emit(AppMsg::SaveServer));

        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();
        header.pack_start(&cancel);
        header.pack_end(&save);

        let name = adw::EntryRow::builder().activates_default(true).build();
        let host = gtk::Entry::builder()
            .activates_default(true)
            .hexpand(true)
            .width_chars(8)
            .placeholder_text("localhost")
            .input_purpose(gtk::InputPurpose::Url)
            .build();

        let port = gtk::Entry::builder()
            .activates_default(true)
            .input_purpose(gtk::InputPurpose::Digits)
            .width_chars(5)
            .max_width_chars(5)
            .build();

        let address = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        address.set_margin_all(12);

        let scheme = gtk::Label::new(Some("http://"));
        scheme.add_css_class("dim-label");

        let separator = gtk::Label::new(Some(":"));
        separator.add_css_class("dim-label");

        address.append(&scheme);
        address.append(&host);
        address.append(&separator);
        address.append(&port);

        let address_frame = gtk::Frame::builder().child(&address).build();
        let group = adw::PreferencesGroup::new();
        group.add(&name);

        let make_default = gtk::CheckButton::new();
        let error = gtk::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .visible(false)
            .build();
        error.add_css_class("error");

        let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
        body.set_margin_all(24);
        body.append(&group);
        body.append(&address_frame);
        body.append(&make_default);
        body.append(&error);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .child(&body)
            .build();
        toolbar.set_content(Some(&scroll));
        dialog.set_child(Some(&toolbar));
        dialog.set_default_widget(Some(&save));
        dialog.set_focus(Some(&name));

        let form = Self {
            editing: None,
            dialog,
            name,
            host,
            port,
            make_default,
            error,
            validation_error: None,
            cancel,
            save,
        };
        form.retranslate();
        form
    }

    pub fn retranslate(&self) {
        self.dialog.set_title(
            if self.editing.is_some() {
                t!("server.edit")
            } else {
                t!("dialog.title")
            }
            .as_ref(),
        );
        self.cancel.set_label(t!("dialog.cancel").as_ref());
        self.save.set_label(
            if self.editing.is_some() {
                t!("dialog.save")
            } else {
                t!("dialog.connect")
            }
            .as_ref(),
        );
        self.name.set_title(t!("dialog.server_name").as_ref());
        self.host.set_tooltip_text(Some(t!("dialog.host").as_ref()));
        self.host
            .update_property(&[gtk::accessible::Property::Label(t!("dialog.host").as_ref())]);
        self.port.set_tooltip_text(Some(t!("dialog.port").as_ref()));
        self.port
            .update_property(&[gtk::accessible::Property::Label(t!("dialog.port").as_ref())]);
        self.make_default
            .set_label(Some(t!("dialog.make_default").as_ref()));
        self.error.set_label(
            &self
                .validation_error
                .as_ref()
                .map(|e| e.user_message())
                .unwrap_or_default(),
        );
        self.error.set_visible(self.validation_error.is_some());
    }

    pub fn present(
        &mut self,
        parent: &adw::ApplicationWindow,
        settings: &ServerSettings,
        editing: Option<usize>,
    ) {
        self.editing = editing;
        match editing {
            Some(index) => {
                let server = &settings.servers()[index];
                self.name.set_text(&server.name);
                match &server.url {
                    Ok(url) => {
                        self.host.set_text(url.host_str().unwrap_or_default());
                        self.port.set_text(
                            &url.port_or_known_default()
                                .map(|port| port.to_string())
                                .unwrap_or_default(),
                        );
                    }
                    Err(error) => {
                        self.host.set_text(&error.value);
                        self.port.set_text("2023");
                    }
                }
                self.make_default.set_active(server.is_default);
                self.make_default.set_sensitive(!server.is_default);
            }
            None => {
                self.name.set_text("");
                self.host.set_text("");
                self.port.set_text("2023");
                self.make_default.set_active(settings.servers().is_empty());
                self.make_default
                    .set_sensitive(!settings.servers().is_empty());
            }
        }
        self.validation_error = None;
        self.retranslate();
        self.dialog.set_focus(Some(&self.name));
        self.dialog.present(Some(parent));
    }
}
