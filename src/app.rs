use adw::prelude::*;
use gtk::{gio, glib};
use relm4::{adw, gtk, prelude::*};
use rust_i18n::t;

use crate::{error::ServerSettingsError, settings::ServerSettings};

pub struct App {
    settings: ServerSettings,
    active: Option<usize>,
    dialog: ServerDialog,
    error: Option<ServerSettingsError>,
    language_action: gio::SimpleAction,
    language_menu: gio::Menu,
}

#[derive(Debug)]
pub enum AppMsg {
    Opened,
    ShowAdd,
    EditServer(usize),
    SaveServer,
    OpenServer(usize),
    SetDefault(usize),
    SetLanguage(String),
}

#[relm4::component(pub)]
impl Component for App {
    type Init = ServerSettings;
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some("Waddle"),
            set_default_size: (760, 600),
            connect_map => AppMsg::Opened,

                #[name = "navigation"]
                adw::NavigationView {
                    set_vexpand: true,
                    add = &adw::NavigationPage {
                        set_tag: Some("home"),
                        set_title: "Waddle",
                        adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                pack_end = &gtk::MenuButton {
                                    set_icon_name: "preferences-desktop-locale-symbolic",
                                    #[watch]
                                    set_tooltip_text: Some(t!("language.title").as_ref()),
                                    #[watch]
                                    update_property: &[gtk::accessible::Property::Label(t!("language.title").as_ref())],
                                    set_menu_model: Some(&model.language_menu),
                                },
                                pack_end = &gtk::Button {
                                    set_icon_name: "network-server-symbolic",
                                    #[watch]
                                    set_tooltip_text: Some(t!("app.servers").as_ref()),
                                    #[watch]
                                    update_property: &[gtk::accessible::Property::Label(t!("app.servers").as_ref())],
                                    set_action_name: Some("navigation.push"),
                                    set_action_target_value: Some(&"servers".to_variant()),
                                },
                            },
                            add_top_bar = &adw::Banner {
                                #[watch]
                                set_revealed: model.error.is_some(),
                                #[watch]
                                set_title: &model.error.as_ref().map(ServerSettingsError::user_message).unwrap_or_default(),
                            },
                            adw::StatusPage {
                                set_icon_name: Some("network-server-symbolic"),
                                #[watch]
                                set_title: &model.active.map_or_else(
                                    || t!("app.first_server").to_string(),
                                    |index| {
                                        let server = &model.settings.servers()[index];
                                        if server.ip.is_ok() {
                                            t!("server.connected", name = &server.name).to_string()
                                        } else {
                                            t!("server.invalid_address_title", name = &server.name).to_string()
                                        }
                                    },
                                ),
                                #[watch]
                                set_description: Some(&model.active.map_or_else(
                                    || t!("app.start_hint").to_string(),
                                    |index| {
                                        let server = &model.settings.servers()[index];
                                        match &server.ip {
                                            Ok(ip) => format!("{}\n{}", std::net::SocketAddr::new(*ip, server.port), t!("app.placeholder")),
                                            Err(_) => t!("server.invalid_ip").to_string(),
                                        }
                                    },
                                )),
                                set_vexpand: true,
                                gtk::Button {
                                    #[watch]
                                    set_label: t!("app.add_server").as_ref(),
                                    set_halign: gtk::Align::Center,
                                    add_css_class: "suggested-action",
                                    add_css_class: "pill",
                                    #[watch]
                                    set_visible: model.settings.servers().is_empty(),
                                    connect_clicked => AppMsg::ShowAdd,
                                },
                            },
                        },
                    },
                    add = &adw::NavigationPage {
                        set_tag: Some("servers"),
                        #[watch]
                        set_title: t!("app.servers").as_ref(),
                        adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                pack_start = &gtk::Button {
                                    set_icon_name: "list-add-symbolic",
                                    #[watch]
                                    set_tooltip_text: Some(t!("app.add_server").as_ref()),
                                    #[watch]
                                    update_property: &[gtk::accessible::Property::Label(t!("app.add_server").as_ref())],
                                    connect_clicked => AppMsg::ShowAdd,
                                },
                            },
                            add_top_bar = &adw::Banner {
                                #[watch]
                                set_revealed: model.error.is_some(),
                                #[watch]
                                set_title: &model.error.as_ref().map(ServerSettingsError::user_message).unwrap_or_default(),
                            },
                            gtk::Stack {
                                add_named[Some("empty")] = &adw::StatusPage {
                                    set_icon_name: Some("network-server-symbolic"),
                                    #[watch]
                                    set_title: t!("app.first_server").as_ref(),
                                    #[watch]
                                    set_description: Some(t!("app.start_hint").as_ref()),
                                    gtk::Button {
                                        #[watch]
                                        set_label: t!("app.add_server").as_ref(),
                                        set_halign: gtk::Align::Center,
                                        add_css_class: "suggested-action",
                                        add_css_class: "pill",
                                        connect_clicked => AppMsg::ShowAdd,
                                    },
                                },
                                add_named[Some("list")] = &gtk::ScrolledWindow {
                                    set_hscrollbar_policy: gtk::PolicyType::Never,
                                    adw::Clamp {
                                        set_maximum_size: 640,
                                        set_margin_all: 24,
                                        adw::PreferencesGroup {
                                            set_valign: gtk::Align::Start,
                                            #[watch]
                                            set_description: Some(t!("app.default_server_hint").as_ref()),
                                            #[name = "servers"]
                                            gtk::ListBox {
                                                add_css_class: "boxed-list",
                                                set_selection_mode: gtk::SelectionMode::None,
                                            },
                                        },
                                    },
                                },
                                #[watch]
                                set_visible_child_name: if model.settings.servers().is_empty() { "empty" } else { "list" },
                            },
                        },
                    },
                },
        }
    }

    fn init(
        settings: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        crate::i18n::apply_language(&settings.language());
        let language_action = gio::SimpleAction::new_stateful(
            "language",
            Some(&String::static_variant_type()),
            &settings.language().to_variant(),
        );
        let input = sender.input_sender().clone();
        language_action.connect_change_state(move |_, value| {
            input.emit(AppMsg::SetLanguage(value.unwrap().str().unwrap().into()));
        });
        let actions = gio::SimpleActionGroup::new();
        actions.add_action(&language_action);
        root.insert_action_group("preferences", Some(&actions));
        let model = Self {
            active: settings.default_server_index(),
            settings,
            dialog: ServerDialog::new(&sender),
            error: None,
            language_action,
            language_menu: gio::Menu::new(),
        };
        model.render_language_menu();
        let widgets = view_output!();
        model.render_servers(&widgets.servers, &sender);
        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppMsg::Opened => {
                if self.settings.servers().is_empty() {
                    self.dialog.present(root, &self.settings, None);
                }
            }
            AppMsg::ShowAdd => self.dialog.present(root, &self.settings, None),
            AppMsg::EditServer(index) => self.dialog.present(root, &self.settings, Some(index)),
            AppMsg::SaveServer => {
                match self.settings.save_server(
                    self.dialog.editing,
                    &self.dialog.name.text(),
                    &self.dialog.ip.text(),
                    &self.dialog.port.text(),
                    self.dialog.make_default.is_active(),
                ) {
                    Ok(()) => {
                        if self.dialog.editing.is_none() {
                            self.active = Some(self.settings.servers().len() - 1);
                            widgets.navigation.pop_to_tag("home");
                        }
                        self.dialog.validation_error = None;
                        self.dialog.dialog.close();
                        self.error = None;
                    }
                    Err(error) => {
                        tracing::warn!(error = %error, "Could not save server");
                        self.dialog.validation_error = Some(error);
                        self.dialog.retranslate();
                    }
                }
            }
            AppMsg::OpenServer(index) => {
                self.active = Some(index);
                widgets.navigation.pop_to_tag("home");
            }
            AppMsg::SetDefault(index) => {
                self.error = self.settings.set_default(index).err();
            }
            AppMsg::SetLanguage(language) => match self.settings.set_language(&language) {
                Ok(()) => {
                    crate::i18n::apply_language(&language);
                    self.language_action.set_state(&language.to_variant());
                    self.render_language_menu();
                    self.dialog.retranslate();
                }
                Err(error) => self.error = Some(error),
            },
        }
        self.update_view(widgets, sender);
    }

    fn post_view() {
        self.render_servers(&widgets.servers, &sender);
    }
}

