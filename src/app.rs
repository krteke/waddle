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

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_title_widget: Some(&adw::WindowTitle::new("Waddle", "")),
                    pack_end = &gtk::Button {
                        #[watch]
                        set_label: t!("app.add_server").as_ref(),
                        #[watch]
                        set_tooltip_text: Some(t!("app.add_server_hint").as_ref()),
                        connect_clicked => AppMsg::ShowAdd,
                    },
                    pack_end = &gtk::MenuButton {
                        #[watch]
                        set_label: t!("language.title").as_ref(),
                        set_menu_model: Some(&model.language_menu),
                    },
                },

                gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    adw::Clamp {
                        set_maximum_size: 640,
                        set_margin_all: 24,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 24,

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
                            },

                            gtk::Label {
                                add_css_class: "error",
                                set_wrap: true,
                                #[watch]
                                set_visible: model.error.is_some(),
                                #[watch]
                                set_label: &model.error.as_ref().map(ServerSettingsError::user_message).unwrap_or_default(),
                            },

                            adw::PreferencesGroup {
                                #[watch]
                                set_title: t!("app.servers").as_ref(),
                                #[watch]
                                set_description: Some(t!("app.default_server_hint").as_ref()),
                                #[watch]
                                set_visible: !model.settings.servers().is_empty(),

                                #[name = "servers"]
                                gtk::ListBox {
                                    add_css_class: "boxed-list",
                                    set_selection_mode: gtk::SelectionMode::None,
                                },
                            },
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

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            AppMsg::Opened => {
                if self.settings.servers().is_empty() {
                    self.dialog.present(root, true);
                }
            }
            AppMsg::ShowAdd => self
                .dialog
                .present(root, self.settings.servers().is_empty()),
            AppMsg::SaveServer => {
                match self.settings.add_server(
                    &self.dialog.name.text(),
                    &self.dialog.ip.text(),
                    &self.dialog.port.text(),
                    self.dialog.make_default.is_active(),
                ) {
                    Ok(()) => {
                        self.active = Some(self.settings.servers().len() - 1);
                        self.dialog.validation_error = None;
                        self.dialog.dialog.close();
                        self.error = None;
                    }
                    Err(error) => {
                        tracing::warn!(error = %error, "Could not add server");
                        self.dialog.validation_error = Some(error);
                        self.dialog.retranslate();
                    }
                }
            }
            AppMsg::OpenServer(index) => self.active = Some(index),
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
                .label(if server.is_default {
                    t!("server.default").to_string()
                } else {
                    t!("server.make_default").to_string()
                })
                .tooltip_text(t!("server.open_default_hint").to_string())
                .sensitive(!server.is_default)
                .valign(gtk::Align::Center)
                .build();
            let input = sender.input_sender().clone();
            default.connect_clicked(move |_| {
                input.emit(AppMsg::SetDefault(index));
            });
            row.add_suffix(&default);

            let is_active = self.active == Some(index);
            let open = gtk::Button::builder()
                .label(if is_active {
                    t!("server.current").to_string()
                } else {
                    t!("server.open").to_string()
                })
                .sensitive(!is_active && server.ip.is_ok())
                .valign(gtk::Align::Center)
                .build();
            let input = sender.input_sender().clone();
            open.connect_clicked(move |_| {
                input.emit(AppMsg::OpenServer(index));
            });
            row.add_suffix(&open);
            list.append(&row);
        }
    }
}

struct ServerDialog {
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
        toolbar.set_content(Some(&body));
        dialog.set_child(Some(&toolbar));
        dialog.set_default_widget(Some(&save));
        dialog.set_focus(Some(&name));

        let form = Self {
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
        self.dialog.set_title(t!("dialog.title").as_ref());
        self.cancel.set_label(t!("dialog.cancel").as_ref());
        self.save.set_label(t!("dialog.connect").as_ref());
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

    fn present(&mut self, parent: &adw::ApplicationWindow, first_server: bool) {
        self.name.set_text("");
        self.ip.set_text("");
        self.port.set_text("2023");
        self.make_default.set_active(first_server);
        self.make_default.set_sensitive(!first_server);
        self.validation_error = None;
        self.error.set_visible(false);
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
        find_widget::<gtk::Button>(root, &|button| button.label().as_deref() == Some(label))
            .unwrap_or_else(|| panic!("Missing button: {label}"))
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

        fill(&dialog, "Server Name", "Home");
        fill(&dialog, "IP Address", "192.168.1.2");
        fill(&dialog, "Port", "2023");
        click(&dialog, "Connect");
        wait_for(|| window.visible_dialog().is_none());
        let status = find_widget::<adw::StatusPage>(window, &|_| true).unwrap();
        assert_eq!(status.title(), "Connected to Home");
        let menu = find_widget::<gtk::MenuButton>(window, &|button| {
            button.label().as_deref() == Some("Language")
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
        assert_eq!(menu.label().as_deref(), Some("语言"));
        assert!(
            find_widget::<gtk::Button>(window, &|button| button.label().as_deref()
                == Some("添加服务器"))
            .is_some()
        );
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

        click(window, "Add Server");
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
        let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Office").unwrap();
        click(&row, "Make Default");
        wait_for(|| ServerSettings::load(settings.clone()).default_server_index() == Some(1));

        let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Home").unwrap();
        click(&row, "Open");
        wait_for(|| status.title() == "Connected to Home");
        window.close();
        drop(controller);

        settings.set_string("language", "zh_CN").unwrap();
        let reopened = App::builder()
            .launch(ServerSettings::load(settings.clone()))
            .detach();
        reopened.widget().present();
        wait_for(|| reopened.widget().is_mapped());
        let status = find_widget::<adw::StatusPage>(reopened.widget(), &|_| true).unwrap();
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
        let broken =
            find_widget::<adw::ActionRow>(reopened.widget(), &|row| row.title() == "Broken")
                .unwrap();
        assert_eq!(broken.subtitle().as_deref(), Some("Invalid IP address"));
        let open = find_widget::<gtk::Button>(&broken, &|button| {
            button.label().as_deref() == Some("Current")
        })
        .unwrap();
        assert!(!open.is_sensitive());
        let status = find_widget::<adw::StatusPage>(reopened.widget(), &|_| true).unwrap();
        assert!(!status.title().starts_with("Connected to"));
        assert_eq!(status.description().as_deref(), Some("Invalid IP address"));

        let home =
            find_widget::<adw::ActionRow>(reopened.widget(), &|row| row.title() == "Home").unwrap();
        click(&home, "Open");
        wait_for(|| status.title() == "Connected to Home");
        let home =
            find_widget::<adw::ActionRow>(reopened.widget(), &|row| row.title() == "Home").unwrap();
        click(&home, "Make Default");
        wait_for(|| !settings.get::<Vec<(String, String, u16, bool)>>("servers")[0].3);
        let records = settings.get::<Vec<(String, String, u16, bool)>>("servers");
        assert_eq!(records[0].1, "not-an-ip");
        assert!(records[1].3);
        reopened.widget().close();
    }
}
