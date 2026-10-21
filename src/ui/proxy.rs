use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use crate::{
    api::Api,
    error::{ApiError, Message, ProxyError},
    ui::error_dialog::DialogExt,
};

#[derive(Clone, Copy, Debug)]
enum Operation {
    Refresh,
    Set(bool),
}

pub struct Proxy {
    api: Api,
    running: Option<bool>,
    pending: Option<Operation>,
    error: Option<ProxyError>,
    error_dialog: adw::AlertDialog,
    overview_status: adw::ActionRow,
}

#[derive(Debug)]
pub enum ProxyMsg {
    Refresh,
    SetRunning(bool),
    Retranslate,
    ShowError,
}

#[derive(Debug)]
pub struct ProxyUpdate {
    running: Result<bool, ApiError>,
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
                    set_sensitive: model.pending.is_none() && model.running.is_some(),
                    #[watch]
                    #[block_signal(switch_changed)]
                    set_active: model.running.unwrap_or(false),
                    connect_active_notify[sender] => move |row| {
                        sender.input(ProxyMsg::SetRunning(row.is_active()));
                    } @switch_changed,
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
            running: None,
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
            ProxyMsg::Refresh => Operation::Refresh,
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
                Operation::Refresh => None,
            };

            ProxyUpdate {
                running: api.proxy_running().await,
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
        let expired = matches!(&result.running, Err(ApiError::Unauthorized))
            || matches!(&result.change_error, Some(ApiError::Unauthorized));

        self.pending = None;
        self.error = result.change_error.map(ProxyError::Change);

        match result.running {
            Ok(running) => self.running = Some(running),
            Err(error) => {
                self.running = None;
                tracing::warn!(endpoint = %self.api.endpoint(), %error, "Could not read proxy status");
                if self.error.is_none() {
                    self.error = Some(ProxyError::Read(error));
                }
            }
        }

        if let Some(ProxyError::Change(error)) = &self.error {
            tracing::warn!(endpoint = %self.api.endpoint(), %error, "Could not change proxy status");
        }

        if expired {
            sender.output(()).unwrap();
        } else if matches!(self.error, Some(ProxyError::Change(_))) {
            self.show_error(root);
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
        let (ProxyError::Read(source) | ProxyError::Change(source)) = error;

        self.error_dialog
            .show_error(&error.user_message(), source, parent);
    }

    fn status_text(&self) -> String {
        match self.pending {
            Some(Operation::Refresh) => t!("proxy.checking"),
            Some(Operation::Set(true)) => t!("proxy.starting"),
            Some(Operation::Set(false)) => t!("proxy.stopping"),
            None => match self.running {
                Some(true) => t!("proxy.running"),
                Some(false) => t!("proxy.stopped"),
                None => t!("proxy.unknown"),
            },
        }
        .to_string()
    }
}
