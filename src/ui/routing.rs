mod dialogs;
mod editor;
mod presets;
mod rules;

use adw::prelude::*;
use gtk::glib;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use crate::{
    api::{
        Api,
        routing::{Routing, RoutingCatalog},
    },
    error::ApiError,
    ui::error_dialog::DialogExt,
};
use editor::{Editor, EditorMsg, EditorOutput};

pub struct Routings {
    api: Api,
    records: Vec<Routing>,
    busy: bool,
    error: Option<ApiError>,
    list: gtk::ListBox,
    group_names: Vec<String>,
    details: Option<adw::Dialog>,
    error_dialog: adw::AlertDialog,
    action_dialog: Option<adw::AlertDialog>,
    editor: Option<Controller<Editor>>,
}

#[derive(Clone, Debug)]
pub enum RoutingMsg {
    Refresh,
    Select(String),
    Details(String),
    ShowError,
    Retranslate,
    ShowRename(String),
    Rename(String, String),
    Duplicate(String),
    ShowDelete(String),
    Delete(String),
    New,
    Edit(String),
    Editor(EditorOutput),
}

enum Operation {
    Refresh,
    Select(String),
    Rename(String, String),
    Duplicate(Box<Routing>),
    Delete(String),
}

#[derive(Debug)]
pub struct RoutingUpdate {
    catalog: Result<RoutingCatalog, ApiError>,
    mutation_error: Option<ApiError>,
}

#[relm4::component(pub)]
impl Component for Routings {
    type Init = Api;
    type Input = RoutingMsg;
    type Output = ();
    type CommandOutput = RoutingUpdate;

