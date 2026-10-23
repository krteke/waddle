use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use super::{
    config::{ConfigMsg, Configs},
    groups::{GroupMsg, Groups},
    login_dialog::LoginDialog,
    nodes::{NodeMsg, Nodes},
    overview::Overview,
    proxy::{Proxy, ProxyMsg},
};
use crate::{
    api::Api,
    auth::{self, AuthResult},
    error::{ConnectionError, Message},
    settings::Server,
};

#[derive(Debug)]
enum Phase {
    InvalidAddress,
    Checking,
    AwaitingCredentials,
    SigningIn,
    Connected { api: Api, username: String },
    Failed,
}

pub struct Connection {
    server_name: String,
    parent: adw::ApplicationWindow,
    api: Option<Api>,
    phase: Phase,
    error: Option<ConnectionError>,
    login: LoginDialog,
    overview: Overview,
    overview_stack: gtk::Stack,
    proxy: Option<Controller<Proxy>>,
    proxy_content: gtk::Box,
    configs: Option<Controller<Configs>>,
    config_stack: gtk::Stack,
    nodes: Option<Controller<Nodes>>,
    node_stack: gtk::Stack,
    groups: Option<Controller<Groups>>,
    group_stack: gtk::Stack,
}

#[derive(Debug)]
pub enum ConnectionMsg {
    Retry(bool),
    Submit,
    SignOut,
    Retranslate,
    RefreshSession,
}

#[derive(Debug)]
pub enum ConnectionOutput {
    Retry,
    SignOut,
    SignedIn,
}

