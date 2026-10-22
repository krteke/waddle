mod add;

use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use crate::{
    api::{Api, node::Node},
    error::ApiError,
    ui::error_dialog::DialogExt,
};
use add::{AddMsg, AddNodes, AddOutput};

pub struct Nodes {
    api: Api,
    records: Vec<Node>,
    group: adw::PreferencesGroup,
    rows: Vec<adw::ActionRow>,
    busy: bool,
    error: Option<ApiError>,
    error_dialog: adw::AlertDialog,
    add: Option<Controller<AddNodes>>,
}

#[derive(Debug)]
pub enum NodeMsg {
    Refresh,
    Add,
    Added(AddOutput),
    ShowError,
    Retranslate,
}

#[relm4::component(pub)]
impl Component for Nodes {
    type Init = Api;
    type Input = NodeMsg;
    type Output = ();
    type CommandOutput = Result<Vec<Node>, ApiError>;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            adw::Banner {
                #[watch]
                set_title: t!("nodes.load_error").as_ref(),
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
                                set_label: &model.records.len().to_string(),
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
                                set_sensitive: false,
                                #[watch]
                                set_tooltip_text: Some(t!("nodes.test").as_ref()),
                            },
                            gtk::Button {
                                set_icon_name: "document-edit-symbolic",
                                add_css_class: "flat",
                                set_sensitive: false,
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
                            set_visible: model.records.is_empty() && !model.busy && model.error.is_none(),
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

    fn init(api: Api, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let error_dialog = adw::AlertDialog::new(None, None);

        error_dialog.add_response("close", t!("dialog.close").as_ref());
        error_dialog.set_close_response("close");

        let model = Self {
            api,
            records: Vec::new(),
            group: adw::PreferencesGroup::new(),
            rows: Vec::new(),
            busy: false,
            error: None,
            error_dialog,
            add: None,
        };

        let group = &model.group;
        let widgets = view_output!();

        sender.input(NodeMsg::Refresh);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: NodeMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            NodeMsg::Refresh | NodeMsg::Added(AddOutput::Imported) => {
                self.busy = true;
                self.error = None;
                let api = self.api.clone();
                sender.oneshot_command(async move { api.nodes().await });
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
                if let Some(error) = &self.error {
                    self.error_dialog
                        .show_error(t!("nodes.load_error").as_ref(), error, root);
                }
            }
            NodeMsg::Retranslate => {
                self.render();
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
        _root: &Self::Root,
    ) {
        self.busy = false;

        match result {
            Ok(records) => {
                self.records = records;
                self.render();
            }
            Err(ApiError::Unauthorized) => {
                sender.output(()).unwrap();
            }
            Err(error) => {
                tracing::warn!(endpoint = %self.api.endpoint(), %error, "Could not read nodes");
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

impl Nodes {
    fn render(&mut self) {
        self.rows.iter().for_each(|row| {
            self.group.remove(row);
        });

        self.records.iter().for_each(|node| {
            let tag = node.tag.as_deref().filter(|tag| !tag.is_empty());

            let row = adw::ActionRow::builder()
                .title(tag.unwrap_or(&node.name))
                .subtitle(format!("{} · {}", node.protocol.to_uppercase(), node.name))
                .use_markup(false)
                .title_lines(1)
                .subtitle_lines(1)
                .build();

            let icon = gtk::Image::builder()
                .icon_name("network-workgroup-symbolic")
                .icon_size(gtk::IconSize::Large)
                .css_classes(["dim-label"])
                .build();

            row.add_prefix(&icon);
            row.set_tooltip_text(Some(&node.name));

            for (icon, key) in [
                ("document-edit-symbolic", "nodes.edit"),
                ("insert-link-symbolic", "nodes.share"),
                ("user-trash-symbolic", "nodes.delete"),
            ] {
                let button = gtk::Button::builder()
                    .icon_name(icon)
                    .tooltip_text(t!(key).as_ref())
                    .sensitive(false)
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();

                row.add_suffix(&button);
            }

            self.group.add(&row);
            self.rows.push(row);
        });
    }
}