    view! {
        adw::NavigationView {
            connect_map => RoutingMsg::Refresh,
            add = &adw::NavigationPage {
                set_tag: Some("routing-list"),
                #[watch]
                set_title: t!("navigation.routing").as_ref(),
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    adw::Banner {
                        #[watch]
                        set_title: t!("routing.request_error").as_ref(),
                        #[watch]
                        set_button_label: Some(t!("error.details").as_ref()),
                        #[watch]
                        set_revealed: model.error.is_some(),
                        connect_button_clicked => RoutingMsg::ShowError,
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
                                    set_spacing: 12,
                                    gtk::Label {
                                        set_xalign: 0.0,
                                        set_hexpand: true,
                                        add_css_class: "title-1",
                                        #[watch]
                                        set_label: t!("navigation.routing").as_ref(),
                                    },
                                    gtk::Spinner {
                                        #[watch]
                                        set_spinning: model.busy,
                                        #[watch]
                                        set_visible: model.busy,
                                    },
                                    gtk::Button {
                                        set_icon_name: "list-add-symbolic",
                                        add_css_class: "flat",
                                        #[watch]
                                        set_tooltip_text: Some(t!("routing.new_title").as_ref()),
                                        #[watch]
                                        set_sensitive: !model.busy,
                                        connect_clicked => RoutingMsg::New,
                                    },
                                    gtk::Button {
                                        set_icon_name: "view-refresh-symbolic",
                                        add_css_class: "flat",
                                        #[watch]
                                        set_tooltip_text: Some(t!("routing.refresh").as_ref()),
                                        #[watch]
                                        set_sensitive: !model.busy,
                                        connect_clicked => RoutingMsg::Refresh,
                                    },
                                },
                                #[local_ref]
                                list -> gtk::ListBox {
                                    add_css_class: "boxed-list",
                                    set_selection_mode: gtk::SelectionMode::None,
                                    #[watch]
                                    set_sensitive: !model.busy,
                                },
                                adw::StatusPage {
                                    set_icon_name: Some("preferences-system-symbolic"),
                                    #[watch]
                                    set_title: t!("routing.empty").as_ref(),
                                    #[watch]
                                    set_visible: model.records.is_empty() && !model.busy && model.error.is_none(),
                                },
                            },
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
            busy: false,
            error: None,
            list: gtk::ListBox::new(),
            group_names: Vec::new(),
            details: None,
            error_dialog,
            action_dialog: None,
            editor: None,
        };

        let list = &model.list;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: RoutingMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        let operation = match message {
            RoutingMsg::New | RoutingMsg::Edit(_) => {
                let routing = if let RoutingMsg::Edit(id) = message {
                    Some(
                        self.records
                            .iter()
                            .find(|routing| routing.id == id)
                            .unwrap()
                            .clone(),
                    )
                } else {
                    None
                };

                let editor = Editor::builder()
                    .launch((self.api.clone(), routing, self.group_names.clone()))
                    .forward(sender.input_sender(), RoutingMsg::Editor);

                editor.widget().present(Some(root));
                self.editor = Some(editor);

                return;
            }
            RoutingMsg::Editor(output) => match output {
                EditorOutput::Closed => {
                    self.editor.take();
                    return;
                }
                EditorOutput::Unauthorized => {
                    sender.output(()).unwrap();
                    return;
                }
                EditorOutput::Saved => Operation::Refresh,
            },
            RoutingMsg::ShowRename(ref id) | RoutingMsg::ShowDelete(ref id) => {
                let routing = self
                    .records
                    .iter()
                    .find(|routing| &routing.id == id)
                    .unwrap();

                let dialog = if matches!(message, RoutingMsg::ShowRename(_)) {
                    routing.rename(sender.input_sender().clone())
                } else {
                    routing.delete(sender.input_sender().clone())
                };

                dialog.present(Some(root));
                self.action_dialog = Some(dialog);

                return;
            }
            RoutingMsg::Details(id) => {
                let routing = self.records.iter().find(|r| r.id == id).unwrap();
                let dialog = routing.details();

                dialog.present(Some(root));
                self.details = Some(dialog);

                return;
            }
            RoutingMsg::ShowError => {
                self.show_error(root);

                return;
            }
            RoutingMsg::Retranslate => {
                if let Some(editor) = &self.editor {
                    editor.emit(EditorMsg::Retranslate);
                }

                self.render(&sender);

                return;
            }
            RoutingMsg::Refresh => Operation::Refresh,
            RoutingMsg::Select(id) => Operation::Select(id),
            RoutingMsg::Rename(id, name) => Operation::Rename(id, name),
            RoutingMsg::Delete(id) => Operation::Delete(id),
            RoutingMsg::Duplicate(id) => Operation::Duplicate(Box::new(
                self.records
                    .iter()
                    .find(|routing| routing.id == id)
                    .unwrap()
                    .clone(),
            )),
        };

        if self.busy {
            return;
        }

        self.busy = true;
        self.error = None;

        let api = self.api.clone();

        sender.oneshot_command(async move {
            let mutation_error = match operation {
                Operation::Refresh => Ok(()),
                Operation::Select(id) => api.select_routing(id).await,
                Operation::Rename(id, name) => api.rename_routing(id, name).await,
                Operation::Delete(id) => api.remove_routing(id).await,
                Operation::Duplicate(routing) => {
                    api.create_routing(format!("{} (Copy)", routing.name), routing.routing.string)
                        .await
                }
            }
            .err();

            RoutingUpdate {
                catalog: api.routings().await,
                mutation_error,
            }
        });
    }