#[relm4::component(pub)]
impl Component for Connection {
    type Init = (
        adw::ApplicationWindow,
        Server,
        gtk::Stack,
        gtk::Stack,
        gtk::Stack,
        gtk::Stack,
    );
    type Input = ConnectionMsg;
    type Output = ConnectionOutput;
    type CommandOutput = AuthResult;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,
            gtk::Box {
                set_spacing: 8,
                gtk::Spinner {
                    #[watch]
                    set_spinning: matches!(model.phase, Phase::Checking | Phase::SigningIn),
                    #[watch]
                    set_visible: matches!(model.phase, Phase::Checking | Phase::SigningIn),
                },
                gtk::Image {
                    #[watch]
                    set_icon_name: Some(if matches!(model.phase, Phase::Connected { .. }) { "object-select-symbolic" } else { "network-offline-symbolic" }),
                    #[watch]
                    set_css_classes: if matches!(model.phase, Phase::Connected { .. }) { &["success"] } else { &["dim-label"] },
                    #[watch]
                    set_visible: !matches!(model.phase, Phase::Checking | Phase::SigningIn),
                },
                gtk::Label {
                    add_css_class: "caption",
                    set_xalign: 0.0,
                    set_hexpand: true,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    #[watch]
                    set_label: match &model.phase {
                        Phase::Connected { .. } => t!("auth.connected"),
                        Phase::InvalidAddress => t!("server.invalid_url"),
                        Phase::Checking => t!("auth.checking"),
                        Phase::SigningIn => t!("auth.signing_in"),
                        _ => t!("auth.disconnected"),
                    }.as_ref(),
                    #[watch]
                    set_tooltip_text: match &model.phase {
                        Phase::Connected { api, username } => Some(format!("{}\n{}", t!("auth.signed_in", username = username), api.endpoint())),
                        _ => None,
                    }.as_deref(),
                },
            },
            gtk::Label {
                set_xalign: 0.0,
                set_wrap: true,
                add_css_class: "error",
                #[watch]
                set_visible: model.error.is_some(),
                #[watch]
                set_label: &model.error.as_ref().map(|e| e.user_message()).unwrap_or_default(),
            },
            gtk::Button {
                set_halign: gtk::Align::Start,
                #[watch]
                set_label: if matches!(model.phase, Phase::AwaitingCredentials) { t!("auth.login") } else { t!("auth.retry") }.as_ref(),
                #[watch]
                set_visible: matches!(model.phase, Phase::AwaitingCredentials | Phase::Failed),
                connect_clicked[sender] => move |_| { sender.output(ConnectionOutput::Retry).unwrap(); },
            },
            #[local_ref]
            proxy_content -> gtk::Box {
                #[watch]
                set_visible: matches!(model.phase, Phase::Connected { .. }),
            },
        }
    }

    fn init(
        (parent, server, overview_stack, config_stack, node_stack, group_stack): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let api = server.url.as_ref().ok().cloned().map(Api::new);

        let phase = if api.is_some() {
            Phase::Checking
        } else {
            Phase::InvalidAddress
        };

        let login = LoginDialog::new(&sender, &server.name);
        let overview = Overview::new(&sender);
        overview_stack.add_child(&overview.content);
        let model = Self {
            server_name: server.name,
            parent,
            api,
            phase,
            error: None,
            login,
            overview,
            overview_stack,
            proxy: None,
            proxy_content: gtk::Box::new(gtk::Orientation::Vertical, 0),
            configs: None,
            config_stack,
            nodes: None,
            node_stack,
            groups: None,
            group_stack,
        };

        let proxy_content = &model.proxy_content;
        let widgets = view_output!();
        if model.api.is_some() {
            sender.input(ConnectionMsg::Retry(server.logged_out));
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        if matches!(
            message,
            ConnectionMsg::Retry(_) | ConnectionMsg::Submit | ConnectionMsg::SignOut
        ) {
            if let Some(proxy) = self.proxy.take() {
                self.proxy_content.remove(proxy.widget());
            }

            if let Some(configs) = self.configs.take() {
                self.config_stack.remove(configs.widget());
            }

            self.config_stack.set_visible_child_name("placeholder");
            if let Some(nodes) = self.nodes.take() {
                self.node_stack.remove(nodes.widget());
            }

            self.node_stack.set_visible_child_name("placeholder");
            if let Some(groups) = self.groups.take() {
                self.group_stack.remove(groups.widget());
            }

            self.group_stack.set_visible_child_name("placeholder");
        }

        match message {
            ConnectionMsg::Retry(logged_out) => {
                self.phase = Phase::Checking;
                self.error = None;
                let api = self.api.as_ref().unwrap().clone();
                let server_name = self.server_name.clone();
                sender.oneshot_command(async move { api.prepare(&server_name, logged_out).await });
            }
            ConnectionMsg::Submit => {
                let credentials = self.login.credentials();
                let create_admin = self.login.create_admin;
                let save_password = self.login.save_password();
                self.phase = Phase::SigningIn;
                self.error = None;
                let api = self.api.as_ref().unwrap().clone();
                let server_name = self.server_name.clone();
                sender.oneshot_command(async move {
                    auth::authenticate(&api, &server_name, credentials, create_admin, save_password)
                        .await
                });
            }
            ConnectionMsg::SignOut => {
                self.phase = Phase::Checking;
                self.error = None;
                let server_name = self.server_name.clone();
                sender.oneshot_command(async move { auth::sign_out(&server_name).await });
            }
            ConnectionMsg::Retranslate => {
                if let Some(groups) = &self.groups {
                    groups.emit(GroupMsg::Retranslate);
                }
                if let Some(nodes) = &self.nodes {
                    nodes.emit(NodeMsg::Retranslate);
                }
                if let Some(configs) = &self.configs {
                    configs.emit(ConfigMsg::Retranslate);
                }

                if let Some(proxy) = &self.proxy {
                    proxy.emit(ProxyMsg::Retranslate);
                }
            }
            ConnectionMsg::RefreshSession => {
                sender.output(ConnectionOutput::Retry).unwrap();
            }
        }

        self.login.refresh(
            matches!(self.phase, Phase::SigningIn),
            self.error.as_ref().map(|e| e.user_message()).as_deref(),
        );
    }

    fn update_cmd(
        &mut self,
        result: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match result {
            AuthResult::Prompt {
                create_admin,
                credentials,
                save_password,
                error,
            } => {
                if matches!(self.phase, Phase::SigningIn) {
                    self.login.clear_password();
                } else {
                    self.login
                        .present(&self.parent, create_admin, &credentials, save_password);
                }
                self.phase = Phase::AwaitingCredentials;
                self.error = error;
            }
            AuthResult::Connected {
                api,
                username,
                warning,
            } => {
                let proxy = Proxy::builder()
                    .launch((api.clone(), self.overview.proxy.clone()))
                    .forward(sender.input_sender(), |()| ConnectionMsg::RefreshSession);
                self.proxy_content.append(proxy.widget());
                self.proxy = Some(proxy);

                let configs = Configs::builder()
                    .launch(api.clone())
                    .forward(sender.input_sender(), |()| ConnectionMsg::RefreshSession);

                self.config_stack.add_child(configs.widget());
                self.config_stack.set_visible_child(configs.widget());

                self.configs = Some(configs);
                let nodes = Nodes::builder()
                    .launch(api.clone())
                    .forward(sender.input_sender(), |()| ConnectionMsg::RefreshSession);

                self.node_stack.add_child(nodes.widget());
                self.node_stack.set_visible_child(nodes.widget());
                self.nodes = Some(nodes);

                let groups = Groups::builder()
                    .launch(api.clone())
                    .forward(sender.input_sender(), |()| ConnectionMsg::RefreshSession);

                self.group_stack.add_child(groups.widget());
                self.group_stack.set_visible_child(groups.widget());
                self.groups = Some(groups);

                self.phase = Phase::Connected { api, username };
                self.error = warning;

                if self.error.is_none() {
                    sender.output(ConnectionOutput::SignedIn).unwrap();
                }

                self.login.close();
            }
            AuthResult::Failed(error) => {
                self.phase = Phase::Failed;
                self.error = Some(error);
            }
        }

        if let Some(error) = &self.error {
            tracing::warn!(server = %self.server_name, error = %error, "Server connection");
        }

        self.login.refresh(
            false,
            self.error.as_ref().map(|e| e.user_message()).as_deref(),
        );
    }

    fn post_view() {
        if let Phase::Connected { username, .. } = &self.phase {
            self.overview.refresh(username);
            self.overview_stack
                .set_visible_child(&self.overview.content);
        } else {
            self.overview_stack.set_visible_child_name("placeholder");
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        self.login.close();
        self.overview_stack.remove(&self.overview.content);
        if let Some(configs) = &self.configs {
            self.config_stack.remove(configs.widget());
        }
        self.config_stack.set_visible_child_name("placeholder");

        if let Some(nodes) = &self.nodes {
            self.node_stack.remove(nodes.widget());
        }

        self.node_stack.set_visible_child_name("placeholder");
        if let Some(groups) = &self.groups {
            self.group_stack.remove(groups.widget());
        }

        self.group_stack.set_visible_child_name("placeholder");
    }
}
