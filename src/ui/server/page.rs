use adw::prelude::*;
use relm4::{RelmWidgetExt, Sender, adw, gtk};
use rust_i18n::t;

use crate::{
    app::AppMsg,
    error::{Message, ServerSettingsError},
    settings::Server,
};

pub struct ServerPage {
    pub page: adw::NavigationPage,
    add: gtk::Button,
    error: adw::Banner,
    content: gtk::Stack,
    empty: adw::StatusPage,
    empty_add: gtk::Button,
    group: adw::PreferencesGroup,
    servers: gtk::ListBox,
    input: Sender<AppMsg>,
}

impl ServerPage {
    pub fn confirm_delete(&self, parent: &adw::ApplicationWindow, index: usize, name: &str) {
        let dialog = adw::AlertDialog::builder()
            .heading(t!("server.delete_title").as_ref())
            .body(t!("server.delete_hint", name = name).as_ref())
            .build();

        dialog.add_response("cancel", t!("dialog.cancel").as_ref());
        dialog.add_response("delete", t!("server.delete").as_ref());
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

        let input = self.input.clone();

        dialog.connect_response(Some("delete"), move |_, _| {
            input.emit(AppMsg::DeleteServer(index))
        });
        dialog.present(Some(parent));
    }

    pub fn new(input: Sender<AppMsg>) -> Self {
        let add = gtk::Button::from_icon_name("list-add-symbolic");
        let messages = input.clone();
        add.connect_clicked(move |_| messages.emit(AppMsg::ShowAdd));
        let header = adw::HeaderBar::new();
        header.pack_start(&add);
        let error = adw::Banner::new("");

        let empty_add = gtk::Button::new();
        empty_add.set_halign(gtk::Align::Center);
        empty_add.add_css_class("suggested-action");
        empty_add.add_css_class("pill");
        let messages = input.clone();
        empty_add.connect_clicked(move |_| messages.emit(AppMsg::ShowAdd));
        let empty = adw::StatusPage::builder()
            .icon_name("network-server-symbolic")
            .child(&empty_add)
            .build();

        let servers = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .build();
        servers.add_css_class("boxed-list");
        let group = adw::PreferencesGroup::builder()
            .valign(gtk::Align::Start)
            .build();
        group.add(&servers);
        let clamp = adw::Clamp::builder()
            .maximum_size(640)
            .child(&group)
            .build();
        clamp.set_margin_all(24);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&clamp)
            .build();
        let content = gtk::Stack::new();
        content.add_named(&empty, Some("empty"));
        content.add_named(&scroll, Some("list"));
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.add_top_bar(&error);
        toolbar.set_content(Some(&content));
        let page = adw::NavigationPage::builder()
            .tag("servers")
            .child(&toolbar)
            .build();

        Self {
            page,
            add,
            error,
            content,
            empty,
            empty_add,
            group,
            servers,
            input,
        }
    }

    pub fn refresh(
        &self,
        servers: &[Server],
        active: Option<usize>,
        error: Option<&ServerSettingsError>,
    ) {
        self.page.set_title(t!("app.servers").as_ref());
        self.add
            .set_tooltip_text(Some(t!("app.add_server").as_ref()));
        self.add.update_property(&[gtk::accessible::Property::Label(
            t!("app.add_server").as_ref(),
        )]);
        self.empty.set_title(t!("app.first_server").as_ref());
        self.empty
            .set_description(Some(t!("app.start_hint").as_ref()));
        self.empty_add.set_label(t!("app.add_server").as_ref());
        self.group
            .set_description(Some(t!("app.default_server_hint").as_ref()));
        self.error
            .set_title(&error.map(|e| e.user_message()).unwrap_or_default());
        self.error.set_revealed(error.is_some());
        self.content
            .set_visible_child_name(if servers.is_empty() { "empty" } else { "list" });

        self.servers.remove_all();
        for (index, server) in servers.iter().enumerate() {
            let row = adw::ActionRow::builder()
                .title(&server.name)
                .use_markup(false)
                .title_lines(1)
                .subtitle_lines(1)
                .activatable(server.url.is_ok())
                .build();

            match &server.url {
                Ok(url) => row.set_subtitle(url.as_str()),
                Err(_) => {
                    row.set_subtitle(t!("server.invalid_url").as_ref());
                    let icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
                    icon.add_css_class("error");
                    row.add_prefix(&icon);
                }
            }
            let default = gtk::Button::builder()
                .icon_name(if server.is_default {
                    "starred-symbolic"
                } else {
                    "non-starred-symbolic"
                })
                .tooltip_text(if server.is_default {
                    t!("server.default").to_string()
                } else {
                    t!("server.make_default").to_string()
                })
                .sensitive(!server.is_default && server.url.is_ok())
                .valign(gtk::Align::Center)
                .build();
            default.add_css_class("flat");
            default.update_property(&[gtk::accessible::Property::Label(
                if server.is_default {
                    t!("server.default")
                } else {
                    t!("server.make_default")
                }
                .as_ref(),
            )]);
            let input = self.input.clone();
            default.connect_clicked(move |_| {
                input.emit(AppMsg::SetDefault(index));
            });
            row.add_suffix(&default);

            let edit = gtk::Button::builder()
                .icon_name("document-edit-symbolic")
                .tooltip_text(t!("server.edit").as_ref())
                .valign(gtk::Align::Center)
                .build();
            edit.add_css_class("flat");
            edit.update_property(&[gtk::accessible::Property::Label(t!("server.edit").as_ref())]);
            let input = self.input.clone();
            edit.connect_clicked(move |_| input.emit(AppMsg::EditServer(index)));
            row.add_suffix(&edit);

            let delete = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .tooltip_text(t!("server.delete_title").as_ref())
                .valign(gtk::Align::Center)
                .build();
            delete.add_css_class("flat");
            delete.update_property(&[gtk::accessible::Property::Label(
                t!("server.delete_title").as_ref(),
            )]);

            let input = self.input.clone();
            delete.connect_clicked(move |_| input.emit(AppMsg::ShowDelete(index)));
            row.add_suffix(&delete);

            let current = active == Some(index);
            let indicator = gtk::Image::from_icon_name(if current {
                "object-select-symbolic"
            } else {
                "go-next-symbolic"
            });
            if current {
                indicator.set_tooltip_text(Some(t!("server.current").as_ref()));
                indicator.update_property(&[gtk::accessible::Property::Label(
                    t!("server.current").as_ref(),
                )]);
            } else {
                indicator.add_css_class("dim-label");
            }
            indicator.set_visible(server.url.is_ok());
            row.add_suffix(&indicator);
            if server.url.is_ok() {
                let input = self.input.clone();
                row.connect_activated(move |_| {
                    input.emit(AppMsg::OpenServer(index));
                });
            }
            self.servers.append(&row);
        }
    }
}
