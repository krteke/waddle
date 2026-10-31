use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use crate::{
    api::{Api, DaeStatus},
    error::{ApiError, Message, ProxyError},
    ui::error_dialog::DialogExt,
};

#[derive(Clone, Copy, Debug)]
enum Operation {
    Refresh,
    Set(bool),
    Reload,
}

pub struct Proxy {
    api: Api,
    status: Option<DaeStatus>,
    refresh_pending: bool,
    pending: Option<Operation>,
    error: Option<ProxyError>,
    error_dialog: adw::AlertDialog,
    overview_status: adw::ActionRow,
}

#[derive(Debug)]
pub enum ProxyMsg {
    Refresh,
    SetRunning(bool),
    Reload,
    Retranslate,
    ShowError,
}

#[derive(Debug)]
pub struct ProxyUpdate {
    status: Result<DaeStatus, ApiError>,
    change_error: Option<ApiError>,
}

#[relm4::component(pub)]
impl Component for Proxy {
    type Init = (Api, adw::ActionRow);
    type Input = ProxyMsg;
    type Output = ();
    type CommandOutput = ProxyUpdate;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 4,
            gtk::ListBox {
                add_css_class: "boxed-list",
                set_selection_mode: gtk::SelectionMode::None,
                adw::SwitchRow {
                    #[watch]
                    set_title: t!("proxy.service").as_ref(),
                    #[watch]
                    set_subtitle: &model.status_text(),
                    #[watch]
                    set_sensitive: model.pending.is_none() && model.status.is_some(),
                    #[watch]
                    #[block_signal(switch_changed)]
                    set_active: model.status.as_ref().is_some_and(|s| s.running),
                    connect_active_notify[sender] => move |row| {
                        sender.input(ProxyMsg::SetRunning(row.is_active()));
                    } @switch_changed,
                },
                adw::ActionRow {
                    set_title_lines: 0,
                    #[watch]
                    set_title: t!("proxy.pending_changes").as_ref(),
                    #[watch]
                    set_visible: model.status.as_ref().is_some_and(|s| s.modified),
                    add_suffix = &gtk::Button {
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_label: t!("proxy.reload").as_ref(),
                        #[watch]
                        set_tooltip_text: Some(t!("proxy.reload_config").as_ref()),
                        #[watch]
                        set_sensitive: model.pending.is_none(),
                        connect_clicked => ProxyMsg::Reload,
                    },
                },
            },
            gtk::Box {
                set_spacing: 6,
                gtk::Label {
                    set_xalign: 0.0,
                    set_hexpand: true,
                    set_wrap: true,
                    add_css_class: "error",
                    add_css_class: "caption",
                    #[watch]
                    set_label: &model.error.as_ref().map(|e| e.user_message()).unwrap_or_default(),
                },
                gtk::Button {
                    set_icon_name: "dialog-information-symbolic",
                    add_css_class: "flat",
                    set_valign: gtk::Align::Start,
                    #[watch]
                    set_visible: model.error.is_some(),
                    #[watch]
                    set_tooltip_text: Some(t!("error.details").as_ref()),
                    #[watch]
                    update_property: &[gtk::accessible::Property::Label(t!("error.details").as_ref())],
                    connect_clicked => ProxyMsg::ShowError,
                },
                gtk::Button {
                    set_icon_name: "view-refresh-symbolic",
                    add_css_class: "flat",
                    set_valign: gtk::Align::Start,
                    #[watch]
                    set_sensitive: model.pending.is_none(),
                    #[watch]
                    set_tooltip_text: Some(t!("proxy.refresh").as_ref()),
                    #[watch]
                    update_property: &[gtk::accessible::Property::Label(t!("proxy.refresh").as_ref())],
                    connect_clicked => ProxyMsg::Refresh,
                },
            },
        }
    }

    fn init(
        (api, overview_status): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let error_dialog = adw::AlertDialog::new(None, None);
        error_dialog.add_response("close", t!("dialog.close").as_ref());
        error_dialog.set_close_response("close");
        error_dialog.set_default_response(Some("close"));
        let model = Self {
            api,
            status: None,
            refresh_pending: false,
            pending: None,
            error: None,
            error_dialog,
            overview_status,
        };

        let widgets = view_output!();
        sender.input(ProxyMsg::Refresh);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        let operation = match message {
            ProxyMsg::Refresh => {
                if self.pending.is_some() {
                    self.refresh_pending = true;
                    return;
                }
                Operation::Refresh
            }
            ProxyMsg::Reload => Operation::Reload,
            ProxyMsg::SetRunning(running) => Operation::Set(running),
            ProxyMsg::Retranslate => return,
            ProxyMsg::ShowError => {
                self.show_error(root);
                return;
            }
        };

        if self.pending.is_some() {
            return;
        }

        self.pending = Some(operation);
        self.error = None;

        let api = self.api.clone();

        sender.oneshot_command(async move {
            let change_error = match operation {
                Operation::Set(running) => api.set_proxy_running(running).await.err(),
                Operation::Reload => api.set_proxy_running(true).await.err(),
                Operation::Refresh => None,
            };

            ProxyUpdate {
                status: api.proxy_status().await,
                change_error,
            }
        });
    }

    fn update_cmd(
        &mut self,
        result: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        let expired = matches!(&result.status, Err(ApiError::Unauthorized))
            || matches!(&result.change_error, Some(ApiError::Unauthorized));

        let operation = self.pending.take().unwrap();
        self.error = result.change_error.map(|error| match operation {
            Operation::Reload => ProxyError::Reload(error),
            _ => ProxyError::Change(error),
        });

        match result.status {
            Ok(status) => self.status = Some(status),
            Err(error) => {
                self.status = None;
                tracing::warn!(endpoint = %self.api.endpoint(), %error, "Could not read proxy status");
                if self.error.is_none() {
                    self.error = Some(ProxyError::Read(error));
                }
            }
        }

        if let Some(ProxyError::Change(error) | ProxyError::Reload(error)) = &self.error {
            tracing::warn!(endpoint = %self.api.endpoint(), %error, "Could not change proxy status");
        }

        if expired {
            sender.output(()).unwrap();
        } else if matches!(
            self.error,
            Some(ProxyError::Change(_) | ProxyError::Reload(_))
        ) {
            self.show_error(root);
        }
        if std::mem::take(&mut self.refresh_pending) && !expired {
            sender.input(ProxyMsg::Refresh);
        }
    }

    fn post_view() {
        self.overview_status.set_subtitle(&self.status_text());
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        if self.error_dialog.parent().is_some() {
            self.error_dialog.close();
        }
    }
}

impl Proxy {
    fn show_error(&self, parent: &gtk::Box) {
        let Some(error) = &self.error else { return };
        let (ProxyError::Read(source) | ProxyError::Change(source) | ProxyError::Reload(source)) =
            error;

        self.error_dialog
            .show_error(&error.user_message(), source, parent);
    }

    fn status_text(&self) -> String {
        match self.pending {
            Some(Operation::Refresh) => t!("proxy.checking"),
            Some(Operation::Set(true)) => t!("proxy.starting"),
            Some(Operation::Set(false)) => t!("proxy.stopping"),
            Some(Operation::Reload) => t!("proxy.reloading"),
            None => match self.status.as_ref().map(|s| s.running) {
                Some(true) => t!("proxy.running"),
                Some(false) => t!("proxy.stopped"),
                None => t!("proxy.unknown"),
            },
        }
        .to_string()
    }
}
