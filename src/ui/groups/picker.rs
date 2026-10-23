use super::identity::node_row;
use crate::{
    api::{Api, node::Node},
    error::ApiError,
    ui::error_dialog::DialogExt,
};
use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

pub struct Picker {
    api: Api,
    excluded: Vec<String>,
    choices: Vec<(Node, adw::ActionRow, gtk::CheckButton)>,
    list: gtk::ListBox,
    search: gtk::SearchEntry,
    search_handler: Option<gtk::glib::SignalHandlerId>,
    busy: bool,
    selected: usize,
    matches: usize,
    error: Option<ApiError>,
    error_dialog: adw::AlertDialog,
}

#[derive(Debug)]
pub enum PickerMsg {
    Load,
    Filter,
    Changed,
    Add,
    Close,
    ShowError,
    Retranslate,
}

#[derive(Debug)]
pub enum PickerOutput {
    Added(Vec<Node>),
    Unauthorized,
}

#[relm4::component(pub)]
impl Component for Picker {
    type Init = (Api, Vec<String>);
    type Input = PickerMsg;
    type Output = PickerOutput;
    type CommandOutput = Result<Vec<Node>, ApiError>;
    view! {
        adw::Dialog {
            set_content_width: 480,
            set_content_height: 500,
            #[watch]
            set_title: &t!("groups.add_nodes"),
            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        #[watch]
                        set_label: &t!("dialog.cancel"),
                        connect_clicked => PickerMsg::Close,
                    },
                    pack_end = &gtk::Button {
                        add_css_class: "suggested-action",
                        #[watch]
                        set_label: &t!("groups.add_selected"),
                        #[watch]
                        set_sensitive: !model.busy && model.selected > 0,
                        connect_clicked => PickerMsg::Add,
                    },
                },
                add_top_bar = &gtk::Box {
                    set_margin_all: 12,
                    #[local_ref]
                    search -> gtk::SearchEntry {
                        set_hexpand: true,
                        #[watch]
                        set_placeholder_text: Some(t!("groups.search_nodes").as_ref()),
                    },
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    adw::Banner {
                        #[watch]
                        set_title: &t!("groups.nodes_error"),
                        #[watch]
                        set_revealed: model.error.is_some(),
                        #[watch]
                        set_button_label: Some(t!("error.details").as_ref()),
                        connect_button_clicked => PickerMsg::ShowError,
                    },
                    gtk::Button {
                        set_halign: gtk::Align::Center,
                        #[watch]
                        set_label: &t!("auth.retry"),
                        #[watch]
                        set_visible: model.error.is_some(),
                        connect_clicked => PickerMsg::Load,
                    },
                    gtk::Spinner {
                        set_margin_all: 12,
                        #[watch]
                        set_visible: model.busy,
                        #[watch]
                        set_spinning: model.busy,
                    },
                    adw::StatusPage {
                        set_icon_name: Some("edit-find-symbolic"),
                        #[watch]
                        set_title: &t!("groups.no_available_nodes"),
                        #[watch]
                        set_visible: !model.busy && model.error.is_none() && model.matches == 0,
                    },
                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        set_margin_all: 12,
                        #[local_ref]
                        list -> gtk::ListBox {
                            set_valign: gtk::Align::Start,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                        },
                    },
                },
            },
        }
    }

    fn init(
        (api, excluded): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let error_dialog = adw::AlertDialog::new(None, None);
        error_dialog.add_response("close", &t!("dialog.close"));

        let search = gtk::SearchEntry::new();
        let input = sender.clone();

        let search_handler = search.connect_search_changed(move |_| input.input(PickerMsg::Filter));

        let model = Self {
            api,
            excluded,
            choices: Vec::new(),
            list: gtk::ListBox::new(),
            search,
            search_handler: Some(search_handler),
            busy: true,
            selected: 0,
            matches: 0,
            error: None,
            error_dialog,
        };

        let list = &model.list;
        let search = &model.search;

        let widgets = view_output!();

        sender.input(PickerMsg::Load);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: PickerMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            PickerMsg::Load => {
                self.busy = true;
                self.error = None;

                let api = self.api.clone();

                sender.oneshot_command(async move { api.nodes().await });
            }
            PickerMsg::Filter => {
                let text = self.search.text().to_lowercase();

                self.matches = 0;

                for (node, row, _) in &self.choices {
                    let visible = [
                        &node.name,
                        node.tag.as_deref().unwrap_or(""),
                        &node.protocol,
                    ]
                    .iter()
                    .any(|s| s.to_lowercase().contains(&text));

                    row.set_visible(visible);

                    self.matches += usize::from(visible);
                }
            }
            PickerMsg::Changed => {
                self.selected = self
                    .choices
                    .iter()
                    .filter(|(_, _, check)| check.is_active())
                    .count()
            }
            PickerMsg::Add => {
                sender
                    .output(PickerOutput::Added(
                        self.choices
                            .iter()
                            .filter(|(_, _, check)| check.is_active())
                            .map(|(node, _, _)| node.clone())
                            .collect(),
                    ))
                    .unwrap();

                root.close();
            }
            PickerMsg::Close => {
                root.close();
            }
            PickerMsg::ShowError => {
                if let Some(error) = &self.error {
                    self.error_dialog
                        .show_error(&t!("groups.nodes_error"), error, root);
                }
            }
            PickerMsg::Retranslate => {}
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
            Ok(nodes) => {
                self.list.remove_all();
                self.choices.clear();

                for node in nodes
                    .into_iter()
                    .filter(|node| !self.excluded.contains(&node.id))
                {
                    let row = node_row(&node.name, node.tag.as_deref(), &node.protocol);

                    let check = gtk::CheckButton::builder()
                        .valign(gtk::Align::Center)
                        .build();

                    row.add_prefix(&check);
                    row.set_activatable_widget(Some(&check));

                    let input = sender.input_sender().clone();

                    check.connect_toggled(move |_| input.emit(PickerMsg::Changed));

                    self.list.append(&row);
                    self.choices.push((node, row, check));
                }

                sender.input(PickerMsg::Filter);
            }
            Err(ApiError::Unauthorized) => {
                sender.output(PickerOutput::Unauthorized).unwrap();

                root.close();
            }
            Err(error) => {
                tracing::warn!(%error, "Could not load group node choices");

                self.error = Some(error);
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        self.search.disconnect(self.search_handler.take().unwrap());

        if self.error_dialog.parent().is_some() {
            self.error_dialog.force_close();
        }
    }
}
