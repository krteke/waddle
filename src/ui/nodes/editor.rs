use super::fields::Fields;
use crate::{
    api::{
        Api,
        node::{ImportArgument, Node},
    },
    error::{ApiError, SaveError},
    node::{NodeConfig, ProtocolKind},
    ui::error_dialog::DialogExt,
};
use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

pub struct Editor {
    api: Api,
    id: Option<String>,
    original: NodeConfig,
    original_tag: String,
    fields: Fields,
    form: adw::Bin,
    dirty: bool,
    busy: bool,
    submitted: bool,
    error: Option<SaveError>,
    error_dialog: adw::AlertDialog,
    discard_dialog: adw::AlertDialog,
}

#[derive(Debug)]
pub enum EditorMsg {
    Changed,
    Protocol(ProtocolKind),
    Save,
    Reset,
    Back,
    Discard,
    ShowError,
    Retranslate,
}

#[derive(Debug)]
pub enum EditorOutput {
    Saved,
    Closed,
    Updated,
    Unauthorized,
}

#[derive(Debug)]
pub struct SaveReply {
    link_saved: bool,
    result: Result<(), SaveError>,
}

#[relm4::component(pub)]
impl Component for Editor {
    type Init = (Api, Option<Node>, NodeConfig);
    type Input = EditorMsg;
    type Output = EditorOutput;
    type CommandOutput = SaveReply;

