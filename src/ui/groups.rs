mod dialogs;
mod editor;
mod identity;
mod picker;
mod row;

use crate::{
    api::{
        Api,
        group::{Group, Policy},
    },
    error::ApiError,
    ui::error_dialog::DialogExt,
};

use adw::prelude::*;
use editor::{Editor, EditorMsg, EditorOutput};
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

pub struct Groups {
    api: Api,
    records: Vec<Group>,
    list: gtk::ListBox,
    busy: bool,
    error: Option<ApiError>,
    error_dialog: adw::AlertDialog,
    delete_dialog: Option<adw::AlertDialog>,
    create: Option<dialogs::NewGroup>,
    editor: Option<Controller<Editor>>,
}

#[derive(Debug)]
pub enum GroupMsg {
    Refresh,
    New,
    Create(String, Policy),
    ConfirmDelete(String),
    Delete(String),
    Edit(String),
    Editor(EditorOutput),
    EditorPopped,
    ShowError,
    Retranslate,
}

enum Operation {
    Refresh,
    Create(String, Policy),
    Delete(String),
}

#[derive(Debug)]
pub struct GroupReply {
    groups: Result<Vec<Group>, ApiError>,
    error: Option<ApiError>,
    created: bool,
}

#[relm4::component(pub)]
impl Component for Groups {
    type Init = Api;
    type Input = GroupMsg;
    type Output = ();
    type CommandOutput = GroupReply;
    view! {
        adw::NavigationView {
            connect_map => GroupMsg::Refresh,
            connect_popped[sender] => move |_, page| {
                if page.tag().as_deref() == Some("group-editor") { sender.input(GroupMsg::EditorPopped); }
            },
            add = &adw::NavigationPage {
                set_tag: Some("group-list"),
                #[watch]
                set_title: &t!("navigation.groups"),
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    adw::Banner {
                        #[watch]
                        set_title: &t!("groups.request_error"),
                        #[watch]
                        set_button_label: Some(t!("error.details").as_ref()),
                        #[watch]
                        set_revealed: model.error.is_some(),
                        connect_button_clicked => GroupMsg::ShowError,
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
                                        set_label: &t!("navigation.groups"),
                                    },
                                    gtk::Spinner {
                                        #[watch]
                                        set_spinning: model.busy,
                                        #[watch]
                                        set_visible: model.busy,
                                    },
                                    gtk::Button {
                                        set_icon_name: "view-refresh-symbolic",
                                        add_css_class: "flat",
                                        #[watch]
                                        set_tooltip_text: Some(t!("groups.refresh").as_ref()),
                                        #[watch]
                                        set_sensitive: !model.busy,
                                        connect_clicked => GroupMsg::Refresh,
                                    },
                                    gtk::Button {
                                        set_icon_name: "list-add-symbolic",
                                        add_css_class: "flat",
                                        #[watch]
                                        set_tooltip_text: Some(t!("groups.add").as_ref()),
                                        #[watch]
                                        set_sensitive: !model.busy,
                                        connect_clicked => GroupMsg::New,
                                    },
                                },
                                #[local_ref]
                                list -> gtk::ListBox {
                                    set_selection_mode: gtk::SelectionMode::None,
                                    add_css_class: "boxed-list",
                                    #[watch]
                                    set_sensitive: !model.busy,
                                },
                                adw::StatusPage {
                                    set_icon_name: Some("view-list-symbolic"),
                                    #[watch]
                                    set_title: &t!("groups.empty"),
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

    fn init(
        api: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let error_dialog = adw::AlertDialog::new(None, None);

        error_dialog.add_response("close", &t!("dialog.close"));

        let model = Self {
            api,
            records: Vec::new(),
            list: gtk::ListBox::new(),
            busy: false,
            error: None,
            error_dialog,
            delete_dialog: None,
            create: None,
            editor: None,
        };

        let list = &model.list;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: GroupMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        let operation = match message {
            GroupMsg::New => {
                let dialog = dialogs::NewGroup::new(sender.input_sender().clone());

                dialog.dialog.present(Some(root));
                self.create = Some(dialog);

                return;
            }
            GroupMsg::ConfirmDelete(id) => {
                let group = self.records.iter().find(|g| g.id == id).unwrap();
                let dialog = dialogs::delete(group, sender.input_sender().clone());

                dialog.present(Some(root));
                self.delete_dialog = Some(dialog);

                return;
            }
            GroupMsg::Edit(id) => {
                let group = self.records.iter().find(|g| g.id == id).unwrap().clone();

                let editor = Editor::builder()
                    .launch((self.api.clone(), group))
                    .forward(sender.input_sender(), GroupMsg::Editor);

                root.push(editor.widget());
                self.editor = Some(editor);

                return;
            }
            GroupMsg::Editor(output) => {
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
            GroupMsg::EditorPopped => {
                self.editor.take();

                return;
            }
            GroupMsg::ShowError => {
                self.show_error(root);

                return;
            }
            GroupMsg::Retranslate => {
                if let Some(editor) = &self.editor {
                    editor.emit(EditorMsg::Retranslate);
                }

                if let Some(create) = &self.create {
                    create.retranslate();
                }

                self.render(sender.input_sender());

                return;
            }
            GroupMsg::Refresh => Operation::Refresh,
            GroupMsg::Create(name, policy) => Operation::Create(name, policy),
            GroupMsg::Delete(id) => Operation::Delete(id),
        };

        if self.busy {
            return;
        }

        self.busy = true;
        self.error = None;

        if let Some(create) = &self.create {
            create.set_busy(true);
        }

        let api = self.api.clone();

        sender.oneshot_command(async move {
            let created = matches!(&operation, Operation::Create(..));

            let error = match operation {
                Operation::Refresh => Ok(()),
                Operation::Create(name, policy) => api.create_group(name, policy).await,
                Operation::Delete(id) => api.remove_group(id).await,
            }
            .err();

            GroupReply {
                groups: api.groups().await,
                error,
                created,
            }
        });
    }

    fn update_cmd(&mut self, reply: GroupReply, sender: ComponentSender<Self>, root: &Self::Root) {
        self.busy = false;

        if let Some(create) = &self.create {
            create.set_busy(false);

            if reply.created && reply.error.is_none() {
                create.dialog.close();
            }
        }

        let unauthorized = matches!(reply.error, Some(ApiError::Unauthorized))
            || matches!(reply.groups, Err(ApiError::Unauthorized));

        let mutation_failed = reply.error.is_some();

        self.error = reply.error;

        match reply.groups {
            Ok(groups) => {
                self.records = groups;
                self.render(sender.input_sender());
            }

            Err(error) => {
                if self.error.is_none() {
                    self.error = Some(error);
                }
            }
        }

        if let Some(error) = &self.error {
            tracing::warn!(%error, "Group request failed");
        }

        if unauthorized {
            sender.output(()).unwrap();
        } else if mutation_failed {
            self.show_error(root);
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        if self.error_dialog.parent().is_some() {
            self.error_dialog.force_close();
        }

        if let Some(dialog) = &self.delete_dialog
            && dialog.parent().is_some()
        {
            dialog.force_close();
        }

        if let Some(create) = &self.create
            && create.dialog.parent().is_some()
        {
            create.dialog.force_close();
        }
    }
}
impl Groups {
    fn render(&self, input: &Sender<GroupMsg>) {
        self.list.remove_all();

        for group in &self.records {
            self.list.append(&row::group_row(group, input));
        }
    }

    fn show_error(&self, root: &adw::NavigationView) {
        if let Some(error) = &self.error {
            let parent: &gtk::Widget = self
                .create
                .as_ref()
                .filter(|c| c.dialog.parent().is_some())
                .map(|c| c.dialog.upcast_ref())
                .unwrap_or(root.upcast_ref());

            self.error_dialog
                .show_error(&t!("groups.request_error"), error, parent);
        }
    }
}
