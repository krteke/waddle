use super::ServerEvent;
mod dialogs;
mod editor;
mod fields;
mod form;

use adw::prelude::*;
use gtk::glib;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

use crate::{
    api::{Api, dns::Dns},
    error::ApiError,
    ui::error_dialog::DialogExt,
};
use editor::{Editor, EditorMsg, EditorOutput};

pub struct DnsConfigs {
    api: Api,
    records: Vec<Dns>,
    busy: bool,
    error: Option<ApiError>,
    list: gtk::ListBox,
    details: Option<adw::Dialog>,
    error_dialog: adw::AlertDialog,
    action_dialog: Option<adw::AlertDialog>,
    editor: Option<Controller<Editor>>,
}

#[derive(Clone, Debug)]
pub enum DnsMsg {
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
    Duplicate(Box<Dns>),
    Delete(String),
}

#[derive(Debug)]
pub struct DnsUpdate {
    catalog: Result<Vec<Dns>, ApiError>,
    mutation_error: Option<ApiError>,
}

#[relm4::component(pub)]
impl Component for DnsConfigs {
    type Init = Api;
    type Input = DnsMsg;
    type Output = ServerEvent;
    type CommandOutput = DnsUpdate;

    view! {
        adw::NavigationView {
            connect_map => DnsMsg::Refresh,
            connect_popped[sender] => move |_, page| {
                if page.tag().as_deref() == Some("dns-editor") { sender.input(DnsMsg::EditorPopped); }
            },
            add = &adw::NavigationPage {
                set_tag: Some("dns-list"),
                #[watch]
                set_title: t!("navigation.dns").as_ref(),
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    adw::Banner {
                        #[watch]
                        set_title: t!("dns.request_error").as_ref(),
                        #[watch]
                        set_button_label: Some(t!("error.details").as_ref()),
                        #[watch]
                        set_revealed: model.error.is_some(),
                        connect_button_clicked => DnsMsg::ShowError,
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
                                        set_label: t!("navigation.dns").as_ref(),
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
                                        set_tooltip_text: Some(t!("dns.new_title").as_ref()),
                                        #[watch]
                                        set_sensitive: !model.busy,
                                        connect_clicked => DnsMsg::New,
                                    },
                                    gtk::Button {
                                        set_icon_name: "view-refresh-symbolic",
                                        add_css_class: "flat",
                                        #[watch]
                                        set_tooltip_text: Some(t!("dns.refresh").as_ref()),
                                        #[watch]
                                        set_sensitive: !model.busy,
                                        connect_clicked => DnsMsg::Refresh,
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
                                    set_title: t!("dns.empty").as_ref(),
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
            details: None,
            error_dialog,
            action_dialog: None,
            editor: None,
        };

        let list = &model.list;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: DnsMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        let operation = match message {
            DnsMsg::New | DnsMsg::Edit(_) => {
                let dns = if let DnsMsg::Edit(id) = message {
                    Some(
                        self.records
                            .iter()
                            .find(|dns| dns.id == id)
                            .unwrap()
                            .clone(),
                    )
                } else {
                    None
                };

                let editor = Editor::builder()
                    .launch((self.api.clone(), dns))
                    .forward(sender.input_sender(), DnsMsg::Editor);

                root.push(editor.widget());
                self.editor = Some(editor);

                return;
            }
            DnsMsg::EditorPopped => {
                self.editor.take();
                return;
            }
            DnsMsg::Editor(output) => {
                self.editor.as_ref().unwrap().widget().set_can_pop(true);
                root.pop();
                match output {
                    EditorOutput::Closed => {
                        return;
                    }
                    EditorOutput::Unauthorized => {
                        sender.output(ServerEvent::Unauthorized).unwrap();
                        return;
                    }
                    EditorOutput::Saved => Operation::Refresh,
                }
            }
            DnsMsg::ShowRename(ref id) | DnsMsg::ShowDelete(ref id) => {
                let dns = self.records.iter().find(|dns| &dns.id == id).unwrap();

                let dialog = if matches!(message, DnsMsg::ShowRename(_)) {
                    dns.rename(sender.input_sender().clone())
                } else {
                    dns.delete(sender.input_sender().clone())
                };

                dialog.present(Some(root));
                self.action_dialog = Some(dialog);

                return;
            }
            DnsMsg::Details(id) => {
                let dns = self.records.iter().find(|r| r.id == id).unwrap();
                let dialog = dns.details();

                dialog.present(Some(root));
                self.details = Some(dialog);

                return;
            }
            DnsMsg::ShowError => {
                self.show_error(root);

                return;
            }
            DnsMsg::Retranslate => {
                if let Some(editor) = &self.editor {
                    editor.emit(EditorMsg::Retranslate);
                }

                self.render(&sender);

                return;
            }
            DnsMsg::Refresh => Operation::Refresh,
            DnsMsg::Select(id) => Operation::Select(id),
            DnsMsg::Rename(id, name) => Operation::Rename(id, name),
            DnsMsg::Delete(id) => Operation::Delete(id),
            DnsMsg::Duplicate(id) => Operation::Duplicate(Box::new(
                self.records
                    .iter()
                    .find(|dns| dns.id == id)
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
                Operation::Select(id) => api.select_dns(id).await,
                Operation::Rename(id, name) => api.rename_dns(id, name).await,
                Operation::Delete(id) => api.remove_dns(id).await,
                Operation::Duplicate(dns) => {
                    api.create_dns(format!("{} (Copy)", dns.name), dns.dns.string)
                        .await
                }
            }
            .err();

            DnsUpdate {
                catalog: api.dnss().await,
                mutation_error,
            }
        });
    }

    fn update_cmd(&mut self, result: DnsUpdate, sender: ComponentSender<Self>, root: &Self::Root) {
        self.busy = false;

        let mutation_failed = result.mutation_error.is_some();
        let unauthorized = matches!(&result.mutation_error, Some(ApiError::Unauthorized))
            || matches!(&result.catalog, Err(ApiError::Unauthorized));

        self.error = result.mutation_error;

        match result.catalog {
            Ok(catalog) => {
                self.records = catalog;
            }
            Err(error) => {
                tracing::warn!(endpoint = %self.api.endpoint(), %error, "Could not read dnss");

                if self.error.is_none() {
                    self.error = Some(error);
                }
            }
        }

        self.render(&sender);

        if let Some(error) = &self.error {
            tracing::warn!(endpoint = %self.api.endpoint(), %error, "Dns request failed");
        }

        if unauthorized {
            sender.output(ServerEvent::Unauthorized).unwrap();
        } else {
            sender.output(ServerEvent::Changed).unwrap();
            if mutation_failed {
                self.show_error(root);
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
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

impl DnsConfigs {
    fn render(&mut self, sender: &ComponentSender<Self>) {
        self.list.remove_all();

        let mut group = None;

        self.records.iter().for_each(|dns| {
            let row = adw::ActionRow::builder()
                .title(&dns.name)
                .use_markup(false)
                .activatable(!dns.selected)
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

            radio.set_active(dns.selected);
            row.add_prefix(&radio);

            let input = sender.input_sender().clone();
            let id = dns.id.clone();

            row.connect_activated(move |_| input.emit(DnsMsg::Select(id.clone())));

            let edit = gtk::Button::builder()
                .icon_name("document-edit-symbolic")
                .tooltip_text(t!("dns.edit").as_ref())
                .valign(gtk::Align::Center)
                .build();

            edit.add_css_class("flat");

            let id = dns.id.clone();
            let input = sender.input_sender().clone();

            edit.connect_clicked(move |_| input.emit(DnsMsg::Edit(id.clone())));
            row.add_suffix(&edit);

            let popover = gtk::Popover::new();
            let actions = gtk::Box::new(gtk::Orientation::Vertical, 0);

            let details = gtk::Button::with_label(t!("config.details").as_ref());
            details.add_css_class("flat");

            let input = sender.input_sender().clone();
            let id = dns.id.clone();

            details.connect_clicked(gtk::glib::clone!(
                #[weak]
                popover,
                move |_| {
                    popover.popdown();
                    input.emit(DnsMsg::Details(id.clone()));
                }
            ));

            actions.append(&details);

            [
                ("config.rename", DnsMsg::ShowRename(dns.id.clone())),
                ("config.duplicate", DnsMsg::Duplicate(dns.id.clone())),
                ("config.delete", DnsMsg::ShowDelete(dns.id.clone())),
            ]
            .into_iter()
            .for_each(|(key, message)| {
                let button = gtk::Button::with_label(t!(key).as_ref());
                button.add_css_class("flat");

                if key == "config.delete" {
                    button.set_sensitive(!dns.selected);
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
                .tooltip_text(t!("dns.more").as_ref())
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
                .show_error(t!("dns.request_error").as_ref(), error, parent);
        }
    }
}
