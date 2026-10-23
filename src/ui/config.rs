mod choice_list;
mod details;
mod dialogs;
mod editor;
mod field_note;
mod fields;
mod input_list;
mod interface_list;

use adw::prelude::*;
use gtk::glib;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use crate::{
    api::{Api, config::Config},
    error::ApiError,
    ui::error_dialog::DialogExt,
};
use details::Details;
use editor::{Editor, EditorMsg, EditorOutput};

pub struct Configs {
    api: Api,
    records: Vec<Config>,
    busy: bool,
    error: Option<ApiError>,
    list: gtk::ListBox,
    details: Details,
    detail_id: Option<String>,
    error_dialog: adw::AlertDialog,
    action_dialog: Option<adw::AlertDialog>,
    editor: Option<Controller<Editor>>,
}

#[derive(Clone, Debug)]
pub enum ConfigMsg {
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
    EditorPopped,
}

enum Operation {
    Refresh,
    Select(String),
    Rename(String, String),
    Duplicate(Box<Config>),
    Delete(String),
}

#[derive(Debug)]
pub struct ConfigUpdate {
    records: Result<Vec<Config>, ApiError>,
    mutation_error: Option<ApiError>,
}

#[relm4::component(pub)]
impl Component for Configs {
    type Init = Api;
    type Input = ConfigMsg;
    type Output = ();
    type CommandOutput = ConfigUpdate;

