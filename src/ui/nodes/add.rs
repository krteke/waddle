mod input;

use crate::{
    api::{
        Api,
        node::{ImportArgument, NodeImportResult},
    },
    error::ApiError,
    ui::error_dialog::DialogExt,
};
use adw::prelude::*;
use gtk::glib;
use input::NodeInput;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

pub struct AddNodes {
    api: Api,
    groups: gtk::Box,
    inputs: Vec<NodeInput>,
    busy: bool,
    submitted: bool,
    dirty: bool,
    error_dialog: adw::AlertDialog,
    height_animation: adw::TimedAnimation,
}

#[derive(Debug)]
pub enum AddMsg {
    Append,
    Remove(adw::PreferencesGroup),
    Changed,
    Submit,
    Reset,
    Cancel,
    Retranslate,
}

#[derive(Debug)]
pub enum AddOutput {
    Imported,
    Closed,
    Unauthorized,
}

#[relm4::component(pub)]
impl Component for AddNodes {
    type Init = Api;
    type Input = AddMsg;
    type Output = AddOutput;
    type CommandOutput = Result<Vec<NodeImportResult>, ApiError>;

    view! {
        #[name = "dialog"]
        adw::Dialog {
            set_content_width: 600,
            #[watch]
            set_title: t!("nodes.add_title").as_ref(),
            #[watch]
            set_can_close: !model.busy,
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
                        connect_clicked => AddMsg::Cancel,
                    },
                    pack_end = &gtk::Button {
                        add_css_class: "suggested-action",
                        #[watch]
                        set_label: t!("nodes.add").as_ref(),
                        #[watch]
                        set_sensitive: !model.busy && !model.inputs.is_empty(),
                        connect_clicked => AddMsg::Submit,
                    },
                },
                gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_propagate_natural_height: true,
                    set_max_content_height: 420,
                    #[local_ref]
                    groups -> gtk::Box {
                        set_margin_all: 24,
                        #[watch]
                        set_sensitive: !model.busy,
                    },
                },
                add_bottom_bar = &gtk::Box {
                    set_margin_all: 12,
                    set_spacing: 12,
                    gtk::Button {
                        set_icon_name: "list-add-symbolic",
                        #[watch]
                        set_tooltip_text: Some(t!("nodes.add_another").as_ref()),
                        #[watch]
                        set_sensitive: !model.busy,
                        connect_clicked => AddMsg::Append,
                    },
                    gtk::Button {
                        #[watch]
                        set_label: t!("config.reset").as_ref(),
                        #[watch]
                        set_sensitive: !model.busy && model.dirty,
                        connect_clicked => AddMsg::Reset,
                    },
                    gtk::Spinner {
                        #[watch]
                        set_spinning: model.busy,
                        #[watch]
                        set_visible: model.busy,
                    },
                },
            },
            set_content_height: -1,
        }
    }

    fn init(api: Api, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let error_dialog = adw::AlertDialog::new(None, None);

        error_dialog.add_response("close", t!("dialog.close").as_ref());
        error_dialog.set_close_response("close");

        let target = adw::CallbackAnimationTarget::new(glib::clone!(
            #[weak]
            root,
            move |height| root.set_content_height(height.round() as i32)
        ));

        let height_animation = adw::TimedAnimation::new(&root, 0.0, 0.0, 200, target);
        height_animation.set_easing(adw::Easing::EaseOutCubic);

        let mut model = Self {
            api,
            groups: gtk::Box::new(gtk::Orientation::Vertical, 24),
            inputs: Vec::new(),
            busy: false,
            submitted: false,
            dirty: false,
            error_dialog,
            height_animation,
        };

        model.append(&sender);

        let groups = &model.groups;
        let widgets = view_output!();

        root.connect_closed(move |_| {
            let _ = sender.output(AddOutput::Closed);
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: AddMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            AddMsg::Append => {
                self.append(&sender);
                self.inputs.last().unwrap().link.row.grab_focus();
            }
            AddMsg::Remove(group) => {
                let index = self
                    .inputs
                    .iter()
                    .position(|input| input.group == group)
                    .unwrap();
                self.groups.remove(&group);
                self.inputs.remove(index);
                if let Some(input) = self.inputs.get(index).or_else(|| self.inputs.last()) {
                    input.link.row.grab_focus();
                }
            }
            AddMsg::Reset => {
                for input in self.inputs.drain(..) {
                    self.groups.remove(&input.group);
                }
                self.submitted = false;
                self.append(&sender);
            }
            AddMsg::Changed => {
                if self.submitted {
                    for input in &self.inputs {
                        input.validate();
                    }
                }
            }
            AddMsg::Retranslate => {}
            AddMsg::Cancel => {
                root.close();
            }
            AddMsg::Submit => {
                self.submitted = true;
                let mut valid = true;
                for input in &self.inputs {
                    if !input.validate() {
                        valid = false;
                    }
                }
                if !valid {
                    if let Some(input) = self.inputs.iter().find(|input| {
                        input.link.row.has_css_class("error")
                            || input.tag.row.has_css_class("error")
                    }) {
                        if input.link.row.has_css_class("error") {
                            input.link.row.grab_focus();
                        } else {
                            input.tag.row.grab_focus();
                        }
                    }
                    return;
                }
                let args = self
                    .inputs
                    .iter()
                    .map(|input| ImportArgument {
                        link: input.link.row.text().trim().to_owned(),
                        tag: Some(input.tag.row.text().trim().to_owned()),
                    })
                    .collect();
                self.busy = true;
                let api = self.api.clone();
                sender.oneshot_command(async move { api.import_nodes(args).await });
            }
        }
        self.dirty = self.inputs.len() != 1
            || self
                .inputs
                .iter()
                .any(|input| !input.link.row.text().is_empty() || !input.tag.row.text().is_empty());
        for (index, input) in self.inputs.iter().enumerate() {
            input.retranslate(index + 1);
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
            Ok(results) => {
                let mut remaining = Vec::new();
                let mut imported = false;
                for (input, result) in self.inputs.drain(..).zip(results) {
                    if let Some(error) = result.error {
                        tracing::warn!(%error, "Node import failed");
                        input.show_error(&error);
                        remaining.push(input);
                    } else {
                        self.groups.remove(&input.group);
                        imported = true;
                    }
                }
                self.inputs = remaining;
                if imported {
                    sender.output(AddOutput::Imported).unwrap();
                }
                if self.inputs.is_empty() {
                    root.force_close();
                } else {
                    sender.input(AddMsg::Changed);
                }
            }
            Err(ApiError::Unauthorized) => {
                sender.output(AddOutput::Unauthorized).unwrap();
            }
            Err(error) => {
                tracing::warn!(%error, "Node import request failed");
                self.error_dialog
                    .show_error(t!("nodes.import_error").as_ref(), &error, root);
            }
        }
    }

    fn post_view() {
        if !dialog.is_mapped() {
            dialog.set_content_height(-1);
            return;
        }

        let content = dialog.child().unwrap();
        let height = content
            .measure(gtk::Orientation::Vertical, content.width())
            .1 as f64;
        if self.height_animation.value_to() == height {
            return;
        }

        self.height_animation.pause();
        self.height_animation
            .set_value_from(content.height() as f64);
        self.height_animation.set_value_to(height);
        self.height_animation.play();
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        self.height_animation.pause();
        if self.error_dialog.parent().is_some() {
            self.error_dialog.force_close();
        }
        if widgets.dialog.parent().is_some() {
            widgets.dialog.force_close();
        }
    }
}

impl AddNodes {
    fn append(&mut self, sender: &ComponentSender<Self>) {
        let input = NodeInput::new(sender.input_sender());
        input.retranslate(self.inputs.len() + 1);
        self.groups.append(&input.group);
        self.inputs.push(input);
    }
}
