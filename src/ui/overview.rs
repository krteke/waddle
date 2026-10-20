use adw::prelude::*;
use relm4::{ComponentSender, RelmWidgetExt, adw, gtk};
use rust_i18n::t;

use super::connection::{Connection, ConnectionOutput};

pub struct Overview {
    pub content: gtk::ScrolledWindow,
    title: gtk::Label,
    proxy: adw::ActionRow,
    login: adw::ActionRow,
    sign_out: gtk::Button,
}

impl Overview {
    pub fn new(sender: &ComponentSender<Connection>) -> Self {
        let title = gtk::Label::builder().xalign(0.0).build();
        title.add_css_class("title-1");

        let proxy = adw::ActionRow::new();
        let login = adw::ActionRow::builder().use_markup(false).build();

        let sign_out = gtk::Button::builder()
            .icon_name("system-log-out-symbolic")
            .valign(gtk::Align::Center)
            .build();

        sign_out.connect_clicked({
            let sender = sender.clone();
            move |_| sender.output(ConnectionOutput::SignOut).unwrap()
        });

        login.add_suffix(&sign_out);

        let group = adw::PreferencesGroup::new();
        group.add(&proxy);
        group.add(&login);

        let body = gtk::Box::new(gtk::Orientation::Vertical, 24);

        body.set_margin_all(24);
        body.append(&title);
        body.append(&group);

        let clamp = adw::Clamp::builder().maximum_size(640).child(&body).build();

        let content = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&clamp)
            .build();

        Self {
            content,
            title,
            proxy,
            login,
            sign_out,
        }
    }

    pub fn refresh(&self, username: &str) {
        self.title.set_label(t!("navigation.overview").as_ref());

        self.proxy.set_title(t!("proxy.status").as_ref());
        self.proxy.set_subtitle(t!("proxy.unknown").as_ref());

        self.login.set_title(t!("auth.status").as_ref());
        self.login
            .set_subtitle(t!("auth.signed_in", username = username).as_ref());

        self.sign_out
            .set_tooltip_text(Some(t!("auth.sign_out").as_ref()));
        self.sign_out
            .update_property(&[gtk::accessible::Property::Label(
                t!("auth.sign_out").as_ref(),
            )]);
    }
}
