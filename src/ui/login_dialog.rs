use adw::prelude::*;
use gtk::glib;
use relm4::{ComponentSender, adw, gtk};
use rust_i18n::t;

use super::connection::{Connection, ConnectionMsg};
use crate::credentials::Credentials;

pub struct LoginDialog {
    server_name: String,
    pub dialog: adw::Dialog,
    username: adw::EntryRow,
    password: adw::PasswordEntryRow,
    remember_password: adw::SwitchRow,
    hint: gtk::Label,
    error: gtk::Label,
    cancel: gtk::Button,
    submit: gtk::Button,
    pub create_admin: bool,
}

impl LoginDialog {
    pub fn new(sender: &ComponentSender<Connection>, server_name: &str) -> Self {
        let dialog = adw::Dialog::builder()
            .content_width(440)
            .content_height(326)
            .build();

        let username = adw::EntryRow::builder().activates_default(true).build();
        let password = adw::PasswordEntryRow::builder()
            .activates_default(true)
            .build();
        let remember_password = adw::SwitchRow::new();

        let hint = gtk::Label::builder().wrap(true).xalign(0.0).build();
        hint.add_css_class("dim-label");

        let error = gtk::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .visible(false)
            .build();

        error.add_css_class("error");

        let cancel = gtk::Button::new();
        let weak_dialog = dialog.downgrade();

        cancel.connect_clicked(move |_| {
            weak_dialog.upgrade().unwrap().close();
        });

        let password_entry = password.clone();
        dialog.connect_closed(move |_| password_entry.set_text(""));

        let submit = gtk::Button::builder().sensitive(false).build();
        submit.add_css_class("suggested-action");

        let input = sender.input_sender().clone();
        submit.connect_clicked(move |_| input.emit(ConnectionMsg::Submit));

        for entry in [
            username.clone().upcast::<gtk::Editable>(),
            password.clone().upcast::<gtk::Editable>(),
        ] {
            entry.connect_changed(gtk::glib::clone!(
                #[weak]
                username,
                #[weak]
                password,
                #[weak]
                submit,
                move |_| {
                    submit.set_sensitive(
                        !username.text().trim().is_empty() && !password.text().is_empty(),
                    );
                }
            ));
        }

        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();
        header.pack_start(&cancel);
        header.pack_end(&submit);

        let group = adw::PreferencesGroup::new();
        group.add(&username);
        group.add(&password);
        group.add(&remember_password);

        let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
        body.set_margin_top(24);
        body.set_margin_bottom(24);
        body.set_margin_start(24);
        body.set_margin_end(24);
        body.append(&hint);
        body.append(&group);
        body.append(&error);

        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .child(&body)
            .build();

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&scroll));

        dialog.set_child(Some(&toolbar));
        dialog.set_default_widget(Some(&submit));

        Self {
            server_name: server_name.into(),
            dialog,
            username,
            password,
            remember_password,
            hint,
            error,
            cancel,
            submit,
            create_admin: false,
        }
    }

    pub fn present(
        &mut self,
        parent: &adw::ApplicationWindow,
        create_admin: bool,
        credentials: &Credentials,
        save_password: bool,
    ) {
        self.create_admin = create_admin;
        self.username.set_text(&credentials.username);
        self.password
            .set_text(credentials.password.as_deref().unwrap_or_default());
        self.remember_password.set_active(save_password);
        self.dialog.set_focus(Some(&self.username));
        self.dialog.present(Some(parent));
    }

    pub fn clear_password(&self) {
        self.password.set_text("");
    }

    pub fn close(&self) {
        self.dialog.set_can_close(true);
        if self.dialog.parent().is_some() {
            self.dialog.close();
        }
    }

    pub fn credentials(&self) -> Credentials {
        Credentials::new(self.username.text().trim(), self.password.text())
    }

    pub fn save_password(&self) -> bool {
        self.remember_password.is_active()
    }

    pub fn refresh(&self, busy: bool, error: Option<&str>) {
        self.dialog.set_title(
            if self.create_admin {
                t!("auth.create_title")
            } else {
                t!("auth.login_title")
            }
            .as_ref(),
        );

        self.username.set_title(t!("auth.username").as_ref());
        self.password.set_title(t!("auth.password").as_ref());
        self.remember_password
            .set_title(t!("auth.save_password").as_ref());

        self.hint.set_label(
            if self.create_admin {
                t!("auth.create_hint", name = &self.server_name)
            } else {
                t!("auth.login_hint", name = &self.server_name)
            }
            .as_ref(),
        );

        self.cancel.set_label(t!("dialog.cancel").as_ref());
        self.submit.set_label(
            if self.create_admin {
                t!("auth.create")
            } else {
                t!("auth.login")
            }
            .as_ref(),
        );

        self.username.set_sensitive(!busy);
        self.password.set_sensitive(!busy);
        self.remember_password.set_sensitive(!busy);

        self.submit.set_sensitive(
            !busy && !self.username.text().trim().is_empty() && !self.password.text().is_empty(),
        );

        self.cancel.set_sensitive(!busy);
        self.dialog.set_can_close(!busy);
        self.error.set_label(error.unwrap_or_default());
        self.error.set_visible(error.is_some());
    }
}
