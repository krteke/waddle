use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use super::fields::Fields;
use crate::{
    api::{Api, config::Config},
    error::ApiError,
    ui::error_dialog::DialogExt,
};

pub struct Editor {
    api: Api,
    creating: bool,
    original: Option<Config>,
    fields: Fields,
    busy: bool,
    dirty: bool,
    submitted: bool,
    error: Option<ApiError>,
    error_dialog: adw::AlertDialog,
}

#[derive(Debug)]
pub enum EditorMsg {
    LoadDefaults,
    Save,
    Reset,
    Cancel,
    Changed,
    ShowError,
    Retranslate,
}

#[derive(Clone, Debug)]
pub enum EditorOutput {
    Saved,
    Closed,
    Unauthorized,
}

#[derive(Debug)]
pub enum EditorReply {
    Defaults(Result<Box<Config>, ApiError>),
    Saved(Result<(), ApiError>),
}

#[relm4::component(pub)]
impl Component for Editor {
    type Init = (Api, Option<Config>);
    type Input = EditorMsg;
    type Output = EditorOutput;
    type CommandOutput = EditorReply;

    view! {
        #[name = "dialog"]
        adw::Dialog {
            set_content_width: 720,
            set_content_height: 720,
            #[watch]
            set_title: if model.creating { t!("config.new_title") } else { t!("config.edit") }.as_ref(),
            #[watch]
            set_can_close: !model.busy || model.original.is_none(),
            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        #[watch]
                        set_label: t!("dialog.cancel").as_ref(),
                        #[watch]
                        set_sensitive: !model.busy || model.original.is_none(),
                        connect_clicked => EditorMsg::Cancel,
                    },
                    pack_end = &gtk::Button {
                        add_css_class: "suggested-action",
                        #[watch]
                        set_label: t!("dialog.save").as_ref(),
                        #[watch]
                        set_sensitive: !model.busy && model.original.is_some(),
                        connect_clicked => EditorMsg::Save,
                    },
                },
                add_top_bar = &adw::Banner {
                    #[watch]
                    set_title: if model.original.is_none() { t!("config.defaults_error") } else { t!("config.save_error") }.as_ref(),
                    #[watch]
                    set_revealed: model.error.is_some(),
                    #[watch]
                    set_button_label: Some(t!("error.details").as_ref()),
                    connect_button_clicked => EditorMsg::ShowError,
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    #[local_ref]
                    page -> adw::PreferencesPage {
                        set_vexpand: true,
                        #[watch]
                        set_sensitive: !model.busy,
                        #[watch]
                        set_visible: model.original.is_some(),
                    },
                    gtk::Spinner {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_vexpand: true,
                        #[watch]
                        set_spinning: model.busy,
                        #[watch]
                        set_visible: model.original.is_none() && model.busy,
                    },
                    adw::StatusPage {
                        set_icon_name: Some("dialog-warning-symbolic"),
                        #[watch]
                        set_title: t!("config.defaults_error").as_ref(),
                        #[watch]
                        set_visible: model.original.is_none() && !model.busy,
                        gtk::Button {
                            set_halign: gtk::Align::Center,
                            #[watch]
                            set_label: t!("auth.retry").as_ref(),
                            connect_clicked => EditorMsg::LoadDefaults,
                        },
                    },
                },
                add_bottom_bar = &gtk::Box {
                    set_margin_all: 12,
                    set_spacing: 12,
                    gtk::Button {
                        #[watch]
                        set_label: t!("config.reset").as_ref(),
                        #[watch]
                        set_sensitive: !model.busy && model.dirty,
                        connect_clicked => EditorMsg::Reset,
                    },
                    gtk::Spinner {
                        #[watch]
                        set_spinning: model.busy,
                        #[watch]
                        set_visible: model.busy && model.original.is_some(),
                    },
                },
            },
        }
    }

    fn init(
        (api, config): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let fields = Fields::new(sender.input_sender());

        if let Some(config) = &config {
            fields.load(config);
            fields.update_conditions();
        }

        let error_dialog = adw::AlertDialog::new(None, None);

        error_dialog.add_response("close", t!("dialog.close").as_ref());
        error_dialog.set_close_response("close");

        let model = Self {
            api,
            creating: config.is_none(),
            original: config,
            fields,
            busy: false,
            dirty: false,
            submitted: false,
            error: None,
            error_dialog,
        };

        let page = &model.fields.page;
        let widgets = view_output!();

        root.connect_closed({
            let sender = sender.clone();

            move |_| {
                let _ = sender.output(EditorOutput::Closed);
            }
        });

        if model.creating {
            sender.input(EditorMsg::LoadDefaults);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: EditorMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            EditorMsg::Changed => {
                self.fields.update_conditions();

                self.dirty = self
                    .original
                    .as_ref()
                    .is_some_and(|original| self.fields.read(original).as_ref() != Some(original));

                if self.submitted {
                    self.fields.validate(false);
                }
            }
            EditorMsg::Retranslate => self.fields.retranslate(),
            EditorMsg::Cancel => {
                root.close();
            }
            EditorMsg::Reset => {
                self.fields.load(self.original.as_ref().unwrap());
                self.error = None;
                self.submitted = false;
                self.dirty = false;
                self.fields.clear_errors();
            }
            EditorMsg::ShowError => self.show_error(root),
            EditorMsg::LoadDefaults => {
                self.busy = true;
                self.error = None;

                let api = self.api.clone();

                sender.oneshot_command(async move {
                    EditorReply::Defaults(api.default_config().await.map(Box::new))
                });
            }
            EditorMsg::Save => {
                if self.busy {
                    return;
                }

                self.submitted = true;

                if !self.fields.validate(true) {
                    return;
                }

                let original = self.original.as_ref().unwrap();
                let config = self.fields.read(original).unwrap();

                let original_name = (!self.creating).then(|| original.name.clone());

                self.busy = true;
                self.error = None;

                let api = self.api.clone();

                sender.oneshot_command(async move {
                    EditorReply::Saved(api.save_config(config, original_name.as_deref()).await)
                });
            }
        }
    }

    fn update_cmd(&mut self, reply: EditorReply, sender: ComponentSender<Self>, root: &Self::Root) {
        self.busy = false;

        match reply {
            EditorReply::Defaults(Ok(config)) => {
                self.fields.load(&config);
                self.original = Some(*config);
            }
            EditorReply::Saved(Ok(())) => {
                sender.output(EditorOutput::Saved).unwrap();
            }
            EditorReply::Defaults(Err(error)) | EditorReply::Saved(Err(error)) => {
                tracing::warn!(endpoint = %self.api.endpoint(), %error, "Config editor request failed");

                if matches!(error, ApiError::Unauthorized) {
                    sender.output(EditorOutput::Unauthorized).unwrap();
                } else {
                    self.error = Some(error);
                    self.show_error(root);
                }
            }
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        if self.error_dialog.parent().is_some() {
            self.error_dialog.force_close();
        }

        if widgets.dialog.parent().is_some() {
            widgets.dialog.force_close();
        }
    }
}

impl Editor {
    fn show_error(&self, parent: &adw::Dialog) {
        if let Some(error) = &self.error {
            let heading = if self.original.is_none() {
                t!("config.defaults_error")
            } else {
                t!("config.save_error")
            };

            self.error_dialog
                .show_error(heading.as_ref(), error, parent);
        }
    }
}
