mod add;
mod editor;
pub mod fields;
mod row;
mod share;

use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use crate::node::{DecodeExt, Protocol};
use crate::ui::latency::Latency;
use crate::{
    api::{
        Api,
        node::{Node, NodeLatency},
    },
    error::ApiError,
    ui::error_dialog::DialogExt,
};
use add::{AddMsg, AddNodes, AddOutput};
use editor::{Editor, EditorMsg, EditorOutput};
use row::NodeRow;

pub struct Nodes {
    api: Api,
    editor: Option<Controller<Editor>>,
    group: adw::PreferencesGroup,
    rows: Vec<NodeRow>,
    busy: bool,
    error: Option<(&'static str, ApiError)>,
    error_dialog: adw::AlertDialog,
    add: Option<Controller<AddNodes>>,
    delete_dialog: Option<adw::AlertDialog>,
    share_dialog: Option<adw::Dialog>,
}

#[derive(Debug)]
pub enum NodeMsg {
    Refresh,
    Configure,
    Edit(String),
    Edited(EditorOutput),
    EditorPopped,
    Test,
    Share(String),
    ConfirmDelete(String),
    Delete(String),
    Add,
    Added(AddOutput),
    ShowError,
    Retranslate,
}

#[derive(Debug)]
pub enum NodeCommand {
    Loaded(Result<Vec<Node>, ApiError>),
    Tested(Result<Vec<NodeLatency>, ApiError>),
    Deleted(String, Result<(), ApiError>),
}

#[relm4::component(pub)]
impl Component for Nodes {
    type Init = Api;
    type Input = NodeMsg;
    type Output = ();
    type CommandOutput = NodeCommand;

