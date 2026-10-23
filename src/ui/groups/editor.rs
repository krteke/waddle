use super::{
    identity::{Identity, node_row},
    picker::{Picker, PickerMsg, PickerOutput},
};
use crate::{
    api::{
        Api,
        group::{Group, GroupNode},
    },
    error::ApiError,
    ui::error_dialog::DialogExt,
};

use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

pub struct Editor {
    api: Api,
    original: Group,
    members: Vec<GroupNode>,
    fields: Identity,
    nodes: adw::PreferencesGroup,
    subscriptions: adw::PreferencesGroup,
    rows: Vec<adw::ActionRow>,
    busy: bool,
    dirty: bool,
    error: Option<ApiError>,
    error_dialog: adw::AlertDialog,
    discard: adw::AlertDialog,
    picker: Option<Controller<Picker>>,
}

#[derive(Debug)]
pub enum EditorMsg {
    Changed,
    Save,
    Reset,
    Back,
    Discard,
    AddNodes,
    RemoveNode(String),
    Picker(PickerOutput),
    ShowError,
    Retranslate,
}

#[derive(Debug)]
pub enum EditorOutput {
    Closed,
    Saved,
    Unauthorized,
}

#[derive(Debug)]
pub struct SaveReply {
    result: Result<(), ApiError>,
    current: Option<Group>,
}

