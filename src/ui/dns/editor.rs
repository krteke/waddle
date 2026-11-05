use super::{
    fields::{Direction, Fields, RuleAction},
    form::{Form, Upstream},
};
use crate::{
    api::{
        Api,
        dns::{Dns, DnsDocument},
    },
    error::ApiError,
    ui::error_dialog::DialogExt,
};
use adw::prelude::*;
use relm4::{Sender, adw, gtk, prelude::*};
use rust_i18n::t;

pub struct Editor {
    api: Api,
    id: Option<String>,
    original_name: String,
    original_text: String,
    original_form: Option<Form>,
    form_source: Form,
    form_text: String,
    fields: Fields,
    buffer: gtk::TextBuffer,
    advanced: bool,
    dirty: bool,
    busy: bool,
    submitted: bool,
    unsupported: bool,
    error_dialog: adw::AlertDialog,
    discard: adw::AlertDialog,
}

#[derive(Debug)]
pub enum EditorMsg {
    Changed,
    Mode(u32),
    AddUpstream,
    RemoveUpstream(adw::ExpanderRow),
    AddRule(Direction),
    RuleAction(Direction, adw::ExpanderRow, RuleAction),
    Save,
    Reset,
    Back,
    Discard,
    Retranslate,
}
#[derive(Clone, Debug)]
pub enum EditorOutput {
    Saved,
    Closed,
    Unauthorized,
}
#[derive(Debug)]
pub enum EditorReply {
    Saved(Result<(), ApiError>),
    Parsed(Result<DnsDocument, ApiError>),
}

#[relm4::component(pub)]
impl Component for Editor {
    type Init = (Api, Option<Dns>);
    type Input = EditorMsg;
    type Output = EditorOutput;
    type CommandOutput = EditorReply;