    view! {
        adw::NavigationView {
            connect_popped[ sender ] => move |_, _| sender.input(NodeMsg::EditorPopped),
            add = &adw::NavigationPage {
                #[watch]
                set_title: t!("navigation.nodes").as_ref(),
                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    connect_map => NodeMsg::Test,
                    adw::Banner {
                        #[watch]
                        set_title: model.error.as_ref().map(|(key, _)| t!(*key)).unwrap_or_default().as_ref(),
                        #[watch]
                        set_button_label: Some(t!("error.details").as_ref()),
                        #[watch]
                        set_revealed: model.error.is_some(),
                        connect_button_clicked => NodeMsg::ShowError,
                    },
                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        adw::Clamp {
                            set_maximum_size: 900,
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 18,
                                set_margin_all: 24,
                                gtk::Box {
                                    set_spacing: 6,
                                    gtk::Label {
                                        set_xalign: 0.0,
                                        add_css_class: "title-1",
                                        #[watch]
                                        set_label: t!("navigation.nodes").as_ref(),
                                    },
                                    gtk::Label {
                                        add_css_class: "dim-label",
                                        set_hexpand: true,
                                        set_xalign: 0.0,
                                        #[watch]
                                        set_label: &model.rows.len().to_string(),
                                    },
                                    gtk::Spinner {
                                        #[watch]
                                        set_spinning: model.busy,
                                        #[watch]
                                        set_visible: model.busy,
                                    },
                                    gtk::Button {
                                        set_icon_name: "network-transmit-receive-symbolic",
                                        add_css_class: "flat",
                                        #[watch]
                                        set_sensitive: !model.busy && !model.rows.is_empty(),
                                        connect_clicked => NodeMsg::Test,
                                        #[watch]
                                        set_tooltip_text: Some(t!("nodes.test").as_ref()),
                                    },
                                    gtk::Button {
                                        set_icon_name: "document-edit-symbolic",
                                        add_css_class: "flat",
                                        connect_clicked => NodeMsg::Configure,
                                        #[watch]
                                        set_tooltip_text: Some(t!("nodes.configure").as_ref()),
                                    },
                                    gtk::Button {
                                        set_icon_name: "list-add-symbolic",
                                        add_css_class: "flat",
                                        #[watch]
                                        set_sensitive: !model.busy,
                                        #[watch]
                                        set_tooltip_text: Some(t!("nodes.add_title").as_ref()),
                                        connect_clicked => NodeMsg::Add,
                                    },
                                },
                                #[local_ref]
                                group -> adw::PreferencesGroup {},
                                adw::StatusPage {
                                    set_icon_name: Some("network-server-symbolic"),
                                    #[watch]
                                    set_title: t!("nodes.empty").as_ref(),
                                    #[watch]
                                    set_visible: model.rows.is_empty() && !model.busy && model.error.is_none(),
                                },
                                gtk::Button {
                                    set_halign: gtk::Align::Center,
                                    #[watch]
                                    set_label: t!("auth.retry").as_ref(),
                                    #[watch]
                                    set_visible: model.error.is_some(),
                                    #[watch]
                                    set_sensitive: !model.busy,
                                    connect_clicked => NodeMsg::Refresh,
                                },
                            },
                        },
                    },
                }
            }
        }
    }

    fn init(api: Api, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let error_dialog = adw::AlertDialog::new(None, None);

        error_dialog.add_response("close", t!("dialog.close").as_ref());
        error_dialog.set_close_response("close");

        let model = Self {
            api,
            editor: None,
            group: adw::PreferencesGroup::new(),
            rows: Vec::new(),
            busy: false,
            error: None,
            error_dialog,
            add: None,
            delete_dialog: None,
            share_dialog: None,
        };

        let group = &model.group;
        let widgets = view_output!();

        sender.input(NodeMsg::Refresh);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: NodeMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            NodeMsg::Configure | NodeMsg::Edit(_) => {
                let node = match message {
                    NodeMsg::Edit(id) => Some(
                        self.rows
                            .iter()
                            .find(|r| r.node.id == id)
                            .unwrap()
                            .node
                            .clone(),
                    ),
                    _ => None,
                };

                let config = match node
                    .as_ref()
                    .map(|n| n.link.decode::<Protocol>())
                    .transpose()
                {
                    Ok(config) => config.unwrap_or_default(),
                    Err(error) => {
                        tracing::warn!(%error, "Could not open node editor");

                        self.error_dialog
                            .set_heading(Some(t!("nodes.edit").as_ref()));

                        self.error_dialog.set_extra_child(None::<&gtk::Widget>);
                        self.error_dialog
                            .set_body(t!("node_editor.parse_error").as_ref());

                        self.error_dialog.present(Some(root));

                        return;
                    }
                };

                let editor = Editor::builder()
                    .launch((self.api.clone(), node, config))
                    .forward(sender.input_sender(), NodeMsg::Edited);

                root.push(editor.widget());

                self.editor = Some(editor);
            }
            NodeMsg::Edited(EditorOutput::Saved | EditorOutput::Closed) => {
                if let Some(editor) = &self.editor {
                    editor.widget().set_can_pop(true);
                }

                root.pop();

                if matches!(message, NodeMsg::Edited(EditorOutput::Saved)) {
                    sender.input(NodeMsg::Refresh);
                }
            }
            NodeMsg::Edited(EditorOutput::Updated) => sender.input(NodeMsg::Refresh),
            NodeMsg::Edited(EditorOutput::Unauthorized) => {
                sender.output(()).unwrap();
            }
            NodeMsg::EditorPopped => {
                self.editor.take();
            }

            NodeMsg::Refresh | NodeMsg::Added(AddOutput::Imported) => {
                self.busy = true;
                self.error = None;

                let api = self.api.clone();
                sender.oneshot_command(async move { NodeCommand::Loaded(api.nodes().await) });
            }
            NodeMsg::Test => {
                if self.busy || self.rows.is_empty() || self.editor.is_some() {
                    return;
                }
                self.busy = true;
                self.error = None;
                for row in &mut self.rows {
                    row.latency = Latency::Testing;
                }
                let ids = self.rows.iter().map(|row| row.node.id.clone()).collect();
                let api = self.api.clone();
                sender.oneshot_command(async move {
                    NodeCommand::Tested(api.test_node_latencies(ids).await)
                });
            }
            NodeMsg::Share(id) => {
                let row = self.rows.iter().find(|row| row.node.id == id).unwrap();
                let dialog = row.node.dialog();
                dialog.present(Some(root));
                self.share_dialog = Some(dialog);
            }
            NodeMsg::ConfirmDelete(id) => {
                let row = self.rows.iter().find(|row| row.node.id == id).unwrap();
                let dialog = adw::AlertDialog::builder()
                    .heading(t!("nodes.delete_title").as_ref())
                    .body(t!("nodes.delete_hint", name = row.widget.title()).as_ref())
                    .body_use_markup(false)
                    .build();
                dialog.add_response("cancel", t!("dialog.cancel").as_ref());
                dialog.add_response("delete", t!("config.delete").as_ref());
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                dialog.set_default_response(Some("cancel"));
                dialog.set_close_response("cancel");
                dialog.connect_response(Some("delete"), move |_, _| {
                    sender.input(NodeMsg::Delete(id.clone()))
                });
                dialog.present(Some(root));
                self.delete_dialog = Some(dialog);
            }
            NodeMsg::Delete(id) => {
                self.busy = true;
                self.error = None;
                let api = self.api.clone();
                sender.oneshot_command(async move {
                    let result = api.remove_nodes(vec![id.clone()]).await;
                    NodeCommand::Deleted(id, result)
                });
            }
            NodeMsg::Add => {
                let add = AddNodes::builder()
                    .launch(self.api.clone())
                    .forward(sender.input_sender(), NodeMsg::Added);
                add.widget().present(Some(root));
                self.add = Some(add);
            }
            NodeMsg::Added(AddOutput::Closed) => {
                self.add.take();
            }
            NodeMsg::Added(AddOutput::Unauthorized) => {
                self.add.take();
                sender.output(()).unwrap();
            }
            NodeMsg::ShowError => {
                if let Some((key, error)) = &self.error {
                    self.error_dialog.show_error(t!(*key).as_ref(), error, root);
                }
            }
            NodeMsg::Retranslate => {
                if let Some(editor) = &self.editor {
                    editor.emit(EditorMsg::Retranslate);
                }
                if let Some(add) = &self.add {
                    add.emit(AddMsg::Retranslate);
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        result: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.busy = false;
        let report_in_dialog = matches!(&result, NodeCommand::Deleted(_, Err(_)));
        let (key, result) = match result {
            NodeCommand::Loaded(result) => (
                "nodes.load_error",
                result.map(|records| {
                    for row in self.rows.drain(..) {
                        self.group.remove(&row.widget);
                    }
                    for node in records {
                        let row = NodeRow::new(node, sender.input_sender().clone());
                        self.group.add(&row.widget);
                        self.rows.push(row);
                    }
                    if root.is_mapped() && self.editor.is_none() {
                        sender.input(NodeMsg::Test);
                    }
                }),
            ),
            NodeCommand::Tested(result) => {
                for row in &mut self.rows {
                    row.latency = if result.is_ok() {
                        Latency::Untested
                    } else {
                        Latency::RequestFailed
                    };
                }
                (
                    "nodes.test_error",
                    result.map(|results| {
                        for result in results {
                            if let Some(row) =
                                self.rows.iter_mut().find(|row| row.node.id == result.id)
                            {
                                row.latency = Latency::Measured(result);
                            }
                        }
                    }),
                )
            }
            NodeCommand::Deleted(id, result) => (
                "nodes.delete_error",
                result.map(|()| {
                    let index = self.rows.iter().position(|row| row.node.id == id).unwrap();
                    self.group.remove(&self.rows.remove(index).widget);
                }),
            ),
        };
        match result {
            Ok(()) => {}
            Err(ApiError::Unauthorized) => {
                sender.output(()).unwrap();
            }
            Err(error) => {
                tracing::warn!(endpoint = %self.api.endpoint(), operation = key, %error, "Node operation failed");
                if report_in_dialog {
                    self.error_dialog.show_error(t!(key).as_ref(), &error, root);
                }
                self.error = Some((key, error));
            }
        }
    }

    fn post_view(&self, _widgets: &mut Self::Widgets) {
        for row in &self.rows {
            row.update(self.busy);
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        if self.error_dialog.parent().is_some() {
            self.error_dialog.force_close();
        }
        if let Some(dialog) = &self.share_dialog {
            dialog.force_close();
        }
        if let Some(dialog) = &self.delete_dialog {
            dialog.force_close();
        }
    }
}

#[cfg(test)]
pub use fields::tests as form_tests;
