use std::net::SocketAddr;

use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use super::login_dialog::LoginDialog;
use crate::{
    api::Api,
    auth::{self, AuthResult},
    error::ConnectionError,
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
}

#[derive(Debug)]
pub enum ConnectionMsg {
    Retry(bool),
    Submit,
    SignOut,
    Retranslate,
}

#[derive(Debug)]
pub enum ConnectionOutput {
    Retry,
    SignOut,
    SignedIn,
}

#[relm4::component(pub)]
impl Component for Connection {
    type Init = (adw::ApplicationWindow, Server);
    type Input = ConnectionMsg;
    type Output = ConnectionOutput;
    type CommandOutput = AuthResult;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            adw::Banner {
                #[watch]
                set_revealed: model.error.is_some(),
                #[watch]
                set_title: &model.error.as_ref().map(ConnectionError::user_message).unwrap_or_default(),
            },
            adw::StatusPage {
                set_vexpand: true,
                set_icon_name: Some("network-server-symbolic"),
                #[watch]
                set_title: &match &model.phase {
                    Phase::Connected { .. } => t!("server.connected", name = &model.server_name).to_string(),
                    Phase::InvalidAddress => t!("server.invalid_address_title", name = &model.server_name).to_string(),
                    _ => model.server_name.clone(),
                },
                #[watch]
                set_description: Some(&match &model.phase {
                    Phase::InvalidAddress => t!("server.invalid_ip").to_string(),
                    Phase::Checking => t!("auth.checking").to_string(),
                    Phase::AwaitingCredentials => t!("auth.required").to_string(),
                    Phase::SigningIn => t!("auth.signing_in").to_string(),
                    Phase::Connected { api, username } => format!("{}\n{}", api.endpoint(), t!("auth.signed_in", username = gtk::glib::markup_escape_text(username))),
                    Phase::Failed => t!("auth.connection_error").to_string(),
                }),
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 16,
                    set_halign: gtk::Align::Center,
                    gtk::Spinner {
                        #[watch]
                        set_spinning: matches!(model.phase, Phase::Checking | Phase::SigningIn),
                        #[watch]
                        set_visible: matches!(model.phase, Phase::Checking | Phase::SigningIn),
                    },
                    gtk::Button {
                        add_css_class: "suggested-action",
                        add_css_class: "pill",
                        #[watch]
                        set_label: if matches!(model.phase, Phase::AwaitingCredentials) { t!("auth.login") } else { t!("auth.retry") }.as_ref(),
                        #[watch]
                        set_visible: matches!(model.phase, Phase::AwaitingCredentials | Phase::Failed),
                        connect_clicked[sender] => move |_| { sender.output(ConnectionOutput::Retry).unwrap(); },
                    },
                    gtk::Button {
                        #[watch]
                        set_label: t!("auth.sign_out").as_ref(),
                        #[watch]
                        set_visible: matches!(model.phase, Phase::Connected { .. }),
                        connect_clicked[sender] => move |_| { sender.output(ConnectionOutput::SignOut).unwrap(); },
                    },
                },
            },
        }
    }

    fn init(
        (parent, server): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let api = server
            .ip
            .as_ref()
            .ok()
            .map(|ip| Api::new(SocketAddr::new(*ip, server.port)));

        let phase = if api.is_some() {
            Phase::Checking
        } else {
            Phase::InvalidAddress
        };

        let login = LoginDialog::new(&sender, &server.name);
        let model = Self {
            server_name: server.name,
            parent,
            api,
            phase,
            error: None,
            login,
        };

        let widgets = view_output!();
        if model.api.is_some() {
            sender.input(ConnectionMsg::Retry(server.logged_out));
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            ConnectionMsg::Retry(logged_out) => {
                self.phase = Phase::Checking;
                self.error = None;
                let api = self.api.as_ref().unwrap().clone();
                let server_name = self.server_name.clone();
                sender.oneshot_command(async move {
                    auth::prepare(api, &server_name, logged_out).await
                });
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
                    auth::authenticate(api, &server_name, credentials, create_admin, save_password)
                        .await
                });
            }
            ConnectionMsg::SignOut => {
                self.phase = Phase::Checking;
                self.error = None;
                let server_name = self.server_name.clone();
                sender.oneshot_command(async move { auth::sign_out(&server_name).await });
            }
            ConnectionMsg::Retranslate => {}
        }

        self.login.refresh(
            matches!(self.phase, Phase::SigningIn),
            self.error
                .as_ref()
                .map(ConnectionError::user_message)
                .as_deref(),
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
            self.error
                .as_ref()
                .map(ConnectionError::user_message)
                .as_deref(),
        );
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        self.login.close();
    }
}