    view! {
        adw::NavigationPage {
            #[watch]
            set_title: if model.id.is_some() {t!("nodes.edit")} else {t!("nodes.configure")}.as_ref(),
            #[watch]
            set_can_pop: !model.busy && !model.dirty,
            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    set_show_back_button: false,
                    pack_start = &gtk::Button {
                        set_icon_name: "go-previous-symbolic",
                        #[watch]
                        set_tooltip_text: Some(t!("node_editor.back").as_ref()),
                        #[watch]
                        set_sensitive: !model.busy,
                        connect_clicked => EditorMsg::Back,
                    },
                    pack_end = &gtk::Button {
                        add_css_class: "suggested-action",
                        #[watch]
                        set_label: t!("dialog.save").as_ref(),
                        #[watch]
                        set_sensitive: !model.busy && (model.id.is_none() || model.dirty),
                        connect_clicked => EditorMsg::Save,
                    },
                },
                add_top_bar = &adw::Banner {
                    #[watch]
                    set_title: if matches!(model.error, Some(SaveError::Tag(_))) {t!("node_editor.tag_error")} else {t!("node_editor.save_error")}.as_ref(),
                    #[watch]
                    set_revealed: model.error.is_some(),
                    #[watch]
                    set_button_label: Some(t!("error.details").as_ref()),
                    connect_button_clicked => EditorMsg::ShowError,
                },
                #[local_ref]
                form -> adw::Bin {
                    set_vexpand: true,
                    #[watch]
                    set_sensitive: !model.busy,
                },
                add_bottom_bar = &gtk::Box {
                    set_margin_all: 12,
                    set_spacing: 12,
                    gtk::Button {
                        #[watch]
                        set_label: t!("config.reset").as_ref(),
                        #[watch]
                        set_sensitive: model.dirty && !model.busy,
                        connect_clicked => EditorMsg::Reset,
                    },
                    gtk::Spinner {
                        #[watch]
                        set_spinning: model.busy,
                        #[watch]
                        set_visible: model.busy,
                    },
                },
            },
        }
    }

    fn init(
        (api, node, config): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let tag = node
            .as_ref()
            .and_then(|n| n.tag.clone())
            .unwrap_or_default();

        let fields = Fields::new(config, &tag, sender.input_sender());
        let original = fields.config();
        let form = adw::Bin::builder().child(&fields.page).build();

        let error_dialog = adw::AlertDialog::new(None, None);

        error_dialog.add_response("close", t!("dialog.close").as_ref());
        error_dialog.set_close_response("close");

        let discard_dialog = adw::AlertDialog::new(None, None);

        discard_dialog.add_response("cancel", t!("dialog.cancel").as_ref());
        discard_dialog.add_response("discard", t!("node_editor.discard").as_ref());

        discard_dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        discard_dialog.set_default_response(Some("cancel"));
        discard_dialog.set_close_response("cancel");

        let input = sender.input_sender().clone();

        discard_dialog
            .connect_response(Some("discard"), move |_, _| input.emit(EditorMsg::Discard));

        let model = Self {
            api,
            id: node.map(|n| n.id),
            original,
            original_tag: tag,
            fields,
            form,
            dirty: false,
            busy: false,
            submitted: false,
            error: None,
            error_dialog,
            discard_dialog,
        };

        let form = &model.form;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: EditorMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            EditorMsg::Changed => {
                self.fields.update_visibility();

                self.dirty =
                    self.fields.config() != self.original || self.fields.tag() != self.original_tag;

                if self.submitted {
                    self.fields.validate();
                }
            }
            EditorMsg::Protocol(protocol) => {
                if protocol == self.fields.config().protocol.kind() {
                    return;
                }

                let mut config = self.fields.config();
                config.protocol = protocol.empty();
                self.fields = Fields::new(config, &self.fields.tag(), sender.input_sender());

                self.form.set_child(Some(&self.fields.page));
                self.submitted = false;

                sender.input(EditorMsg::Changed);
            }
            EditorMsg::Reset => {
                self.fields = Fields::new(
                    self.original.clone(),
                    &self.original_tag,
                    sender.input_sender(),
                );

                self.form.set_child(Some(&self.fields.page));

                self.submitted = false;
                self.dirty = false;
                self.error = None;
            }
            EditorMsg::Back if self.dirty => {
                self.discard_dialog
                    .set_heading(Some(t!("node_editor.discard_title").as_ref()));

                self.discard_dialog
                    .set_body(t!("node_editor.discard_hint").as_ref());

                self.discard_dialog
                    .set_response_label("cancel", t!("dialog.cancel").as_ref());

                self.discard_dialog
                    .set_response_label("discard", t!("node_editor.discard").as_ref());

                self.discard_dialog.present(Some(root));
            }
            EditorMsg::Back | EditorMsg::Discard => {
                sender.output(EditorOutput::Closed).unwrap();
            }
            EditorMsg::Save => {
                self.fields.commit_numbers();
                self.submitted = true;

                if !self.fields.validate() {
                    return;
                }

                let config = self.fields.config();

                let link = match config.encode() {
                    Ok(link) => link,
                    Err(error) => {
                        tracing::warn!(%error, "Could not encode node configuration");

                        self.error = Some(SaveError::Link(error));
                        sender.input(EditorMsg::ShowError);

                        return;
                    }
                };

                self.busy = true;
                self.error = None;

                let api = self.api.clone();
                let id = self.id.clone();
                let tag = self.fields.tag();

                let tag_changed = tag != self.original_tag;
                let link_changed = config != self.original;

                sender.oneshot_command(async move {
                    if let Some(id) = id {
                        if link_changed && let Err(error) = api.update_node(id.clone(), link).await
                        {
                            return SaveReply {
                                link_saved: false,
                                result: Err(error.into()),
                            };
                        }

                        let result = if tag_changed {
                            api.tag_node(id, tag).await.map_err(SaveError::Tag)
                        } else {
                            Ok(())
                        };

                        SaveReply {
                            link_saved: link_changed,
                            result,
                        }
                    } else {
                        let result = match api
                            .import_nodes(vec![ImportArgument {
                                link,
                                tag: Some(tag),
                            }])
                            .await
                        {
                            Err(error) => Err(error.into()),
                            Ok(mut results) => {
                                let imported = results.remove(0);

                                if let Some(error) = imported.error {
                                    Err(SaveError::Import(error))
                                } else {
                                    Ok(())
                                }
                            }
                        };
                        SaveReply {
                            link_saved: false,
                            result,
                        }
                    }
                });
            }
            EditorMsg::ShowError => {
                if let Some(error) = &self.error {
                    let heading = if matches!(error, SaveError::Tag(_)) {
                        t!("node_editor.tag_error")
                    } else {
                        t!("node_editor.save_error")
                    };

                    match error {
                        SaveError::Api(error) | SaveError::Tag(error) => {
                            self.error_dialog.show_error(&heading, error, root)
                        }
                        SaveError::Import(_) | SaveError::Link(_) => {
                            let message = match error {
                                SaveError::Import(message) => {
                                    std::borrow::Cow::Borrowed(message.as_str())
                                }
                                SaveError::Link(_) => t!("node_editor.invalid_link"),
                                _ => unreachable!(),
                            };

                            self.error_dialog.set_heading(Some(&heading));
                            self.error_dialog.set_extra_child(None::<&gtk::Widget>);

                            self.error_dialog.set_body(&message);
                            self.error_dialog.set_body_use_markup(false);

                            self.error_dialog.present(Some(root));
                        }
                    }
                }
            }
            EditorMsg::Retranslate => self.fields.retranslate(),
        }
    }

    fn update_cmd(&mut self, reply: SaveReply, sender: ComponentSender<Self>, _: &Self::Root) {
        self.busy = false;

        if reply.link_saved && reply.result.is_err() {
            self.original = self.fields.config();
            sender.output(EditorOutput::Updated).unwrap();
        }

        match reply.result {
            Ok(_) => {
                sender.output(EditorOutput::Saved).unwrap();
            }

            Err(
                SaveError::Api(ApiError::Unauthorized) | SaveError::Tag(ApiError::Unauthorized),
            ) => {
                sender.output(EditorOutput::Unauthorized).unwrap();
            }

            Err(error) => {
                tracing::warn!(%error, "Could not save node");

                self.error = Some(error);

                sender.input(EditorMsg::Changed);
                sender.input(EditorMsg::ShowError);
            }
        }
    }

    fn shutdown(&mut self, _: &mut Self::Widgets, _: Sender<Self::Output>) {
        if self.error_dialog.parent().is_some() {
            self.error_dialog.force_close();
        }

        if self.discard_dialog.parent().is_some() {
            self.discard_dialog.force_close();
        }
    }
}