impl App {
    fn render_language_menu(&self) {
        self.language_menu.remove_all();
        for (language, label) in [
            ("", t!("language.system").to_string()),
            ("en", t!("language.english").to_string()),
            ("zh_CN", t!("language.chinese").to_string()),
        ] {
            let item = gio::MenuItem::new(Some(&label), None);
            item.set_action_and_target_value(
                Some("preferences.language"),
                Some(&language.to_variant()),
            );
            self.language_menu.append_item(&item);
        }
    }

    fn render_servers(&self, list: &gtk::ListBox, sender: &ComponentSender<Self>) {
        list.remove_all();
        for (index, server) in self.settings.servers().iter().enumerate() {
            let row = adw::ActionRow::builder()
                .title(&server.name)
                .use_markup(false)
                .title_lines(1)
                .subtitle_lines(1)
                .activatable(server.ip.is_ok())
                .build();
            match &server.ip {
                Ok(ip) => {
                    row.set_subtitle(&std::net::SocketAddr::new(*ip, server.port).to_string())
                }
                Err(_) => {
                    row.set_subtitle(t!("server.invalid_ip").as_ref());
                    let icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
                    icon.add_css_class("error");
                    row.add_prefix(&icon);
                }
            }
            let default = gtk::Button::builder()
                .icon_name(if server.is_default {
                    "starred-symbolic"
                } else {
                    "non-starred-symbolic"
                })
                .tooltip_text(if server.is_default {
                    t!("server.default").to_string()
                } else {
                    t!("server.make_default").to_string()
                })
                .sensitive(!server.is_default && server.ip.is_ok())
                .valign(gtk::Align::Center)
                .build();
            default.add_css_class("flat");
            default.update_property(&[gtk::accessible::Property::Label(
                if server.is_default {
                    t!("server.default")
                } else {
                    t!("server.make_default")
                }
                .as_ref(),
            )]);
            let input = sender.input_sender().clone();
            default.connect_clicked(move |_| {
                input.emit(AppMsg::SetDefault(index));
            });
            row.add_suffix(&default);

            let edit = gtk::Button::builder()
                .icon_name("document-edit-symbolic")
                .tooltip_text(t!("server.edit").as_ref())
                .valign(gtk::Align::Center)
                .build();
            edit.add_css_class("flat");
            edit.update_property(&[gtk::accessible::Property::Label(t!("server.edit").as_ref())]);
            let input = sender.input_sender().clone();
            edit.connect_clicked(move |_| input.emit(AppMsg::EditServer(index)));
            row.add_suffix(&edit);

            let current = self.active == Some(index);
            let indicator = gtk::Image::from_icon_name(if current {
                "object-select-symbolic"
            } else {
                "go-next-symbolic"
            });
            if current {
                indicator.set_tooltip_text(Some(t!("server.current").as_ref()));
                indicator.update_property(&[gtk::accessible::Property::Label(
                    t!("server.current").as_ref(),
                )]);
            } else {
                indicator.add_css_class("dim-label");
            }
            indicator.set_visible(server.ip.is_ok());
            row.add_suffix(&indicator);
            if server.ip.is_ok() {
                let input = sender.input_sender().clone();
                row.connect_activated(move |_| {
                    input.emit(AppMsg::OpenServer(index));
                });
            }
            list.append(&row);
        }
    }
}

