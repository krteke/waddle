use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use super::fields::Fields;
use crate::{
    api::{
        Api,
        config::{Config, Interface},
    },
    error::ApiError,
    ui::error_dialog::DialogExt,
};

pub struct Editor {
    api: Api,
    creating: bool,
    original: Option<Config>,
    fields: Fields,
    busy: bool,
    loaded: bool,
    dirty: bool,
    submitted: bool,
    error: Option<ApiError>,
    error_dialog: adw::AlertDialog,
    discard_dialog: adw::AlertDialog,
}

#[derive(Debug)]
pub enum EditorMsg {
    Load,
    Save,
    Reset,
    Back,
    Discard,
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
    Loaded(Result<(Box<Config>, Vec<Interface>), ApiError>),
    Saved(Result<(), ApiError>),
}

#[relm4::component(pub)]
impl Component for Editor {
    type Init = (Api, Option<Config>);
    type Input = EditorMsg;
    type Output = EditorOutput;
    type CommandOutput = EditorReply;

    view! {
        adw::NavigationPage {
            set_tag: Some("config-editor"),
            #[watch]
            set_title: if model.creating { t!("config.new_title") } else { t!("config.edit") }.as_ref(),
            #[watch]
            set_can_pop: (!model.busy || !model.loaded) && !model.dirty,
            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    set_show_back_button: false,
                    pack_start = &gtk::Button {
                        #[watch]
                        set_tooltip_text: Some(t!("config.back").as_ref()),
                        set_icon_name: "go-previous-symbolic",
                        #[watch]
                        set_sensitive: !model.busy || !model.loaded,
                        connect_clicked => EditorMsg::Back,
                    },
                    pack_end = &gtk::Button {
                        add_css_class: "suggested-action",
                        #[watch]
                        set_label: t!("dialog.save").as_ref(),
                        #[watch]
                        set_sensitive: !model.busy && model.loaded && (model.creating || model.dirty),
                        connect_clicked => EditorMsg::Save,
                    },
                },
                add_top_bar = &adw::Banner {
                    #[watch]
                    set_title: if !model.loaded { t!("config.editor_load_error") } else { t!("config.save_error") }.as_ref(),
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
                        set_sensitive: !model.busy && model.loaded,
                        #[watch]
                        set_visible: model.loaded,
                    },
                    gtk::Spinner {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_vexpand: true,
                        #[watch]
                        set_spinning: model.busy,
                        #[watch]
                        set_visible: !model.loaded && model.busy,
                    },
                    adw::StatusPage {
                        set_icon_name: Some("dialog-warning-symbolic"),
                        #[watch]
                        set_title: t!("config.editor_load_error").as_ref(),
                        #[watch]
                        set_visible: !model.loaded && !model.busy,
                        gtk::Button {
                            set_halign: gtk::Align::Center,
                            #[watch]
                            set_label: t!("auth.retry").as_ref(),
                            connect_clicked => EditorMsg::Load,
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
                        set_visible: model.busy && model.loaded,
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

        let error_dialog = adw::AlertDialog::new(None, None);

        error_dialog.add_response("close", t!("dialog.close").as_ref());
        error_dialog.set_close_response("close");

        let discard_dialog = adw::AlertDialog::new(None, None);
        discard_dialog.add_response("cancel", t!("dialog.cancel").as_ref());
        discard_dialog.add_response("discard", t!("config.discard").as_ref());
        discard_dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        discard_dialog.set_default_response(Some("cancel"));
        discard_dialog.set_close_response("cancel");
        let input = sender.input_sender().clone();
        discard_dialog
            .connect_response(Some("discard"), move |_, _| input.emit(EditorMsg::Discard));

        let model = Self {
            api,
            creating: config.is_none(),
            original: config,
            fields,
            busy: true,
            loaded: false,
            dirty: false,
            submitted: false,
            error: None,
            error_dialog,
            discard_dialog,
        };

        let page = &model.fields.page;
        let widgets = view_output!();

        sender.input(EditorMsg::Load);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: EditorMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            EditorMsg::Changed => {
                self.fields.update_conditions();

                self.dirty = self.loaded
                    && self.original.as_ref().is_some_and(|original| {
                        self.fields.read(original).as_ref() != Some(original)
                    });

                if self.submitted {
                    self.fields.validate(false);
                }
            }
            EditorMsg::Retranslate => self.fields.retranslate(),
            EditorMsg::Back if self.dirty => {
                self.discard_dialog
                    .set_heading(Some(t!("config.discard_title").as_ref()));
                self.discard_dialog
                    .set_body(t!("config.discard_hint").as_ref());
                self.discard_dialog
                    .set_response_label("cancel", t!("dialog.cancel").as_ref());
                self.discard_dialog
                    .set_response_label("discard", t!("config.discard").as_ref());
                self.discard_dialog.present(Some(root));
            }
            EditorMsg::Back | EditorMsg::Discard => {
                sender.output(EditorOutput::Closed).unwrap();
            }
            EditorMsg::Reset => {
                self.fields.load(self.original.as_ref().unwrap());
                self.error = None;
                self.submitted = false;
                self.dirty = false;
                self.fields.clear_errors();
            }
            EditorMsg::ShowError => self.show_error(root),
            EditorMsg::Load => {
                self.busy = true;
                self.error = None;

                let api = self.api.clone();

                let original = self.original.clone();
                sender.oneshot_command(async move {
                    EditorReply::Loaded(
                        async {
                            let config = match original {
                                Some(config) => config,
                                None => api.default_config().await?,
                            };
                            let interfaces = api.interfaces().await?;
                            Ok((Box::new(config), interfaces))
                        }
                        .await,
                    )
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
            EditorReply::Loaded(Ok((config, interfaces))) => {
                self.fields
                    .populate_interfaces(&interfaces, &config, sender.input_sender());
                self.fields.load(&config);
                self.original = Some(*config);
                self.loaded = true;
            }
            EditorReply::Saved(Ok(())) => {
                sender.output(EditorOutput::Saved).unwrap();
            }
            EditorReply::Loaded(Err(error)) | EditorReply::Saved(Err(error)) => {
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

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        if self.error_dialog.parent().is_some() {
            self.error_dialog.force_close();
        }

        if self.discard_dialog.parent().is_some() {
            self.discard_dialog.force_close();
        }
    }
}

impl Editor {
    fn show_error(&self, parent: &adw::NavigationPage) {
        if let Some(error) = &self.error {
            let heading = if !self.loaded {
                t!("config.editor_load_error")
            } else {
                t!("config.save_error")
            };

            self.error_dialog
                .show_error(heading.as_ref(), error, parent);
        }
    }
}
