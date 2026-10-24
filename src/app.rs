use adw::prelude::*;
use gtk::gio;
use relm4::{adw, gtk, prelude::*};
use rust_i18n::t;

use crate::{
    credentials,
    error::{CredentialError, Message, ServerSettingsError},
    settings::ServerSettings,
    ui::{
        connection::{Connection, ConnectionMsg, ConnectionOutput},
        server::{dialog::ServerDialog, page::ServerPage},
        workspace::Workspace,
    },
};

pub struct App {
    settings: ServerSettings,
    active: Option<usize>,
    connection: Option<Controller<Connection>>,
    dialog: ServerDialog,
    server_page: ServerPage,
    workspace: Workspace,
    error: Option<ServerSettingsError>,
    deleting: bool,
    language_action: gio::SimpleAction,
    language_menu: gio::Menu,
}

#[derive(Debug)]
pub enum AppMsg {
    Opened,
    ShowAdd,
    EditServer(usize),
    ShowDelete(usize),
    DeleteServer(usize),
    SaveServer,
    OpenServer(usize),
    SetDefault(usize),
    SetLanguage(String),
    Connection(String, ConnectionOutput),
}

#[relm4::component(pub)]
impl Component for App {
    type Init = ServerSettings;
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = Result<(), CredentialError>;

    view! {
        adw::ApplicationWindow {
            #[watch]
            set_sensitive: !model.deleting,
            set_title: Some("Waddle"),
            set_default_size: (900, 640),
            set_size_request: (360, 360),
            connect_map => AppMsg::Opened,

            #[name = "navigation"]
            adw::NavigationView {
                set_vexpand: true,
                add = &adw::NavigationPage {
                    set_tag: Some("home"),
                    set_title: "Waddle",
                    #[local_ref]
                    workspace -> adw::OverlaySplitView {},
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
        let language_menu = gio::Menu::new();
        let mut model = Self {
            active: settings.default_server_index(),
            connection: None,
            settings,
            dialog: ServerDialog::new(&sender),
            server_page: ServerPage::new(sender.input_sender().clone()),
            workspace: Workspace::new(&language_menu),
            error: None,
            deleting: false,
            language_action,
            language_menu,
        };
        model.render_language_menu();
        model
            .server_page
            .refresh(model.settings.servers(), model.active, model.error.as_ref());
        model.workspace.refresh(
            model.settings.servers(),
            model.active,
            None,
            sender.input_sender(),
        );
        let breakpoint =
            adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 640sp").unwrap());
        breakpoint.add_setters(&[
            (&model.workspace.view, "collapsed", true),
            (&model.workspace.view, "show-sidebar", false),
        ]);
        root.add_breakpoint(breakpoint);
        let workspace = &model.workspace.view;
        let server_page = &model.server_page.page;
        let widgets = view_output!();
        if model.active.is_some() {
            model.open_connection(&root, &sender);
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
            AppMsg::ShowDelete(index) => {
                self.server_page
                    .confirm_delete(root, index, &self.settings.servers()[index].name);
            }
            AppMsg::DeleteServer(index) => {
                let name = self.settings.servers()[index].name.clone();
                match self.settings.delete_server(index) {
                    Ok(()) => {
                        self.error = None;
                        self.deleting = true;
                        match self.active {
                            Some(active) if active == index => {
                                self.active = self.settings.default_server_index();
                                self.open_connection(root, &sender);
                            }
                            Some(active) if active > index => self.active = Some(active - 1),
                            _ => {}
                        }
                        sender.oneshot_command(async move {
                            credentials::delete(&name).await.inspect_err(|error| {
                                tracing::warn!(server = %name, %error, "Could not delete saved credentials");
                            })
                        });
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            AppMsg::SaveServer => {
                match self.settings.save_server(
                    self.dialog.editing,
                    &self.dialog.name.text(),
                    &format!(
                        "http://{}:{}",
                        self.dialog.host.text().trim(),
                        self.dialog.port.text().trim()
                    ),
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
                            self.open_connection(root, &sender);
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
                self.open_connection(root, &sender);
                widgets.navigation.pop_to_tag("home");
            }
            AppMsg::Connection(name, event) => {
                let Some(index) = self.active else {
                    return;
                };
                if self.settings.servers()[index].name != name {
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
        self.workspace.refresh(
            self.settings.servers(),
            self.active,
            self.error.as_ref().map(|e| e.user_message()).as_deref(),
            sender.input_sender(),
        );
        self.server_page
            .refresh(self.settings.servers(), self.active, self.error.as_ref());
    }

    fn update_cmd(
        &mut self,
        result: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.deleting = false;
        if let Err(error) = result {
            self.error = Some(ServerSettingsError::DeleteCredentials(error));
        }
    }
}

impl App {
    fn open_connection(&mut self, root: &adw::ApplicationWindow, sender: &ComponentSender<Self>) {
        if let Some(connection) = self.connection.take() {
            self.workspace
                .connection_content
                .remove(connection.widget());
        }

        let Some(index) = self.active else {
            return;
        };

        let server = self.settings.servers()[index].clone();
        let name = server.name.clone();
        let connection = Connection::builder()
            .launch((
                root.clone(),
                server,
                self.workspace.overview.clone(),
                self.workspace.configurations.clone(),
                self.workspace.nodes.clone(),
                self.workspace.groups.clone(),
                self.workspace.routing.clone(),
            ))
            .forward(sender.input_sender(), move |event| {
                AppMsg::Connection(name.clone(), event)
            });

        self.workspace
            .connection_content
            .append(connection.widget());
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
