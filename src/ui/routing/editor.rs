use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use super::{presets::Presets, rules};
use crate::{
    api::{Api, routing::Routing},
    error::ApiError,
    ui::error_dialog::DialogExt,
};

pub struct Editor {
    api: Api,
    routing_id: Option<String>,
    original_name: String,
    original_text: String,
    name: adw::EntryRow,
    buffer: gtk::TextBuffer,
    presets: Presets,
    mode: adw::ToggleGroup,
    advanced: bool,
    dirty: bool,
    busy: bool,
    submitted: bool,
    error_dialog: adw::AlertDialog,
    confirmation: Option<adw::AlertDialog>,
}

#[derive(Clone, Debug)]
pub enum EditorMsg {
    Changed,
    PresetChanged,
    AddMac,
    RemoveMac(adw::EntryRow),
    Mode(u32),
    Replace,
    Save,
    Reset,
    Close,
    Discard,
    Retranslate,
}
#[derive(Clone, Debug)]
pub enum EditorOutput {
    Saved,
    Closed,
    Unauthorized,
}

#[relm4::component(pub)]
impl Component for Editor {
    type Init = (Api, Option<Routing>, Vec<String>);
    type Input = EditorMsg;
    type Output = EditorOutput;
    type CommandOutput = Result<(), ApiError>;