#[relm4::component(pub)]
impl Component for Editor {
    type Init = (Api, Group);
    type Input = EditorMsg;
    type Output = EditorOutput;
    type CommandOutput = SaveReply;
    view! {
        adw::NavigationPage {
            set_tag: Some("group-editor"),
            #[watch]
            set_title: &model.original.name,
            #[watch]
            set_can_pop: !model.busy && !model.dirty,
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    set_show_back_button: false,
                    pack_start = &gtk::Button {
                        set_icon_name: "go-previous-symbolic",
                        #[watch]
                        set_tooltip_text: Some(t!("config.back").as_ref()),
                        #[watch]
                        set_sensitive: !model.busy,
                        connect_clicked => EditorMsg::Back,
                    },
                    pack_end = &gtk::Button {
                        add_css_class: "suggested-action",
                        #[watch]
                        set_label: &t!("dialog.save"),
                        #[watch]
                        set_sensitive: !model.busy && model.dirty && !model.fields.name.text().trim().is_empty(),
                        connect_clicked => EditorMsg::Save,
                    },
                },
                add_top_bar = &adw::Banner {
                    #[watch]
                    set_title: &t!("groups.save_error"),
                    #[watch]
                    set_revealed: model.error.is_some(),
                    #[watch]
                    set_button_label: Some(t!("error.details").as_ref()),
                    connect_button_clicked => EditorMsg::ShowError,
                },
                adw::PreferencesPage {
                    #[watch]
                    set_sensitive: !model.busy,
                    #[local_ref]
                    add = fields -> adw::PreferencesGroup {},
                    #[local_ref]
                    add = nodes -> adw::PreferencesGroup {
                        #[watch]
                        set_title: &t!("navigation.nodes"),
                        #[wrap(Some)]
                        set_header_suffix = &gtk::Button {
                            set_icon_name: "list-add-symbolic",
                            add_css_class: "flat",
                            #[watch]
                            set_tooltip_text: Some(t!("groups.add_nodes").as_ref()),
                            connect_clicked => EditorMsg::AddNodes,
                        },
                    },
                    #[local_ref]
                    add = subscriptions -> adw::PreferencesGroup {
                        #[watch]
                        set_title: &t!("groups.subscriptions"),
                        #[wrap(Some)]
                        set_header_suffix = &gtk::Button {
                            set_icon_name: "list-add-symbolic",
                            add_css_class: "flat",
                            set_sensitive: false,
                            #[watch]
                            set_tooltip_text: Some(t!("groups.subscriptions_placeholder").as_ref()),
                        },
                    },
                },
                add_bottom_bar = &gtk::Box {
                    set_margin_all: 12,
                    set_spacing: 12,
                    gtk::Button {
                        #[watch]
                        set_label: &t!("config.reset"),
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
        (api, group): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let fields = Identity::new(&group.name, group.policy);
        let input = sender.input_sender().clone();

        fields
            .name
            .connect_changed(move |_| input.emit(EditorMsg::Changed));

        let input = sender.input_sender().clone();

        fields
            .policy
            .connect_selected_notify(move |_| input.emit(EditorMsg::Changed));

        let error_dialog = adw::AlertDialog::new(None, None);
        error_dialog.add_response("close", &t!("dialog.close"));

        let discard = adw::AlertDialog::new(None, None);

        discard.add_response("cancel", &t!("dialog.cancel"));
        discard.add_response("discard", &t!("config.discard"));

        discard.set_default_response(Some("cancel"));
        discard.set_close_response("cancel");

        discard.set_response_appearance("discard", adw::ResponseAppearance::Destructive);

        let input = sender.input_sender().clone();
        discard.connect_response(Some("discard"), move |_, _| input.emit(EditorMsg::Discard));

        let mut model = Self {
            api,
            original: group.clone(),
            members: group.nodes,
            fields,
            nodes: adw::PreferencesGroup::new(),
            subscriptions: adw::PreferencesGroup::new(),
            rows: Vec::new(),
            busy: false,
            dirty: false,
            error: None,
            error_dialog,
            discard,
            picker: None,
        };

        model.render_nodes(sender.input_sender());

        let fields = &model.fields.group;
        let nodes = &model.nodes;
        let subscriptions = &model.subscriptions;

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: EditorMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            EditorMsg::Changed => self.dirty = self.read() != self.original,
            EditorMsg::Reset => {
                self.members = self.original.nodes.clone();

                self.fields.name.set_text(&self.original.name);

                self.fields.policy.set_selected(
                    crate::api::group::Policy::ALL
                        .iter()
                        .position(|p| *p == self.original.policy)
                        .unwrap() as u32,
                );

                self.render_nodes(sender.input_sender());

                self.dirty = false;
                self.error = None;
            }
            EditorMsg::Back if self.dirty => {
                self.discard.set_heading(Some(&t!("config.discard_title")));
                self.discard.set_body(&t!("groups.discard_hint"));

                self.discard
                    .set_response_label("cancel", &t!("dialog.cancel"));

                self.discard
                    .set_response_label("discard", &t!("config.discard"));

                self.discard.present(Some(root));
            }
            EditorMsg::Back | EditorMsg::Discard => {
                sender.output(EditorOutput::Closed).unwrap();
            }
            EditorMsg::Save => {
                self.busy = true;
                self.error = None;
                let api = self.api.clone();
                let original = self.original.clone();
                let edited = self.read();
                sender.oneshot_command(async move {
                    let result = api.save_group(&original, &edited).await;

                    let current = if result.is_err() {
                        api.groups()
                            .await
                            .inspect_err(|error| tracing::warn!(%error, "Could not reconcile group after failed save"))
                            .ok()
                            .and_then(|group| group.into_iter().find(|g| g.id == original.id))
                    } else {
                        None
                    };

                    SaveReply { result, current }
                });
            }
            EditorMsg::AddNodes => {
                let picker = Picker::builder()
                    .launch((
                        self.api.clone(),
                        self.members.iter().map(|n| n.id.clone()).collect(),
                    ))
                    .forward(sender.input_sender(), EditorMsg::Picker);

                picker.widget().present(Some(root));

                self.picker = Some(picker);
            }
            EditorMsg::Picker(PickerOutput::Added(nodes)) => {
                self.members.extend(nodes.into_iter().map(Into::into));
                self.render_nodes(sender.input_sender());

                self.dirty = self.read() != self.original;
            }
            EditorMsg::Picker(PickerOutput::Unauthorized) => {
                sender.output(EditorOutput::Unauthorized).unwrap();
            }
            EditorMsg::RemoveNode(id) => {
                self.members.retain(|node| node.id != id);
                self.render_nodes(sender.input_sender());

                self.dirty = self.read() != self.original;
            }
            EditorMsg::ShowError => {
                if let Some(error) = &self.error {
                    self.error_dialog
                        .show_error(&t!("groups.save_error"), error, root);
                }
            }
            EditorMsg::Retranslate => {
                self.fields.retranslate();
                self.render_nodes(sender.input_sender());

                if let Some(picker) = &self.picker {
                    picker.emit(PickerMsg::Retranslate);
                }
            }
        }
    }

    fn update_cmd(&mut self, reply: SaveReply, sender: ComponentSender<Self>, root: &Self::Root) {
        self.busy = false;

        if let Some(current) = reply.current {
            self.original = current;
        }

        self.dirty = self.read() != self.original;

        match reply.result {
            Ok(_) => {
                sender.output(EditorOutput::Saved).unwrap();
            }
            Err(ApiError::Unauthorized) => {
                sender.output(EditorOutput::Unauthorized).unwrap();
            }
            Err(error) => {
                tracing::warn!(%error, "Could not save group");

                self.error_dialog
                    .show_error(&t!("groups.save_error"), &error, root);
                self.error = Some(error);
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        for dialog in [&self.error_dialog, &self.discard] {
            if dialog.parent().is_some() {
                dialog.force_close();
            }
        }

        if let Some(picker) = &self.picker
            && picker.widget().parent().is_some()
        {
            picker.widget().force_close();
        }
    }
}

impl Editor {
    fn read(&self) -> Group {
        let mut group = self.original.clone();

        group.nodes = self.members.clone();
        group.name = self.fields.name.text().trim().to_owned();
        group.policy = self.fields.policy();

        group
    }

    fn render_nodes(&mut self, input: &Sender<EditorMsg>) {
        for row in self.rows.drain(..) {
            self.nodes.remove(&row);
        }

        for node in &self.members {
            let row = node_row(&node.name, node.tag.as_deref(), &node.protocol);

            let remove = gtk::Button::builder()
                .icon_name("list-remove-symbolic")
                .css_classes(["flat", "error"])
                .valign(gtk::Align::Center)
                .tooltip_text(t!("groups.remove_node").as_ref())
                .build();

            let id = node.id.clone();
            let input = input.clone();

            remove.connect_clicked(move |_| input.emit(EditorMsg::RemoveNode(id.clone())));
            row.add_suffix(&remove);

            self.nodes.add(&row);
            self.rows.push(row);
        }

        self.nodes.set_description(
            self.members
                .is_empty()
                .then(|| t!("groups.no_nodes"))
                .as_deref(),
        );

        self.subscriptions.set_description(Some(&t!(
            "groups.subscription_count",
            count = self.original.subscriptions.len()
        )));
    }
}
