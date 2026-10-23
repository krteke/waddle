use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use crate::{
    api::{Api, overview::Summary},
    error::ApiError,
    ui::error_dialog::DialogExt,
};

pub struct Overview {
    api: Api,
    username: String,
    pub proxy: adw::ActionRow,
    summary: Option<Summary>,
    busy: bool,
    error: Option<ApiError>,
    error_dialog: adw::AlertDialog,
}

#[derive(Debug)]
pub enum OverviewMsg {
    Refresh,
    Retranslate,
    SignOut,
    ShowError,
}
#[derive(Debug)]
pub enum OverviewOutput {
    SignOut,
    Unauthorized,
}

#[relm4::component(pub)]
impl Component for Overview {
    type Init = (Api, String);
    type Input = OverviewMsg;
    type Output = OverviewOutput;
    type CommandOutput = Result<Summary, ApiError>;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            connect_map => OverviewMsg::Refresh,
            adw::Clamp {
                set_maximum_size: 720,
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 24,
                    set_margin_all: 24,
                    gtk::Box {
                        set_spacing: 12,
                        gtk::Label {
                            set_xalign: 0.0,
                            set_hexpand: true,
                            add_css_class: "title-1",
                            #[watch]
                            set_label: &t!("navigation.overview"),
                        },
                        gtk::Spinner {
                            #[watch]
                            set_spinning: model.busy,
                            #[watch]
                            set_visible: model.busy,
                        },
                        gtk::Button {
                            set_icon_name: "view-refresh-symbolic",
                            add_css_class: "flat",
                            #[watch]
                            set_tooltip_text: Some(t!("overview.refresh").as_ref()),
                            #[watch]
                            set_sensitive: !model.busy,
                            connect_clicked => OverviewMsg::Refresh,
                        },
                    },
                    adw::Banner {
                        #[watch]
                        set_visible: model.error.is_some(),
                        #[watch]
                        set_title: &t!("overview.load_error"),
                        #[watch]
                        set_revealed: model.error.is_some(),
                        #[watch]
                        set_button_label: Some(t!("error.details").as_ref()),
                        connect_button_clicked => OverviewMsg::ShowError,
                    },
                    adw::PreferencesGroup {
                        #[local_ref]
                        add = proxy -> adw::ActionRow {
                            #[watch]
                            set_title: &t!("proxy.status"),
                        },
                        adw::ActionRow {
                            set_use_markup: false,
                            #[watch]
                            set_title: &t!("auth.status"),
                            #[watch]
                            set_subtitle: &t!("auth.signed_in", username = &model.username),
                            add_suffix = &gtk::Button {
                                set_icon_name: "system-log-out-symbolic",
                                set_valign: gtk::Align::Center,
                                #[watch]
                                set_tooltip_text: Some(t!("auth.sign_out").as_ref()),
                                connect_clicked => OverviewMsg::SignOut,
                            },
                        },
                    },
                    adw::PreferencesGroup {
                        #[watch]
                        set_title: &t!("overview.selection"),
                        adw::ActionRow {
                            set_use_markup: false,
                            #[watch]
                            set_title: &t!("navigation.configurations"),
                            #[watch]
                            set_subtitle: model.summary.as_ref().map(|s| s.configs.first().map(|c| c.name.clone()).unwrap_or_else(|| t!("overview.none_selected").into_owned())).as_deref().unwrap_or("—"),
                        },
                        adw::ActionRow {
                            set_use_markup: false,
                            #[watch]
                            set_title: &t!("navigation.dns"),
                            #[watch]
                            set_subtitle: model.summary.as_ref().map(|s| s.dnss.first().map(|c| c.name.clone()).unwrap_or_else(|| t!("overview.none_selected").into_owned())).as_deref().unwrap_or("—"),
                        },
                        adw::ActionRow {
                            set_use_markup: false,
                            #[watch]
                            set_title: &t!("navigation.routing"),
                            #[watch]
                            set_subtitle: model.summary.as_ref().map(|s| s.routings.first().map(|c| c.name.clone()).unwrap_or_else(|| t!("overview.none_selected").into_owned())).as_deref().unwrap_or("—"),
                        },
                    },
                    adw::PreferencesGroup {
                        #[watch]
                        set_title: &t!("overview.resources"),
                        adw::ActionRow {
                            #[watch]
                            set_title: &t!("navigation.nodes"),
                            add_suffix = &gtk::Label {
                                add_css_class: "numeric",
                                #[watch]
                                set_label: &model.summary.as_ref().map(|s| s.nodes.total_count.to_string()).unwrap_or_else(|| "—".into()),
                            },
                        },
                        adw::ActionRow {
                            #[watch]
                            set_title: &t!("navigation.groups"),
                            add_suffix = &gtk::Label {
                                add_css_class: "numeric",
                                #[watch]
                                set_label: &model.summary.as_ref().map(|s| s.groups.len().to_string()).unwrap_or_else(|| "—".into()),
                            },
                        },
                        adw::ActionRow {
                            #[watch]
                            set_title: &t!("navigation.subscriptions"),
                            add_suffix = &gtk::Label {
                                add_css_class: "numeric",
                                #[watch]
                                set_label: &model.summary.as_ref().map(|s| s.subscriptions.len().to_string()).unwrap_or_else(|| "—".into()),
                            },
                        },
                    },
                    gtk::Label {
                        set_xalign: 0.0,
                        set_wrap: true,
                        add_css_class: "dim-label",
                        add_css_class: "caption",
                        #[watch]
                        set_visible: model.summary.is_some(),
                        #[watch]
                        set_label: &model.summary.as_ref().map(|s| t!("overview.core_version", version = &s.general.dae.version)).unwrap_or_default(),
                    },
                },
            },
        }
    }

    fn init(
        (api, username): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let error_dialog = adw::AlertDialog::new(None, None);
        error_dialog.add_response("close", &t!("dialog.close"));

        let model = Self {
            api,
            username,
            proxy: adw::ActionRow::new(),
            summary: None,
            busy: false,
            error: None,
            error_dialog,
        };

        let proxy = &model.proxy;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: OverviewMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            OverviewMsg::Refresh => {
                if self.busy {
                    return;
                }

                self.busy = true;
                self.error = None;

                let api = self.api.clone();

                sender.oneshot_command(async move { api.overview().await });
            }
            OverviewMsg::SignOut => {
                sender.output(OverviewOutput::SignOut).unwrap();
            }
            OverviewMsg::Retranslate => {}
            OverviewMsg::ShowError => {
                if let Some(error) = &self.error {
                    self.error_dialog
                        .show_error(&t!("overview.load_error"), error, root);
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        result: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.busy = false;

        match result {
            Ok(summary) => self.summary = Some(summary),
            Err(ApiError::Unauthorized) => {
                sender.output(OverviewOutput::Unauthorized).unwrap();
            }
            Err(error) => {
                tracing::warn!(%error, "Could not load overview");
                self.summary = None;
                self.error = Some(error);
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        if self.error_dialog.parent().is_some() {
            self.error_dialog.force_close();
        }
    }
}
