use std::net::SocketAddr;

use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use super::login_dialog::LoginDialog;
use crate::{
    api::Api,
    credentials::{self, Credentials},
    error::{ApiError, ConnectionError},
    settings::Server,
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub enum AuthResult {
    Prompt {
        create_admin: bool,
        username: String,
        error: Option<ConnectionError>,
    },
    Connected {
        api: Api,
        username: String,
        warning: Option<ConnectionError>,
    },
    Failed(ConnectionError),
}

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
    server: Server,
    parent: adw::ApplicationWindow,
    api: Option<Api>,
    phase: Phase,
    error: Option<ConnectionError>,
    login: LoginDialog,
}

#[derive(Debug)]
pub enum ConnectionMsg {
    Retry,
    Submit,
    Retranslate,
}

#[relm4::component(pub)]
impl Component for Connection {
    type Init = (adw::ApplicationWindow, Server);
    type Input = ConnectionMsg;
    type Output = ();
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
                    Phase::Connected { .. } => t!("server.connected", name = &model.server.name).to_string(),
                    Phase::InvalidAddress => t!("server.invalid_address_title", name = &model.server.name).to_string(),
                    _ => model.server.name.clone(),
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
                        connect_clicked => ConnectionMsg::Retry,
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
            server,
            parent,
            api,
            phase,
            error: None,
            login,
        };

        let widgets = view_output!();
        if model.api.is_some() {
            sender.input(ConnectionMsg::Retry);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            ConnectionMsg::Retry => {
                self.phase = Phase::Checking;
                self.error = None;
                let api = self.api.as_ref().unwrap().clone();
                let server_name = self.server.name.clone();
                sender.oneshot_command(async move { prepare(api, &server_name).await });
            }
            ConnectionMsg::Submit => {
                let credentials = self.login.credentials();
                let create_admin = self.login.create_admin;
                self.phase = Phase::SigningIn;
                self.error = None;
                let api = self.api.as_ref().unwrap().clone();
                let server_name = self.server.name.clone();
                sender.oneshot_command(async move {
                    authenticate(api, &server_name, credentials, create_admin).await
                });
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
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match result {
            AuthResult::Prompt {
                create_admin,
                username,
                error,
            } => {
                if matches!(self.phase, Phase::SigningIn) {
                    self.login.clear_password();
                } else {
                    self.login.present(&self.parent, create_admin, &username);
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
                self.login.close();
            }
            AuthResult::Failed(error) => {
                self.phase = Phase::Failed;
                self.error = Some(error);
            }
        }

        if let Some(error) = &self.error {
            tracing::warn!(server = %self.server.name, error = %error, "Server connection");
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

async fn prepare(api: Api, server_name: &str) -> AuthResult {
    if let Err(error) = api.health_check().await {
        return AuthResult::Failed(ConnectionError::HealthCheck(error));
    }

    match api.has_admin().await {
        Ok(false) => AuthResult::Prompt {
            create_admin: true,
            username: String::new(),
            error: None,
        },
        Ok(true) => match credentials::load(server_name).await {
            Ok(Some(credentials)) => authenticate(api, server_name, credentials, false).await,
            Ok(None) => AuthResult::Prompt {
                create_admin: false,
                username: String::new(),
                error: None,
            },
            Err(error) => AuthResult::Prompt {
                create_admin: false,
                username: String::new(),
                error: Some(ConnectionError::ReadCredentials(error)),
            },
        },
        Err(error) => AuthResult::Failed(ConnectionError::Check(error)),
    }
}

async fn authenticate(
    api: Api,
    server_name: &str,
    credentials: Credentials,
    create_admin: bool,
) -> AuthResult {
    let result = async {
        let api = api.authenticate(&credentials, create_admin).await?;
        let username = api.current_user().await?;
        Ok::<_, ApiError>((api, username))
    }
    .await;

    match result {
        Ok((api, username)) => {
            let warning = credentials
                .save(server_name)
                .await
                .err()
                .map(ConnectionError::SaveCredentials);
            AuthResult::Connected {
                api,
                username,
                warning,
            }
        }
        Err(error) => AuthResult::Prompt {
            create_admin,
            username: credentials.username().to_owned(),
            error: Some(if create_admin {
                ConnectionError::Create(error)
            } else {
                ConnectionError::Login(error)
            }),
        },
    }
}