    fn update_cmd(
        &mut self,
        result: RoutingUpdate,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.busy = false;

        let mutation_failed = result.mutation_error.is_some();
        let unauthorized = matches!(&result.mutation_error, Some(ApiError::Unauthorized))
            || matches!(&result.catalog, Err(ApiError::Unauthorized));

        self.error = result.mutation_error;

        match result.catalog {
            Ok(catalog) => {
                self.records = catalog.routings;
                self.group_names = catalog.groups.into_iter().map(|g| g.name).collect();
            }
            Err(error) => {
                tracing::warn!(endpoint = %self.api.endpoint(), %error, "Could not read routings");

                if self.error.is_none() {
                    self.error = Some(error);
                }
            }
        }

        self.render(&sender);

        if let Some(error) = &self.error {
            tracing::warn!(endpoint = %self.api.endpoint(), %error, "Routing request failed");
        }

        if unauthorized {
            sender.output(()).unwrap();
        } else if mutation_failed {
            self.show_error(root);
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        if let Some(editor) = &self.editor {
            editor.widget().force_close();
        }

        if let Some(details) = &self.details {
            details.force_close();
        }

        if let Some(dialog) = &self.action_dialog
            && dialog.parent().is_some()
        {
            dialog.force_close();
        }

        if self.error_dialog.parent().is_some() {
            self.error_dialog.force_close();
        }
    }
}

impl Routings {
    fn render(&mut self, sender: &ComponentSender<Self>) {
        self.list.remove_all();

        let mut group = None;

        self.records.iter().for_each(|routing| {
            let row = adw::ActionRow::builder()
                .title(&routing.name)
                .use_markup(false)
                .activatable(!routing.selected)
                .build();

            let radio = gtk::CheckButton::builder()
                .can_target(false)
                .focusable(false)
                .build();

            if let Some(first) = &group {
                radio.set_group(Some(first));
            } else {
                group = Some(radio.clone());
            }

            radio.set_active(routing.selected);
            row.add_prefix(&radio);

            let input = sender.input_sender().clone();
            let id = routing.id.clone();

            row.connect_activated(move |_| input.emit(RoutingMsg::Select(id.clone())));

            let edit = gtk::Button::builder()
                .icon_name("document-edit-symbolic")
                .tooltip_text(t!("routing.edit").as_ref())
                .valign(gtk::Align::Center)
                .build();

            edit.add_css_class("flat");

            let id = routing.id.clone();
            let input = sender.input_sender().clone();

            edit.connect_clicked(move |_| input.emit(RoutingMsg::Edit(id.clone())));
            row.add_suffix(&edit);

            let popover = gtk::Popover::new();
            let actions = gtk::Box::new(gtk::Orientation::Vertical, 0);

            let details = gtk::Button::with_label(t!("config.details").as_ref());
            details.add_css_class("flat");

            let input = sender.input_sender().clone();
            let id = routing.id.clone();

            details.connect_clicked(gtk::glib::clone!(
                #[weak]
                popover,
                move |_| {
                    popover.popdown();
                    input.emit(RoutingMsg::Details(id.clone()));
                }
            ));

            actions.append(&details);

            [
                ("config.rename", RoutingMsg::ShowRename(routing.id.clone())),
                (
                    "config.duplicate",
                    RoutingMsg::Duplicate(routing.id.clone()),
                ),
                ("config.delete", RoutingMsg::ShowDelete(routing.id.clone())),
            ]
            .into_iter()
            .for_each(|(key, message)| {
                let button = gtk::Button::with_label(t!(key).as_ref());
                button.add_css_class("flat");

                if key == "config.delete" {
                    button.set_sensitive(!routing.selected);
                    button.add_css_class("destructive-action");
                }

                let input = sender.input_sender().clone();
                button.connect_clicked(glib::clone!(
                    #[weak]
                    popover,
                    move |_| {
                        popover.popdown();
                        input.emit(message.clone());
                    }
                ));

                actions.append(&button);
            });

            popover.set_child(Some(&actions));

            let more = gtk::MenuButton::builder()
                .icon_name("view-more-symbolic")
                .tooltip_text(t!("routing.more").as_ref())
                .popover(&popover)
                .valign(gtk::Align::Center)
                .build();

            more.add_css_class("flat");
            row.add_suffix(&more);

            self.list.append(&row);
        });
    }

    fn show_error(&self, parent: &adw::NavigationView) {
        if let Some(error) = &self.error {
            self.error_dialog
                .show_error(t!("routing.request_error").as_ref(), error, parent);
        }
    }
}