struct ServerDialog {
    editing: Option<usize>,
    dialog: adw::Dialog,
    name: adw::EntryRow,
    ip: adw::EntryRow,
    port: adw::EntryRow,
    make_default: gtk::CheckButton,
    error: gtk::Label,
    validation_error: Option<ServerSettingsError>,
    cancel: gtk::Button,
    save: gtk::Button,
}

impl ServerDialog {
    fn new(sender: &ComponentSender<App>) -> Self {
        let dialog = adw::Dialog::builder().content_width(440).build();
        let cancel = gtk::Button::new();
        cancel.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        let save = gtk::Button::new();
        save.add_css_class("suggested-action");
        let input = sender.input_sender().clone();
        save.connect_clicked(move |_| input.emit(AppMsg::SaveServer));

        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();
        header.pack_start(&cancel);
        header.pack_end(&save);

        let name = adw::EntryRow::builder().activates_default(true).build();
        let ip = adw::EntryRow::builder().activates_default(true).build();
        let port = adw::EntryRow::builder()
            .activates_default(true)
            .input_purpose(gtk::InputPurpose::Digits)
            .build();
        let group = adw::PreferencesGroup::new();
        group.add(&name);
        group.add(&ip);
        group.add(&port);
        let make_default = gtk::CheckButton::new();
        let error = gtk::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .visible(false)
            .build();
        error.add_css_class("error");

        let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
        body.set_margin_all(24);
        body.append(&group);
        body.append(&make_default);
        body.append(&error);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .child(&body)
            .build();
        toolbar.set_content(Some(&scroll));
        dialog.set_child(Some(&toolbar));
        dialog.set_default_widget(Some(&save));
        dialog.set_focus(Some(&name));

        let form = Self {
            editing: None,
            dialog,
            name,
            ip,
            port,
            make_default,
            error,
            validation_error: None,
            cancel,
            save,
        };
        form.retranslate();
        form
    }