    view! {
        adw::NavigationView {
            connect_popped[sender] => move |_, page| {
                if page.tag().as_deref() == Some("config-editor") {
                    sender.input(ConfigMsg::EditorPopped);
                }
            },
            add = &adw::NavigationPage {
                set_tag: Some("config-list"),
                #[watch]
                set_title: t!("navigation.configurations").as_ref(),
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    adw::Banner {
                        #[watch]
                        set_title: t!("config.request_error").as_ref(),
                        #[watch]
                        set_button_label: Some(t!("error.details").as_ref()),
                        #[watch]
                        set_revealed: model.error.is_some(),
                        connect_button_clicked => ConfigMsg::ShowError,
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
                                        set_label: t!("navigation.configurations").as_ref(),
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
                                        set_tooltip_text: Some(t!("config.new_title").as_ref()),
                                        #[watch]
                                        set_sensitive: !model.busy,
                                        connect_clicked => ConfigMsg::New,
                                    },
                                    gtk::Button {
                                        set_icon_name: "view-refresh-symbolic",
                                        add_css_class: "flat",
                                        #[watch]
                                        set_tooltip_text: Some(t!("config.refresh").as_ref()),
                                        #[watch]
                                        set_sensitive: !model.busy,
                                        connect_clicked => ConfigMsg::Refresh,
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
                                    set_title: t!("config.empty").as_ref(),
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
            details: Details::new(),
            detail_id: None,
            error_dialog,
            action_dialog: None,
            editor: None,
        };

        let list = &model.list;
        let widgets = view_output!();

        root.add(&model.details.page);
        sender.input(ConfigMsg::Refresh);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: ConfigMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        let operation = match message {
            ConfigMsg::New | ConfigMsg::Edit(_) => {
                let config = if let ConfigMsg::Edit(id) = message {
                    Some(
                        self.records
                            .iter()
                            .find(|config| config.id == id)
                            .unwrap()
                            .clone(),
                    )
                } else {
                    None
                };

                let editor = Editor::builder()
                    .launch((self.api.clone(), config))
                    .forward(sender.input_sender(), ConfigMsg::Editor);

                root.push(editor.widget());
                self.editor = Some(editor);

                return;
            }
            ConfigMsg::Editor(output) => {
                self.editor.as_ref().unwrap().widget().set_can_pop(true);
                root.pop();

                match output {
                    EditorOutput::Closed => return,
                    EditorOutput::Unauthorized => {
                        sender.output(()).unwrap();
                        return;
                    }
                    EditorOutput::Saved => Operation::Refresh,
                }
            }
            ConfigMsg::EditorPopped => {
                self.editor.take();

                return;
            }
            ConfigMsg::ShowRename(ref id) | ConfigMsg::ShowDelete(ref id) => {
                let config = self.records.iter().find(|config| &config.id == id).unwrap();

                let dialog = if matches!(message, ConfigMsg::ShowRename(_)) {
                    dialogs::rename(config, sender.input_sender().clone())
                } else {
                    dialogs::delete(config, sender.input_sender().clone())
                };

                dialog.present(Some(root));
                self.action_dialog = Some(dialog);

                return;
            }
            ConfigMsg::Details(id) => {
                let config = self.records.iter().find(|config| config.id == id).unwrap();

                self.details.refresh(config);
                self.detail_id = Some(id);

                root.push(&self.details.page);

                return;
            }
            ConfigMsg::ShowError => {
                self.show_error(root);

                return;
            }
            ConfigMsg::Retranslate => {
                if let Some(editor) = &self.editor {
                    editor.emit(EditorMsg::Retranslate);
                }

                self.render(&sender, root);

                return;
            }
            ConfigMsg::Refresh => Operation::Refresh,
            ConfigMsg::Select(id) => Operation::Select(id),
            ConfigMsg::Rename(id, name) => Operation::Rename(id, name),
            ConfigMsg::Delete(id) => Operation::Delete(id),
            ConfigMsg::Duplicate(id) => Operation::Duplicate(Box::new(
                self.records
                    .iter()
                    .find(|config| config.id == id)
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
                Operation::Select(id) => api.select_config(id).await,
                Operation::Rename(id, name) => api.rename_config(id, name).await,
                Operation::Delete(id) => api.remove_config(id).await,
                Operation::Duplicate(config) => api.duplicate_config(*config).await,
            }
            .err();

            ConfigUpdate {
                records: api.configs().await,
                mutation_error,
            }
        });
    }

    fn update_cmd(
        &mut self,
        result: ConfigUpdate,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.busy = false;
        let mutation_failed = result.mutation_error.is_some();
        let unauthorized = matches!(&result.mutation_error, Some(ApiError::Unauthorized))
            || matches!(&result.records, Err(ApiError::Unauthorized));

        self.error = result.mutation_error;

        match result.records {
            Ok(records) => self.records = records,
            Err(error) => {
                tracing::warn!(endpoint = %self.api.endpoint(), %error, "Could not read configurations");

                if self.error.is_none() {
                    self.error = Some(error);
                }
            }
        }

        self.render(&sender, root);

        if let Some(error) = &self.error {
            tracing::warn!(endpoint = %self.api.endpoint(), %error, "Config request failed");
        }

        if unauthorized {
            sender.output(()).unwrap();
        } else if mutation_failed {
            self.show_error(root);
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        if let Some(dialog) = &self.action_dialog
            && dialog.parent().is_some()
        {
            dialog.close();
        }

        if self.error_dialog.parent().is_some() {
            self.error_dialog.close();
        }
    }
}

impl Configs {
    fn render(&mut self, sender: &ComponentSender<Self>, root: &adw::NavigationView) {
        self.list.remove_all();

        let mut group = None;

        self.records.iter().for_each(|config| {
            let row = adw::ActionRow::builder()
                .title(&config.name)
                .use_markup(false)
                .activatable(!config.selected)
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

            radio.set_active(config.selected);
            row.add_prefix(&radio);

            let input = sender.input_sender().clone();
            let id = config.id.clone();

            row.connect_activated(move |_| input.emit(ConfigMsg::Select(id.clone())));

            let edit = gtk::Button::builder()
                .icon_name("document-edit-symbolic")
                .tooltip_text(t!("config.edit").as_ref())
                .valign(gtk::Align::Center)
                .build();

            edit.add_css_class("flat");

            let id = config.id.clone();
            let input = sender.input_sender().clone();

            edit.connect_clicked(move |_| input.emit(ConfigMsg::Edit(id.clone())));
            row.add_suffix(&edit);

            let popover = gtk::Popover::new();
            let actions = gtk::Box::new(gtk::Orientation::Vertical, 0);

            let details = gtk::Button::with_label(t!("config.details").as_ref());
            details.add_css_class("flat");

            let input = sender.input_sender().clone();
            let id = config.id.clone();

            details.connect_clicked(gtk::glib::clone!(
                #[weak]
                popover,
                move |_| {
                    popover.popdown();
                    input.emit(ConfigMsg::Details(id.clone()));
                }
            ));

            actions.append(&details);

            [
                ("config.rename", ConfigMsg::ShowRename(config.id.clone())),
                ("config.duplicate", ConfigMsg::Duplicate(config.id.clone())),
                ("config.delete", ConfigMsg::ShowDelete(config.id.clone())),
            ]
            .into_iter()
            .for_each(|(key, message)| {
                let button = gtk::Button::with_label(t!(key).as_ref());
                button.add_css_class("flat");

                if key == "config.delete" {
                    button.set_sensitive(!config.selected);
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
                .tooltip_text(t!("config.more").as_ref())
                .popover(&popover)
                .valign(gtk::Align::Center)
                .build();

            more.add_css_class("flat");
            row.add_suffix(&more);

            self.list.append(&row);
        });

        if let Some(id) = &self.detail_id {
            if let Some(config) = self.records.iter().find(|config| &config.id == id) {
                self.details.refresh(config);
            } else {
                root.pop_to_tag("config-list");
                self.detail_id = None;
            }
        }
    }

    fn show_error(&self, parent: &adw::NavigationView) {
        if let Some(error) = &self.error {
            self.error_dialog
                .show_error(t!("config.request_error").as_ref(), error, parent);
        }
    }
}
