use super::{
    identity::Identity,
    member::MemberRow,
    picker::{Picker, PickerMsg, PickerOutput},
};
use crate::{
    api::{
        Api,
        group::{Group, GroupNode},
        node::NodeLatency,
    },
    error::ApiError,
    ui::{error_dialog::DialogExt, latency::Latency},
};

use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

pub struct Editor {
    api: Api,
    original: Group,
    members: Vec<MemberRow>,
    fields: Identity,
    nodes: adw::PreferencesGroup,
    subscriptions: adw::PreferencesGroup,
    busy: bool,
    testing: bool,
    dirty: bool,
    error: Option<(&'static str, ApiError)>,
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
    Test,
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

#[derive(Debug)]
pub enum EditorReply {
    Saved(SaveReply),
    Tested(Result<Vec<NodeLatency>, ApiError>),
}

#[relm4::component(pub)]
impl Component for Editor {
    type Init = (Api, Group);
    type Input = EditorMsg;
    type Output = EditorOutput;
    type CommandOutput = EditorReply;
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
                    set_title: &model.error.as_ref().map(|(key, _)| t!(*key)).unwrap_or_default(),
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
                        set_header_suffix = &gtk::Box {
                            gtk::Button {
                                set_icon_name: "network-transmit-receive-symbolic",
                                add_css_class: "flat",
                                #[watch]
                                set_tooltip_text: Some(t!("nodes.test").as_ref()),
                                #[watch]
                                set_sensitive: !model.testing && !model.members.is_empty(),
                                connect_clicked => EditorMsg::Test,
                            },
                            gtk::Button {
                                set_icon_name: "list-add-symbolic",
                                add_css_class: "flat",
                                #[watch]
                                set_tooltip_text: Some(t!("groups.add_nodes").as_ref()),
                                connect_clicked => EditorMsg::AddNodes,
                            },
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
            members: Vec::new(),
            fields,
            nodes: adw::PreferencesGroup::new(),
            subscriptions: adw::PreferencesGroup::new(),
            busy: false,
            testing: false,
            dirty: false,
            error: None,
            error_dialog,
            discard,
            picker: None,
        };

        model.add_members(group.nodes, sender.input_sender());

        let fields = &model.fields.group;
        let nodes = &model.nodes;
        let subscriptions = &model.subscriptions;

        let widgets = view_output!();

        sender.input(EditorMsg::Test);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: EditorMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            EditorMsg::Changed => self.dirty = self.read() != self.original,
            EditorMsg::Reset => {
                for member in self.members.drain(..) {
                    self.nodes.remove(&member.widget);
                }
                self.add_members(self.original.nodes.clone(), sender.input_sender());

                self.fields.name.set_text(&self.original.name);

                self.fields.policy.set_selected(
                    crate::api::group::Policy::ALL
                        .iter()
                        .position(|p| *p == self.original.policy)
                        .unwrap() as u32,
                );

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

                    EditorReply::Saved(SaveReply { result, current })
                });
            }
            EditorMsg::Test => {
                if self.testing || self.members.is_empty() {
                    return;
                }

                self.testing = true;
                self.error = None;

                for member in &mut self.members {
                    member.latency = Latency::Testing;
                }

                let ids = self
                    .members
                    .iter()
                    .map(|member| member.node.id.clone())
                    .collect();

                let api = self.api.clone();

                sender.oneshot_command(async move {
                    EditorReply::Tested(api.test_node_latencies(ids).await)
                });
            }
            EditorMsg::AddNodes => {
                let picker = Picker::builder()
                    .launch((
                        self.api.clone(),
                        self.members.iter().map(|n| n.node.id.clone()).collect(),
                    ))
                    .forward(sender.input_sender(), EditorMsg::Picker);

                picker.widget().present(Some(root));

                self.picker = Some(picker);
            }
            EditorMsg::Picker(PickerOutput::Added(nodes)) => {
                self.add_members(nodes.into_iter().map(Into::into), sender.input_sender());

                self.dirty = self.read() != self.original;
            }
            EditorMsg::Picker(PickerOutput::Unauthorized) => {
                sender.output(EditorOutput::Unauthorized).unwrap();
            }
            EditorMsg::RemoveNode(id) => {
                let index = self
                    .members
                    .iter()
                    .position(|member| member.node.id == id)
                    .unwrap();

                self.nodes.remove(&self.members.remove(index).widget);

                self.dirty = self.read() != self.original;
            }
            EditorMsg::ShowError => {
                if let Some((key, error)) = &self.error {
                    self.error_dialog.show_error(&t!(*key), error, root);
                }
            }
            EditorMsg::Retranslate => {
                self.fields.retranslate();

                if let Some(picker) = &self.picker {
                    picker.emit(PickerMsg::Retranslate);
                }
            }
        }
    }

    fn update_cmd(&mut self, reply: EditorReply, sender: ComponentSender<Self>, root: &Self::Root) {
        let report_in_dialog = matches!(&reply, EditorReply::Saved(_));

        let (key, result) = match reply {
            EditorReply::Saved(reply) => {
                self.busy = false;

                if let Some(current) = reply.current {
                    self.original = current;
                }

                self.dirty = self.read() != self.original;

                if reply.result.is_ok() {
                    sender.output(EditorOutput::Saved).unwrap();
                }

                ("groups.save_error", reply.result)
            }
            EditorReply::Tested(result) => {
                self.testing = false;

                for member in &mut self.members {
                    if matches!(member.latency, Latency::Testing) {
                        member.latency = if result.is_ok() {
                            Latency::Untested
                        } else {
                            Latency::RequestFailed
                        };
                    }
                }

                (
                    "nodes.test_error",
                    result.map(|results| {
                        for result in results {
                            if let Some(member) = self
                                .members
                                .iter_mut()
                                .find(|member| member.node.id == result.id)
                            {
                                member.latency = Latency::Measured(result);
                            }
                        }
                    }),
                )
            }
        };

        match result {
            Ok(_) => {}
            Err(ApiError::Unauthorized) => {
                sender.output(EditorOutput::Unauthorized).unwrap();
            }
            Err(error) => {
                tracing::warn!(operation = key, %error, "Group editor request failed");

                if report_in_dialog {
                    self.error_dialog.show_error(&t!(key), &error, root);
                }

                self.error = Some((key, error));
            }
        }
    }

    fn post_view(&self, _widgets: &mut Self::Widgets) {
        for member in &self.members {
            member.update();
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

        group.nodes = self
            .members
            .iter()
            .map(|member| member.node.clone())
            .collect();

        group.name = self.fields.name.text().trim().to_owned();
        group.policy = self.fields.policy();

        group
    }

    fn add_members(
        &mut self,
        nodes: impl IntoIterator<Item = GroupNode>,
        input: &Sender<EditorMsg>,
    ) {
        for node in nodes {
            let member = MemberRow::new(node, input);

            self.nodes.add(&member.widget);
            self.members.push(member);
        }
    }
}
