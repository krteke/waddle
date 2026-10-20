use adw::prelude::*;
use gtk::gio;
use relm4::{adw, gtk, prelude::*};
use rust_i18n::t;

use crate::{
    error::ServerSettingsError,
    settings::ServerSettings,
    ui::{
        connection::{Connection, ConnectionMsg, ConnectionOutput},
        server::dialog::ServerDialog,
        server::page::ServerPage,
    },
};

pub struct App {
    settings: ServerSettings,
    active: Option<usize>,
    connection: Option<Controller<Connection>>,
    dialog: ServerDialog,
    server_page: ServerPage,
    error: Option<ServerSettingsError>,
    language_action: gio::SimpleAction,
    language_menu: gio::Menu,
}

#[derive(Debug)]
pub enum AppMsg {
    Opened,
    ShowAdd,
    EditServer(usize),
    SaveServer,
    OpenServer(usize),
    SetDefault(usize),
    SetLanguage(String),
    Connection(usize, ConnectionOutput),
}

#[relm4::component(pub)]
impl Component for App {
    type Init = ServerSettings;
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some("Waddle"),
            set_default_size: (760, 600),
            connect_map => AppMsg::Opened,

            #[name = "navigation"]
            adw::NavigationView {
                set_vexpand: true,
                add = &adw::NavigationPage {
                    set_tag: Some("home"),
                    set_title: "Waddle",
                    adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar {
                            pack_end = &gtk::MenuButton {
                                set_icon_name: "preferences-desktop-locale-symbolic",
                                #[watch]
                                set_tooltip_text: Some(t!("language.title").as_ref()),
                                #[watch]
                                update_property: &[gtk::accessible::Property::Label(t!("language.title").as_ref())],
                                set_menu_model: Some(&model.language_menu),
                            },
                            pack_end = &gtk::Button {
                                set_icon_name: "network-server-symbolic",
                                #[watch]
                                set_tooltip_text: Some(t!("app.servers").as_ref()),
                                #[watch]
                                update_property: &[gtk::accessible::Property::Label(t!("app.servers").as_ref())],
                                set_action_name: Some("navigation.push"),
                                set_action_target_value: Some(&"servers".to_variant()),
                            },
                        },
                        add_top_bar = &adw::Banner {
                            #[watch]
                            set_revealed: model.error.is_some(),
                            #[watch]
                            set_title: &model.error.as_ref().map(ServerSettingsError::user_message).unwrap_or_default(),
                        },
                        gtk::Stack {
                            add_named[Some("empty")] = &adw::StatusPage {
                                set_icon_name: Some("network-server-symbolic"),
                                #[watch]
                                set_title: t!("app.first_server").as_ref(),
                                #[watch]
                                set_description: Some(t!("app.start_hint").as_ref()),
                                gtk::Button {
                                    #[watch]
                                    set_label: t!("app.add_server").as_ref(),
                                    set_halign: gtk::Align::Center,
                                    add_css_class: "suggested-action",
                                    add_css_class: "pill",
                                    connect_clicked => AppMsg::ShowAdd,
                                },
                            },
                            #[name = "connection_content"]
                            add_named[Some("connection")] = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_hexpand: true,
                                set_vexpand: true,
                            },
                            #[watch]
                            set_visible_child_name: if model.active.is_some() { "connection" } else { "empty" },
                        },
                    },
                },
                #[local_ref]
                add = server_page -> adw::NavigationPage {},
            },
        }
    }

    fn init(
        settings: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        crate::i18n::apply_language(&settings.language());
        let language_action = gio::SimpleAction::new_stateful(
            "language",
            Some(&String::static_variant_type()),
            &settings.language().to_variant(),
        );
        let input = sender.input_sender().clone();
        language_action.connect_change_state(move |_, value| {
            input.emit(AppMsg::SetLanguage(value.unwrap().str().unwrap().into()));
        });
        let actions = gio::SimpleActionGroup::new();
        actions.add_action(&language_action);
        root.insert_action_group("preferences", Some(&actions));
        let mut model = Self {
            active: settings.default_server_index(),
            connection: None,
            settings,
            dialog: ServerDialog::new(&sender),
            server_page: ServerPage::new(sender.input_sender().clone()),
            error: None,
            language_action,
            language_menu: gio::Menu::new(),
        };
        model.render_language_menu();
        model
            .server_page
            .refresh(model.settings.servers(), model.active, model.error.as_ref());
        let server_page = &model.server_page.page;
        let widgets = view_output!();
        if model.active.is_some() {
            model.open_connection(&root, &widgets.connection_content, &sender);
        }
        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppMsg::Opened => {
                if self.settings.servers().is_empty() {
                    self.dialog.present(root, &self.settings, None);
                }
            }
            AppMsg::ShowAdd => self.dialog.present(root, &self.settings, None),
            AppMsg::EditServer(index) => self.dialog.present(root, &self.settings, Some(index)),
            AppMsg::SaveServer => {
                match self.settings.save_server(
                    self.dialog.editing,
                    &self.dialog.name.text(),
                    &self.dialog.ip.text(),
                    &self.dialog.port.text(),
                    self.dialog.make_default.is_active(),
                ) {
                    Ok(()) => {
                        if self.dialog.editing.is_none() {
                            self.active = Some(self.settings.servers().len() - 1);
                            widgets.navigation.pop_to_tag("home");
                        }
                        self.dialog.validation_error = None;
                        self.dialog.dialog.close();
                        self.error = None;
                        if self.dialog.editing.is_none() || self.dialog.editing == self.active {
                            self.open_connection(root, &widgets.connection_content, &sender);
                        }
                    }
                    Err(error) => {
                        tracing::warn!(error = %error, "Could not save server");
                        self.dialog.validation_error = Some(error);
                        self.dialog.retranslate();
                    }
                }
            }
            AppMsg::OpenServer(index) => {
                self.active = Some(index);
                self.open_connection(root, &widgets.connection_content, &sender);
                widgets.navigation.pop_to_tag("home");
            }
            AppMsg::Connection(index, event) => {
                if self.active != Some(index) {
                    return;
                }
                match event {
                    ConnectionOutput::Retry => {
                        self.connection.as_ref().unwrap().emit(ConnectionMsg::Retry(
                            self.settings.servers()[index].logged_out,
                        ));
                    }
                    ConnectionOutput::SignOut => {
                        self.error = self.settings.logout(index).err();
                        if self.error.is_none() {
                            self.connection
                                .as_ref()
                                .unwrap()
                                .emit(ConnectionMsg::SignOut);
                        }
                    }
                    ConnectionOutput::SignedIn => {
                        if self.settings.servers()[index].logged_out {
                            self.error = self.settings.login(index).err();
                        }
                    }
                }
            }
            AppMsg::SetDefault(index) => {
                self.error = self.settings.set_default(index).err();
            }
            AppMsg::SetLanguage(language) => match self.settings.set_language(&language) {
                Ok(()) => {
                    crate::i18n::apply_language(&language);
                    self.language_action.set_state(&language.to_variant());
                    self.render_language_menu();
                    self.dialog.retranslate();
                    if let Some(connection) = &self.connection {
                        connection.emit(ConnectionMsg::Retranslate);
                    }
                }
                Err(error) => self.error = Some(error),
            },
        }
        self.update_view(widgets, sender);
    }

    fn post_view() {
        self.server_page
            .refresh(self.settings.servers(), self.active, self.error.as_ref());
    }
}

impl App {
    fn open_connection(
        &mut self,
        root: &adw::ApplicationWindow,
        content: &gtk::Box,
        sender: &ComponentSender<Self>,
    ) {
        if let Some(connection) = self.connection.take() {
            content.remove(connection.widget());
        }
        let index = self.active.unwrap();
        let server = self.settings.servers()[index].clone();
        let connection = Connection::builder()
            .launch((root.clone(), server))
            .forward(sender.input_sender(), move |event| {
                AppMsg::Connection(index, event)
            });
        content.append(connection.widget());
        self.connection = Some(connection);
    }

    fn render_language_menu(&self) {
        self.language_menu.remove_all();
        for (language, label) in [
            ("", t!("language.system").to_string()),
            ("en", t!("language.english").to_string()),
            ("zh_CN", t!("language.chinese").to_string()),
        ] {
            let item = gio::MenuItem::new(Some(&label), None);
            item.set_action_and_target_value(
                Some("preferences.language"),
                Some(&language.to_variant()),
            );
            self.language_menu.append_item(&item);
        }
    }
}

#[cfg(test)]
mod tests;