    view! {
        adw::Dialog {
            set_content_width: 760,
            set_content_height: 720,
            #[watch]
            set_title: if model.routing_id.is_none() { t!("routing.new_title") } else { t!("routing.edit") }.as_ref(),
            #[watch]
            set_can_close: !model.busy && !model.dirty,
            connect_close_attempt => EditorMsg::Close,
            connect_closed[sender] => move |_| { sender.output(EditorOutput::Closed).unwrap(); },
            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        #[watch]
                        set_label: t!("dialog.cancel").as_ref(),
                        #[watch]
                        set_sensitive: !model.busy,
                        connect_clicked => EditorMsg::Close,
                    },
                    pack_end = &gtk::Button {
                        add_css_class: "suggested-action",
                        #[watch]
                        set_label: t!("dialog.save").as_ref(),
                        #[watch]
                        set_sensitive: !model.busy && (model.routing_id.is_none() || model.dirty),
                        connect_clicked => EditorMsg::Save,
                    },
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    #[watch]
                    set_sensitive: !model.busy,
                    adw::PreferencesGroup {
                        set_margin_start: 24, set_margin_end: 24, set_margin_top: 12,
                        #[local_ref]
                        name -> adw::EntryRow {
                            #[watch]
                            set_title: t!("config.name").as_ref(),
                            set_use_markup: false,
                            add_suffix = &gtk::Label { set_label: "*", add_css_class: "error" },
                            set_editable: model.routing_id.is_none(),
                            connect_changed => EditorMsg::Changed,
                        },
                        gtk::Label {
                            set_xalign: 0.0,
                            add_css_class: "caption", add_css_class: "error",
                            #[watch]
                            set_label: t!("config.required").as_ref(),
                            #[watch]
                            set_visible: model.submitted && model.name.text().trim().is_empty(),
                        },
                    },
                    #[local_ref]
                    mode -> adw::ToggleGroup {
                        set_margin_start: 24, set_margin_end: 24, set_margin_top: 18, set_margin_bottom: 6,
                        set_homogeneous: true,
                        #[watch]
                        #[block_signal(mode_changed)]
                        set_active: u32::from(model.advanced),
                        connect_active_notify[sender] => move |g| sender.input(EditorMsg::Mode(g.active())) @mode_changed,
                    },
                    gtk::Stack {
                        set_vexpand: true,
                        set_vhomogeneous: false,
                        add_named[Some("simple")] = &model.presets.page.clone(),
                        add_named[Some("advanced")] = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 24,
                            set_spacing: 6,
                            #[local_ref]
                            rules -> adw::Bin {},
                            gtk::Label {
                                set_xalign: 0.0,
                                add_css_class: "caption", add_css_class: "error",
                                #[watch]
                                set_label: t!("config.required").as_ref(),
                                #[watch]
                                set_visible: model.submitted && model.text().trim().is_empty(),
                            },
                        },
                        #[watch]
                        set_visible_child_name: if model.advanced { "advanced" } else { "simple" },
                    },
                },
                add_bottom_bar = &gtk::Box {
                    set_margin_all: 12, set_spacing: 12,
                    gtk::Button {
                        #[watch]
                        set_label: t!("config.reset").as_ref(),
                        #[watch]
                        set_sensitive: model.dirty && !model.busy,
                        connect_clicked => EditorMsg::Reset,
                    },
                    gtk::Spinner {
                        #[watch]
                        set_visible: model.busy,
                        #[watch]
                        set_spinning: model.busy,
                    },
                },
            },
        }
    }

    fn init(
        (api, original, groups): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut presets = Presets::new(groups, sender.input_sender());

        let original_name = original
            .as_ref()
            .map(|r| r.name.clone())
            .unwrap_or_default();

        let original_text = original
            .as_ref()
            .map(|r| r.routing.string.clone())
            .unwrap_or_else(|| presets.read().map(|p| p.text()).unwrap_or_default());

        let advanced = if let Some(preset) = presets.detect(&original_text) {
            presets.load(&preset);
            false
        } else {
            original.is_some()
        };

        let name = adw::EntryRow::builder().text(&original_name).build();

        name.upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Required(true)]);

        let buffer = gtk::TextBuffer::new(None);
        buffer.set_text(&original_text);

        let input = sender.input_sender().clone();

        buffer.connect_changed(move |_| input.emit(EditorMsg::Changed));

        let rules = rules::view(&buffer, true);
        let error_dialog = adw::AlertDialog::new(None, None);

        error_dialog.add_response("close", t!("dialog.close").as_ref());
        error_dialog.set_close_response("close");

        let mode = adw::ToggleGroup::new();

        mode.add(
            adw::Toggle::builder()
                .label(t!("routing.simple").as_ref())
                .build(),
        );

        mode.add(
            adw::Toggle::builder()
                .label(t!("routing.advanced").as_ref())
                .build(),
        );

        let model = Self {
            api,
            routing_id: original.map(|r| r.id),
            original_name,
            original_text,
            name,
            buffer,
            presets,
            mode,
            advanced,
            dirty: false,
            busy: false,
            submitted: false,
            error_dialog,
            confirmation: None,
        };

        let name = &model.name;
        let mode = &model.mode;

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: EditorMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            EditorMsg::Changed => {}
            EditorMsg::PresetChanged => {
                self.presets.update_visibility();
                self.presets.retranslate();

                if !self.advanced {
                    self.apply_preset();
                }
            }
            EditorMsg::AddMac => self.presets.add_mac(""),
            EditorMsg::RemoveMac(row) => {
                self.presets.remove_mac(&row);
                self.apply_preset();
            }
            EditorMsg::Mode(1) => self.advanced = true,
            EditorMsg::Mode(_) => {
                if let Some(preset) = self.presets.detect(&self.text()) {
                    self.presets.load(&preset);
                    self.advanced = false;
                } else {
                    self.confirm(
                        root,
                        &sender,
                        (
                            "routing.overwrite_title",
                            "routing.overwrite_hint",
                            "routing.replace",
                        ),
                        EditorMsg::Replace,
                    );
                }
            }
            EditorMsg::Replace => {
                self.advanced = false;
                self.apply_preset();
            }
            EditorMsg::Reset => {
                self.submitted = false;
                self.name.remove_css_class("error");

                self.name.set_text(&self.original_name);
                self.buffer.set_text(&self.original_text);

                if let Some(preset) = self.presets.detect(&self.original_text) {
                    self.presets.load(&preset);
                    self.advanced = false;
                } else {
                    self.advanced = true;
                }
            }
            EditorMsg::Close => {
                if self.busy {
                    return;
                }

                if self.dirty {
                    self.confirm(
                        root,
                        &sender,
                        (
                            "config.discard_title",
                            "routing.discard_hint",
                            "config.discard",
                        ),
                        EditorMsg::Discard,
                    );
                } else {
                    root.force_close();
                }
            }
            EditorMsg::Discard => root.force_close(),
            EditorMsg::Save => {
                self.submitted = true;

                if !self.validate() {
                    return;
                }

                self.busy = true;

                let api = self.api.clone();
                let id = self.routing_id.clone();

                let name = self.name.text().trim().to_owned();
                let text = self.text();

                sender.oneshot_command(async move {
                    match id {
                        Some(id) => api.update_routing(id, text).await,
                        None => api.create_routing(name, text).await,
                    }
                });
            }
            EditorMsg::Retranslate => {
                self.presets.retranslate();

                self.mode
                    .toggle(0)
                    .unwrap()
                    .set_label(Some(&t!("routing.simple")));

                self.mode
                    .toggle(1)
                    .unwrap()
                    .set_label(Some(&t!("routing.advanced")));
            }
        }

        self.dirty =
            self.name.text().trim() != self.original_name || self.text() != self.original_text;

        if self.submitted {
            self.validate();
        }
    }

    fn update_cmd(
        &mut self,
        result: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.busy = false;

        match result {
            Ok(_) => {
                sender.output(EditorOutput::Saved).unwrap();
                root.force_close();
            }
            Err(ApiError::Unauthorized) => {
                sender.output(EditorOutput::Unauthorized).unwrap();
            }
            Err(error) => {
                tracing::warn!(endpoint=%self.api.endpoint(), %error, "Could not save routing");
                self.error_dialog
                    .show_error(t!("routing.save_error").as_ref(), &error, root);
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        self.error_dialog.force_close();

        if let Some(dialog) = &self.confirmation {
            dialog.force_close();
        }
    }
}

impl Editor {
    fn text(&self) -> String {
        self.buffer
            .text(&self.buffer.start_iter(), &self.buffer.end_iter(), false)
            .to_string()
    }

    fn apply_preset(&self) {
        self.buffer
            .set_text(&self.presets.read().map(|p| p.text()).unwrap_or_default());
    }

    fn validate(&self) -> bool {
        let name_valid = !self.name.text().trim().is_empty();

        if name_valid {
            self.name.remove_css_class("error");
        } else {
            self.name.add_css_class("error");
        }

        let preset_valid = self.advanced || self.presets.validate();

        name_valid && !self.text().trim().is_empty() && preset_valid
    }

    fn confirm(
        &mut self,
        root: &adw::Dialog,
        sender: &ComponentSender<Self>,
        (title, hint, action): (&str, &str, &str),
        message: EditorMsg,
    ) {
        let dialog = adw::AlertDialog::builder()
            .heading(t!(title).as_ref())
            .body(t!(hint).as_ref())
            .build();

        dialog.add_response("cancel", t!("dialog.cancel").as_ref());
        dialog.add_response("confirm", t!(action).as_ref());

        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
        dialog.set_close_response("cancel");
        dialog.set_default_response(Some("cancel"));

        let input = sender.input_sender().clone();

        dialog.connect_response(Some("confirm"), move |_, _| input.emit(message.clone()));
        dialog.present(Some(root));

        self.confirmation = Some(dialog);
    }
}