    view! {
        adw::NavigationPage {
            set_tag: Some("dns-editor"),
            #[watch]
            set_title: if model.id.is_none() { t!("dns.new_title") } else { t!("dns.edit") }.as_ref(),
            #[watch]
            set_can_pop: !model.busy && !model.dirty,
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_back_button: false,
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_icon_name: "go-previous-symbolic",
                        #[watch]
                        set_tooltip_text: Some(t!("config.back").as_ref()),
                        #[watch]
                        set_sensitive: !model.busy,
                        connect_clicked => EditorMsg::Back,
                    },
                    pack_end = &gtk::Button {
                        #[watch]
                        set_label: &t!("dialog.save"),
                        add_css_class: "suggested-action",
                        #[watch]
                        set_sensitive: !model.busy && (model.id.is_none() || model.dirty),
                        connect_clicked => EditorMsg::Save,
                    },
                },
                add_top_bar = &adw::Banner {
                    #[watch]
                    set_title: &t!("dns.advanced_needed"),
                    #[watch]
                    set_revealed: model.unsupported,
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    #[watch]
                    set_sensitive: !model.busy,
                    #[local_ref]
                    identity -> adw::PreferencesGroup {
                        set_margin_top: 12, set_margin_start: 24, set_margin_end: 24,
                    },
                    #[name = "mode"]
                    adw::ToggleGroup {
                        set_margin_top: 18, set_margin_bottom: 6, set_margin_start: 24, set_margin_end: 24,
                        set_homogeneous: true,
                        #[wrap(Clone::clone)]
                        add = &adw::Toggle { #[watch] set_label: Some(t!("dns.simple").as_ref()) },
                        #[wrap(Clone::clone)]
                        add = &adw::Toggle { #[watch] set_label: Some(t!("routing.advanced").as_ref()) },
                        #[watch]
                        #[block_signal(mode_changed)]
                        set_active: u32::from(model.advanced),
                        connect_active_notify[sender] => move |g| sender.input(EditorMsg::Mode(g.active())) @mode_changed,
                    },
                    gtk::Stack {
                        set_vexpand: true,
                        set_vhomogeneous: false,
                        add_named[Some("form")] = &model.fields.page.clone(),
                        add_named[Some("advanced")] = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 6,
                            set_margin_all: 24,
                            #[local_ref]
                            code -> adw::Bin {},
                            gtk::Label {
                                add_css_class: "caption", add_css_class: "error",
                                set_xalign: 0.0,
                                #[watch]
                                set_label: &t!("config.required"),
                                #[watch]
                                set_visible: model.submitted && model.text().trim().is_empty(),
                            },
                        },
                        #[watch]
                        set_visible_child_name: if model.advanced { "advanced" } else { "form" },
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
        (api, original): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let original_name = original
            .as_ref()
            .map(|d| d.name.clone())
            .unwrap_or_default();
        let original_text = original
            .as_ref()
            .map(|d| d.dns.string.clone())
            .unwrap_or_else(|| Form::default().text());
        let form = original
            .as_ref()
            .map(|d| Form::from_document(&original_text, &d.dns))
            .unwrap_or_else(|| Some(Form::default()));
        let advanced = form.is_none();
        let original_form = form.clone();
        let form_source = form.unwrap_or_default();
        let mut fields = Fields::new(sender.input_sender());
        fields.name.row.set_text(&original_name);
        fields.load(&form_source, sender.input_sender());
        let buffer = gtk::TextBuffer::new(None);
        buffer.set_text(&original_text);
        let input = sender.input_sender().clone();
        buffer.connect_changed(move |_| input.emit(EditorMsg::Changed));
        let code = code_view(&buffer, true);
        let error_dialog = adw::AlertDialog::new(None, None);
        error_dialog.add_response("close", &t!("dialog.close"));
        error_dialog.set_close_response("close");
        let discard = adw::AlertDialog::new(None, None);
        discard.add_response("cancel", &t!("dialog.cancel"));
        discard.add_response("discard", &t!("config.discard"));
        discard.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        discard.set_default_response(Some("cancel"));
        discard.set_close_response("cancel");
        let input = sender.input_sender().clone();
        discard.connect_response(Some("discard"), move |_, _| input.emit(EditorMsg::Discard));
        let model = Self {
            api,
            id: original.map(|d| d.id),
            original_name,
            form_text: original_text.clone(),
            original_text,
            original_form,
            form_source,
            fields,
            buffer,
            advanced,
            dirty: false,
            busy: false,
            submitted: false,
            unsupported: advanced,
            error_dialog,
            discard,
        };
        let identity = &model.fields.identity;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: EditorMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            EditorMsg::Changed => {}
            EditorMsg::AddUpstream => {
                self.fields.add_upstream(
                    &Upstream {
                        name: String::new(),
                        address: String::new(),
                    },
                    sender.input_sender(),
                );
            }
            EditorMsg::RemoveUpstream(row) => self.fields.remove_upstream(&row),
            EditorMsg::AddRule(direction) => self.fields.add_rule(direction, sender.input_sender()),
            EditorMsg::RuleAction(direction, row, action) => {
                self.fields.rules(direction).act(&row, action)
            }
            EditorMsg::Mode(1) => {
                let text = self.text();
                self.buffer.set_text(&text);
                self.advanced = true;
                self.unsupported = false;
            }
            EditorMsg::Mode(_) => {
                self.busy = true;
                let raw = self.text();
                let api = self.api.clone();
                sender
                    .oneshot_command(async move { EditorReply::Parsed(api.parse_dns(raw).await) });
            }
            EditorMsg::Reset => {
                self.fields.name.row.set_text(&self.original_name);
                self.buffer.set_text(&self.original_text);
                self.submitted = false;
                if let Some(form) = &self.original_form {
                    self.form_source = form.clone();
                    self.form_text = self.original_text.clone();
                    self.fields.load(form, sender.input_sender());
                    self.advanced = false;
                } else {
                    self.advanced = true;
                }
                self.unsupported = false;
            }
            EditorMsg::Back if self.dirty => {
                self.discard.set_heading(Some(&t!("config.discard_title")));
                self.discard.set_body(&t!("dns.discard_hint"));
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
                self.submitted = true;
                let valid = if self.advanced {
                    self.fields.name.validate(true) && !self.text().trim().is_empty()
                } else {
                    self.fields.validate(true)
                };
                if !valid {
                    return;
                }
                self.busy = true;
                let api = self.api.clone();
                let id = self.id.clone();
                let name = self.fields.name.row.text().trim().to_string();
                let text = self.text();
                let rename = name != self.original_name;
                let changed = text != self.original_text;
                sender.oneshot_command(async move {
                    EditorReply::Saved(
                        async {
                            match id {
                                Some(id) => {
                                    if changed {
                                        api.update_dns(id.clone(), text).await?;
                                    }
                                    if rename {
                                        api.rename_dns(id, name).await?;
                                    }
                                    Ok(())
                                }
                                None => api.create_dns(name, text).await,
                            }
                        }
                        .await,
                    )
                });
            }
            EditorMsg::Retranslate => {}
        }
        self.refresh();
    }

    fn update_cmd(&mut self, reply: EditorReply, sender: ComponentSender<Self>, root: &Self::Root) {
        self.busy = false;
        let (heading, result) = match reply {
            EditorReply::Saved(Ok(())) => {
                sender.output(EditorOutput::Saved).unwrap();
                return;
            }
            EditorReply::Parsed(Ok(document)) => {
                let raw = self.text();
                if let Some(form) = Form::from_document(&raw, &document) {
                    self.fields.load(&form, sender.input_sender());
                    self.form_source = form;
                    self.form_text = raw;
                    self.advanced = false;
                    self.unsupported = false;
                } else {
                    self.unsupported = true;
                }
                self.refresh();
                return;
            }
            EditorReply::Saved(Err(error)) => (t!("dns.save_error"), error),
            EditorReply::Parsed(Err(error)) => (t!("dns.parse_error"), error),
        };
        if matches!(result, ApiError::Unauthorized) {
            sender.output(EditorOutput::Unauthorized).unwrap();
        } else {
            tracing::warn!(endpoint = %self.api.endpoint(), %result, "DNS editor request failed");
            self.error_dialog.show_error(&heading, &result, root);
        }
        self.refresh();
    }
    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        self.error_dialog.force_close();
        self.discard.force_close();
    }
}

impl Editor {
    fn text(&self) -> String {
        if self.advanced {
            self.buffer
                .text(&self.buffer.start_iter(), &self.buffer.end_iter(), false)
                .to_string()
        } else {
            let form = self.fields.read();
            if form == self.form_source {
                self.form_text.clone()
            } else {
                form.text()
            }
        }
    }
    fn refresh(&mut self) {
        self.fields.refresh();
        self.fields.name.validate(self.submitted);
        if !self.advanced {
            self.fields.validate(self.submitted);
        }
        self.dirty = self.fields.name.row.text().trim() != self.original_name
            || self.text() != self.original_text;
    }
}

pub fn code_view(buffer: &gtk::TextBuffer, editable: bool) -> adw::Bin {
    let text = gtk::TextView::builder()
        .buffer(buffer)
        .monospace(true)
        .editable(editable)
        .left_margin(12)
        .right_margin(12)
        .top_margin(12)
        .bottom_margin(12)
        .build();
    text.update_property(&[
        gtk::accessible::Property::Label(&t!("dns.configuration")),
        gtk::accessible::Property::Required(editable),
    ]);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .min_content_height(200)
        .child(&text)
        .build();
    adw::Bin::builder()
        .css_classes(["card"])
        .overflow(gtk::Overflow::Hidden)
        .child(&scroll)
        .build()
}