    fn retranslate(&self) {
        self.dialog.set_title(
            if self.editing.is_some() {
                t!("server.edit")
            } else {
                t!("dialog.title")
            }
            .as_ref(),
        );
        self.cancel.set_label(t!("dialog.cancel").as_ref());
        self.save.set_label(
            if self.editing.is_some() {
                t!("dialog.save")
            } else {
                t!("dialog.connect")
            }
            .as_ref(),
        );
        self.name.set_title(t!("dialog.server_name").as_ref());
        self.ip.set_title(t!("dialog.ip").as_ref());
        self.port.set_title(t!("dialog.port").as_ref());
        self.make_default
            .set_label(Some(t!("dialog.make_default").as_ref()));
        self.error.set_label(
            &self
                .validation_error
                .as_ref()
                .map(ServerSettingsError::user_message)
                .unwrap_or_default(),
        );
        self.error.set_visible(self.validation_error.is_some());
    }

    fn present(
        &mut self,
        parent: &adw::ApplicationWindow,
        settings: &ServerSettings,
        editing: Option<usize>,
    ) {
        self.editing = editing;
        match editing {
            Some(index) => {
                let server = &settings.servers()[index];
                self.name.set_text(&server.name);
                self.ip.set_text(&match &server.ip {
                    Ok(ip) => ip.to_string(),
                    Err(error) => error.value.clone(),
                });
                self.port.set_text(&server.port.to_string());
                self.make_default.set_active(server.is_default);
                self.make_default.set_sensitive(!server.is_default);
            }
            None => {
                self.name.set_text("");
                self.ip.set_text("");
                self.port.set_text("2023");
                self.make_default.set_active(settings.servers().is_empty());
                self.make_default
                    .set_sensitive(!settings.servers().is_empty());
            }
        }
        self.validation_error = None;
        self.retranslate();
        self.dialog.set_focus(Some(&self.name));
        self.dialog.present(Some(parent));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{self, ServerSettings};
    use std::time::{Duration, Instant};

    fn find_widget<W: IsA<gtk::Widget>>(
        root: &impl IsA<gtk::Widget>,
        matches: &impl Fn(&W) -> bool,
    ) -> Option<W> {
        let root = root.as_ref();
        if let Ok(widget) = root.clone().downcast::<W>()
            && matches(&widget)
        {
            return Some(widget);
        }
        let mut child = root.first_child();
        while let Some(widget) = child {
            if let Some(found) = find_widget(&widget, matches) {
                return Some(found);
            }
            child = widget.next_sibling();
        }
        None
    }

    fn click(root: &impl IsA<gtk::Widget>, label: &str) {
        let matches =
            |button: &gtk::Button| button.label().as_deref() == Some(label) && button.is_mapped();
        wait_for(|| find_widget::<gtk::Button>(root, &matches).is_some());
        find_widget::<gtk::Button>(root, &matches)
            .unwrap()
            .emit_clicked();
    }

    fn fill(dialog: &adw::Dialog, title: &str, value: &str) {
        find_widget::<adw::EntryRow>(dialog, &|row| row.title() == title)
            .unwrap()
            .set_text(value);
    }

    fn wait_for(condition: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !condition() {
            assert!(
                Instant::now() < deadline,
                "UI did not reach the expected state"
            );
            let context = gtk::glib::MainContext::default();
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    #[ignore = "requires a GTK display; run with --ignored --test-threads=1"]
    fn first_launch_add_servers_and_reopen_the_default_server() {
        let _app = RelmApp::<AppMsg>::new("io.github.krteke.Waddle.Tests");
        gtk::Settings::default()
            .unwrap()
            .set_gtk_enable_animations(false);
        let settings = settings::tests::settings();
        settings.set_string("language", "en").unwrap();
        let controller = App::builder()
            .launch(ServerSettings::load(settings.clone()))
            .detach();
        let window = controller.widget();
        window.present();
        wait_for(|| window.is_mapped());
        wait_for(|| window.visible_dialog().is_some());
        let dialog = window.visible_dialog().unwrap();
        assert_eq!(dialog.title(), "Connect to Server");
        wait_for(|| dialog.is_mapped() && dialog.width() > 0);

        click(&dialog, "Cancel");
        wait_for(|| window.visible_dialog().is_none());
        let servers_button = find_widget::<gtk::Button>(window, &|button| {
            button.icon_name().as_deref() == Some("network-server-symbolic")
        })
        .expect("The header must provide a server management entry");
        servers_button.emit_clicked();
        let navigation = find_widget::<adw::NavigationView>(window, &|_| true).unwrap();
        wait_for(|| navigation.visible_page_tag().as_deref() == Some("servers"));
        assert!(
            find_widget::<adw::StatusPage>(window, &|page| {
                page.title() == "Add your first server" && page.is_mapped()
            })
            .is_some()
        );
        gtk::prelude::WidgetExt::activate_action(
            &navigation.visible_page().unwrap(),
            "navigation.pop",
            None,
        )
        .unwrap();
        wait_for(|| navigation.visible_page_tag().as_deref() == Some("home"));
        assert!(find_widget::<gtk::ListBox>(window, &|list| list.is_mapped()).is_none());
        click(window, "Add Server");
        wait_for(|| window.visible_dialog().is_some());

        fill(&dialog, "Server Name", "Home");
        fill(&dialog, "IP Address", "192.168.1.2");
        fill(&dialog, "Port", "2023");
        click(&dialog, "Connect");
        wait_for(|| window.visible_dialog().is_none());
        let status = find_widget::<adw::StatusPage>(window, &|page| page.is_mapped()).unwrap();
        assert_eq!(status.title(), "Connected to Home");
        let menu = find_widget::<gtk::MenuButton>(window, &|button| {
            button.icon_name().as_deref() == Some("preferences-desktop-locale-symbolic")
        })
        .unwrap();
        let languages = menu.menu_model().unwrap();
        assert_eq!(languages.n_items(), 3);
        controller.state().get_mut().model.error = Some(ServerSettingsError::Save(
            glib::bool_error!("private backend details"),
        ));
        gtk::prelude::WidgetExt::activate_action(
            window,
            "preferences.language",
            Some(&"zh_CN".to_variant()),
        )
        .unwrap();
        wait_for(|| settings.string("language") == "zh_CN");
        assert_eq!(
            controller.model().language_action.state().unwrap().str(),
            Some("zh_CN")
        );
        assert_eq!(status.title(), "已连接 Home");
        assert_eq!(menu.tooltip_text().as_deref(), Some("语言"));
        assert_eq!(servers_button.tooltip_text().as_deref(), Some("服务器"));
        assert_eq!(navigation.find_page("servers").unwrap().title(), "服务器");
        assert!(menu.label().is_none());
        assert_eq!(
            languages
                .item_attribute_value(0, "label", None)
                .unwrap()
                .str(),
            Some("跟随系统")
        );
        assert!(
            find_widget::<gtk::Label>(window, &|label| label.text() == "无法保存设置。"
                && label.is_visible())
            .is_some()
        );
        gtk::prelude::WidgetExt::activate_action(
            window,
            "preferences.language",
            Some(&"".to_variant()),
        )
        .unwrap();
        wait_for(|| settings.string("language").is_empty());
        gtk::prelude::WidgetExt::activate_action(
            window,
            "preferences.language",
            Some(&"en".to_variant()),
        )
        .unwrap();
        wait_for(|| status.title() == "Connected to Home");
        assert_eq!(
            ServerSettings::load(settings.clone()).default_server_index(),
            Some(0)
        );

        servers_button.emit_clicked();
        wait_for(|| navigation.visible_page_tag().as_deref() == Some("servers"));
        find_widget::<gtk::Button>(window, &|button| {
            button.icon_name().as_deref() == Some("list-add-symbolic")
        })
        .unwrap()
        .emit_clicked();
        wait_for(|| window.visible_dialog().is_some());
        let dialog = window.visible_dialog().unwrap();
        fill(&dialog, "Server Name", "Home");
        fill(&dialog, "IP Address", "10.0.0.2");
        fill(&dialog, "Port", "8080");
        click(&dialog, "Connect");
        wait_for(|| {
            find_widget::<gtk::Label>(&dialog, &|label| label.text().contains("already exists"))
                .is_some()
        });
        assert_eq!(ServerSettings::load(settings.clone()).servers().len(), 1);

        gtk::prelude::WidgetExt::activate_action(
            window,
            "preferences.language",
            Some(&"zh_CN".to_variant()),
        )
        .unwrap();
        wait_for(|| dialog.title() == "连接服务器");
        assert_eq!(
            find_widget::<adw::EntryRow>(&dialog, &|row| row.title() == "服务器名称")
                .unwrap()
                .text(),
            "Home"
        );
        assert_eq!(
            find_widget::<adw::EntryRow>(&dialog, &|row| row.title() == "IP 地址")
                .unwrap()
                .text(),
            "10.0.0.2"
        );
        assert_eq!(
            find_widget::<adw::EntryRow>(&dialog, &|row| row.title() == "端口")
                .unwrap()
                .text(),
            "8080"
        );
        assert!(
            find_widget::<gtk::Button>(&dialog, &|button| button.label().as_deref()
                == Some("连接"))
            .is_some()
        );
        assert!(
            find_widget::<gtk::Label>(&dialog, &|label| label.text() == "服务器名称已存在。"
                && label.is_visible())
            .is_some()
        );
        gtk::prelude::WidgetExt::activate_action(
            window,
            "preferences.language",
            Some(&"en".to_variant()),
        )
        .unwrap();
        wait_for(|| dialog.title() == "Connect to Server");
        assert!(
            find_widget::<gtk::Label>(&dialog, &|label| label.text()
                == "A server with this name already exists."
                && label.is_visible())
            .is_some()
        );

        fill(&dialog, "Server Name", "Office");
        click(&dialog, "Connect");
        wait_for(|| window.visible_dialog().is_none());
        assert_eq!(status.title(), "Connected to Office");
        assert_eq!(navigation.visible_page_tag().as_deref(), Some("home"));
        servers_button.emit_clicked();
        wait_for(|| navigation.visible_page_tag().as_deref() == Some("servers"));
        let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Office").unwrap();
        find_widget::<gtk::Button>(&row, &|button| {
            button.icon_name().as_deref() == Some("non-starred-symbolic")
        })
        .unwrap()
        .emit_clicked();
        wait_for(|| ServerSettings::load(settings.clone()).default_server_index() == Some(1));

        let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Home").unwrap();
        adw::prelude::ActionRowExt::activate(&row);
        wait_for(|| status.title() == "Connected to Home");
        assert_eq!(navigation.visible_page_tag().as_deref(), Some("home"));
        window.close();
        drop(controller);

        settings.set_string("language", "zh_CN").unwrap();
        let reopened = App::builder()
            .launch(ServerSettings::load(settings.clone()))
            .detach();
        reopened.widget().present();
        wait_for(|| reopened.widget().is_mapped());
        let status =
            find_widget::<adw::StatusPage>(reopened.widget(), &|page| page.is_mapped()).unwrap();
        assert_eq!(status.title(), "已连接 Office");
        assert!(reopened.widget().visible_dialog().is_none());
        gtk::prelude::WidgetExt::activate_action(
            reopened.widget(),
            "preferences.language",
            Some(&"en".to_variant()),
        )
        .unwrap();
        wait_for(|| status.title() == "Connected to Office");
        reopened.widget().close();
        drop(reopened);

        settings
            .set(
                "servers",
                vec![
                    ("Broken", "not-an-ip", 2023u16, true),
                    ("Home", "127.0.0.1", 8080u16, false),
                ],
            )
            .unwrap();
        let reopened = App::builder()
            .launch(ServerSettings::load(settings.clone()))
            .detach();
        reopened.widget().present();
        wait_for(|| reopened.widget().is_mapped());
        assert!(reopened.widget().visible_dialog().is_none());
        let status =
            find_widget::<adw::StatusPage>(reopened.widget(), &|page| page.is_mapped()).unwrap();
        let servers_button = find_widget::<gtk::Button>(reopened.widget(), &|button| {
            button.icon_name().as_deref() == Some("network-server-symbolic")
        })
        .unwrap();
        servers_button.emit_clicked();
        let navigation = find_widget::<adw::NavigationView>(reopened.widget(), &|_| true).unwrap();
        wait_for(|| navigation.visible_page_tag().as_deref() == Some("servers"));
        let broken =
            find_widget::<adw::ActionRow>(reopened.widget(), &|row| row.title() == "Broken")
                .unwrap();
        assert_eq!(broken.subtitle().as_deref(), Some("Invalid IP address"));
        assert!(!broken.is_activatable());
        assert!(!status.title().starts_with("Connected to"));
        assert_eq!(status.description().as_deref(), Some("Invalid IP address"));

        let home =
            find_widget::<adw::ActionRow>(reopened.widget(), &|row| row.title() == "Home").unwrap();
        adw::prelude::ActionRowExt::activate(&home);
        wait_for(|| status.title() == "Connected to Home");
        servers_button.emit_clicked();
        wait_for(|| navigation.visible_page_tag().as_deref() == Some("servers"));
        let home =
            find_widget::<adw::ActionRow>(reopened.widget(), &|row| row.title() == "Home").unwrap();
        find_widget::<gtk::Button>(&home, &|button| {
            button.icon_name().as_deref() == Some("non-starred-symbolic")
        })
        .unwrap()
        .emit_clicked();
        wait_for(|| !settings.get::<Vec<(String, String, u16, bool)>>("servers")[0].3);
        let records = settings.get::<Vec<(String, String, u16, bool)>>("servers");
        assert_eq!(records[0].1, "not-an-ip");
        assert!(records[1].3);

        let broken =
            find_widget::<adw::ActionRow>(reopened.widget(), &|row| row.title() == "Broken")
                .unwrap();
        adw::prelude::ActionRowExt::activate(&broken);
        while glib::MainContext::default().pending() {
            glib::MainContext::default().iteration(false);
        }
        assert_eq!(navigation.visible_page_tag().as_deref(), Some("servers"));
        assert_eq!(status.title(), "Connected to Home");
        assert!(
            !find_widget::<gtk::Button>(&broken, &|button| {
                button.icon_name().as_deref() == Some("non-starred-symbolic")
            })
            .unwrap()
            .is_sensitive()
        );

        let edit = find_widget::<gtk::Button>(&broken, &|button| {
            button.icon_name().as_deref() == Some("document-edit-symbolic")
        })
        .expect("Invalid servers must remain editable");
        assert!(edit.is_sensitive());
        edit.emit_clicked();
        wait_for(|| reopened.widget().visible_dialog().is_some());
        let dialog = reopened.widget().visible_dialog().unwrap();
        assert_eq!(dialog.title(), "Edit Server");
        assert_eq!(
            find_widget::<adw::EntryRow>(&dialog, &|row| row.title() == "IP Address")
                .unwrap()
                .text(),
            "not-an-ip"
        );
        fill(&dialog, "IP Address", "192.168.1.10");
        click(&dialog, "Cancel");
        wait_for(|| reopened.widget().visible_dialog().is_none());
        assert_eq!(
            settings.get::<Vec<(String, String, u16, bool)>>("servers"),
            records
        );

        let broken =
            find_widget::<adw::ActionRow>(reopened.widget(), &|row| row.title() == "Broken")
                .unwrap();
        find_widget::<gtk::Button>(&broken, &|button| {
            button.icon_name().as_deref() == Some("document-edit-symbolic")
        })
        .unwrap()
        .emit_clicked();
        wait_for(|| reopened.widget().visible_dialog().is_some());
        assert_eq!(
            find_widget::<adw::EntryRow>(&dialog, &|row| row.title() == "IP Address")
                .unwrap()
                .text(),
            "not-an-ip"
        );
        fill(&dialog, "Server Name", "Repaired");
        fill(&dialog, "IP Address", "192.168.1.10");
        fill(&dialog, "Port", "9090");
        gtk::prelude::WidgetExt::activate_action(
            reopened.widget(),
            "preferences.language",
            Some(&"zh_CN".to_variant()),
        )
        .unwrap();
        wait_for(|| dialog.title() == "编辑服务器");
        assert_eq!(
            find_widget::<adw::EntryRow>(&dialog, &|row| row.title() == "服务器名称")
                .unwrap()
                .text(),
            "Repaired"
        );
        click(&dialog, "保存");
        wait_for(|| reopened.widget().visible_dialog().is_none());
        gtk::prelude::WidgetExt::activate_action(
            reopened.widget(),
            "preferences.language",
            Some(&"en".to_variant()),
        )
        .unwrap();
        wait_for(|| status.title() == "Connected to Home");
        assert_eq!(navigation.visible_page_tag().as_deref(), Some("servers"));
        assert_eq!(status.title(), "Connected to Home");
        assert_eq!(
            settings.get::<Vec<(String, String, u16, bool)>>("servers"),
            vec![
                ("Repaired".into(), "192.168.1.10".into(), 9090, false),
                ("Home".into(), "127.0.0.1".into(), 8080, true),
            ]
        );
        let repaired =
            find_widget::<adw::ActionRow>(reopened.widget(), &|row| row.title() == "Repaired")
                .unwrap();
        assert!(repaired.is_activatable());
        adw::prelude::ActionRowExt::activate(&repaired);
        wait_for(|| status.title() == "Connected to Repaired");
        servers_button.emit_clicked();
        wait_for(|| navigation.visible_page_tag().as_deref() == Some("servers"));
        let repaired =
            find_widget::<adw::ActionRow>(reopened.widget(), &|row| row.title() == "Repaired")
                .unwrap();
        find_widget::<gtk::Button>(&repaired, &|button| {
            button.icon_name().as_deref() == Some("document-edit-symbolic")
        })
        .unwrap()
        .emit_clicked();
        wait_for(|| reopened.widget().visible_dialog().is_some());
        fill(&dialog, "Server Name", "Renamed");
        click(&dialog, "Save");
        wait_for(|| reopened.widget().visible_dialog().is_none());
        assert_eq!(status.title(), "Connected to Renamed");
        assert_eq!(navigation.visible_page_tag().as_deref(), Some("servers"));
        reopened.widget().close();
    }
}
