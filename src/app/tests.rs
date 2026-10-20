use super::*;
use crate::{
    api::Api,
    credentials::{self, Credentials},
    settings,
    test_support::Wing,
};
use gtk::glib;
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

#[track_caller]
fn wait_for(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "UI did not reach the expected state"
        );
        let context = glib::MainContext::default();
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn click(root: &impl IsA<gtk::Widget>, label: &str) {
    let matches =
        |button: &gtk::Button| button.label().as_deref() == Some(label) && button.is_mapped();
    wait_for(|| find_widget::<gtk::Button>(root, &matches).is_some());
    let button = find_widget::<gtk::Button>(root, &matches).unwrap();
    assert!(button.is_sensitive());
    button.emit_clicked();
}

fn click_icon(root: &impl IsA<gtk::Widget>, icon: &str) {
    let matches =
        |button: &gtk::Button| button.icon_name().as_deref() == Some(icon) && button.is_mapped();
    wait_for(|| find_widget::<gtk::Button>(root, &matches).is_some());
    let button = find_widget::<gtk::Button>(root, &matches).unwrap();
    assert!(button.is_sensitive());
    button.emit_clicked();
}

fn fill(dialog: &adw::Dialog, title: &str, value: &str) {
    if let Some(row) = find_widget::<adw::EntryRow>(dialog, &|row| row.title() == title) {
        row.set_text(value);
    } else {
        find_widget::<gtk::Entry>(dialog, &|entry| {
            entry.tooltip_text().as_deref() == Some(title)
        })
        .unwrap()
        .set_text(value);
    }
}

fn password(dialog: &adw::Dialog, value: &str) {
    find_widget::<adw::PasswordEntryRow>(dialog, &|_| true)
        .unwrap()
        .set_text(value);
}

fn connected(window: &adw::ApplicationWindow, name: &str) -> bool {
    find_widget::<gtk::Label>(window, &|label| label.is_mapped() && label.text() == name).is_some()
        && find_widget::<gtk::Label>(window, &|label| {
            label.is_mapped() && matches!(label.text().as_str(), "Connected" | "已连接")
        })
        .is_some()
}

fn language(window: &adw::ApplicationWindow, language: &str) {
    gtk::prelude::WidgetExt::activate_action(
        window,
        "preferences.language",
        Some(&language.to_variant()),
    )
    .unwrap();
    let title = if language == "zh_CN" {
        "语言"
    } else {
        "Language"
    };
    wait_for(|| {
        find_widget::<gtk::MenuButton>(window, &|button| {
            button.tooltip_text().as_deref() == Some(title)
        })
        .is_some()
    });
}

fn capture(widget: &impl IsA<gtk::Widget>, name: &str) {
    let Some(directory) = std::env::var_os("WADDLE_TEST_SCREENSHOTS") else {
        return;
    };
    let painted = std::rc::Rc::new(std::cell::Cell::new(false));
    let ticked = painted.clone();
    let widget = widget.as_ref();
    widget.add_tick_callback(move |_, _| {
        ticked.set(true);
        glib::ControlFlow::Break
    });
    wait_for(|| painted.get());
    let render = gtk::Snapshot::new();
    gtk::WidgetPaintable::new(Some(widget)).snapshot(
        &render,
        widget.width() as f64,
        widget.height() as f64,
    );
    let node = render.to_node().unwrap();
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    widget
        .native()
        .unwrap()
        .renderer()
        .unwrap()
        .render_texture(&node, None)
        .save_to_png(directory.join(format!("{name}.png")))
        .unwrap();
}

fn long_server_menu_stays_scrollable() {
    let workspace = Workspace::new(&gio::Menu::new());
    let (sender, _receiver) = relm4::channel();
    let servers: Vec<_> = (0..20)
        .map(|index| settings::Server {
            name: format!("Server {index}"),
            url: Ok("http://localhost:8080/".parse().unwrap()),
            is_default: index == 0,
            logged_out: false,
        })
        .collect();
    workspace.refresh(&servers, Some(0), None, &sender);
    let selector = find_widget::<gtk::MenuButton>(&workspace.view, &|button| {
        button.tooltip_text().as_deref() == Some("Select Server")
    })
    .unwrap();
    let scroll =
        find_widget::<gtk::ScrolledWindow>(&selector.popover().unwrap(), &|_| true).unwrap();
    assert_eq!(scroll.vscrollbar_policy(), gtk::PolicyType::Automatic);
    assert_eq!(scroll.measure(gtk::Orientation::Vertical, -1).1, 360);
}

#[test]
#[ignore = "requires GTK, DAE_WING_BIN and isolated Secret Service; run scripts/test-e2e.sh"]
fn server_management_and_authentication_lifecycle() {
    let home = Wing::start();
    let office = Wing::start();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        Api::new(format!("http://{}", office.address).parse().unwrap())
            .authenticate(
                &Credentials::new("office-admin", "office-password-42"),
                true,
            )
            .await
            .unwrap();
    });

    let _app = RelmApp::<AppMsg>::new("io.github.krteke.Waddle.Tests");
    gtk::Settings::default()
        .unwrap()
        .set_gtk_enable_animations(false);
    let settings = settings::tests::settings();
    settings.set_string("language", "en").unwrap();
    long_server_menu_stays_scrollable();
    let controller = App::builder()
        .launch(ServerSettings::load(settings.clone()))
        .detach();
    let window = controller.widget();
    window.present();
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    assert_eq!(dialog.title(), "Connect to Server");
    click(&dialog, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    let split = find_widget::<adw::OverlaySplitView>(window, &|_| true)
        .expect("home must provide a collapsible sidebar");
    let pages =
        find_widget::<gtk::Stack>(window, &|stack| stack.child_by_name("overview").is_some())
            .unwrap();
    assert!(split.shows_sidebar());
    assert_eq!(pages.visible_child_name().as_deref(), Some("overview"));
    let nodes = find_widget::<gtk::ListBoxRow>(window, &|row| {
        find_widget::<gtk::Label>(row, &|label| label.text() == "Nodes").is_some()
    })
    .unwrap();
    nodes.activate();
    wait_for(|| pages.visible_child_name().as_deref() == Some("nodes"));
    capture(window, "wide");
    click_icon(window, "sidebar-show-symbolic");
    wait_for(|| !split.shows_sidebar());
    capture(window, "hidden");
    assert_eq!(pages.visible_child_name().as_deref(), Some("nodes"));
    click_icon(window, "sidebar-show-symbolic");
    wait_for(|| split.shows_sidebar());
    window.set_default_size(480, 600);
    wait_for(|| split.is_collapsed());
    assert!(!split.shows_sidebar());
    click_icon(window, "sidebar-show-symbolic");
    wait_for(|| split.shows_sidebar());
    capture(window, "narrow");
    let subscriptions = find_widget::<gtk::ListBoxRow>(window, &|row| {
        find_widget::<gtk::Label>(row, &|label| label.text() == "Subscriptions").is_some()
    })
    .unwrap();
    subscriptions.activate();
    wait_for(|| {
        pages.visible_child_name().as_deref() == Some("subscriptions") && !split.shows_sidebar()
    });
    window.set_default_size(900, 640);
    wait_for(|| !split.is_collapsed());
    language(window, "zh_CN");
    wait_for(|| {
        find_widget::<gtk::Label>(split.sidebar().as_ref().unwrap(), &|label| {
            label.text() == "策略组"
        })
        .is_some()
    });
    assert_eq!(pages.visible_child_name().as_deref(), Some("subscriptions"));
    language(window, "en");
    let selector = find_widget::<gtk::MenuButton>(window, &|button| {
        button.tooltip_text().as_deref() == Some("Select Server")
    })
    .unwrap();
    wait_for(|| window.is_active());
    selector.popup();
    let navigation = find_widget::<adw::NavigationView>(window, &|_| true).unwrap();
    assert_eq!(navigation.visible_page_tag().as_deref(), Some("servers"));
    gtk::prelude::WidgetExt::activate_action(
        &navigation.visible_page().unwrap(),
        "navigation.pop",
        None,
    )
    .unwrap();
    assert_eq!(navigation.visible_page_tag().as_deref(), Some("home"));
    click_icon(window, "network-server-symbolic");
    click(window, "Add Server");
    wait_for(|| window.visible_dialog().is_some());
    fill(&dialog, "Server Name", "Home");
    fill(&dialog, "Host", "localhost");
    fill(&dialog, "Port", &home.address.port().to_string());
    capture(window, "server-address");
    fill(&dialog, "Host", "bad host");
    click(&dialog, "Connect");
    wait_for(|| {
        find_widget::<gtk::Label>(&dialog, &|label| {
            label.is_visible() && label.text() == "Enter a valid server URL."
        })
        .is_some()
    });
    assert!(ServerSettings::load(settings.clone()).servers().is_empty());
    fill(&dialog, "Host", "localhost");
    click(&dialog, "Connect");

    wait_for(|| {
        window
            .visible_dialog()
            .is_some_and(|dialog| dialog.title() == "Create Administrator")
    });
    assert!(!connected(window, "Home"));
    let login = window.visible_dialog().unwrap();
    let remember = find_widget::<adw::SwitchRow>(&login, &|row| row.title() == "Save Password")
        .expect("login must offer an optional Save Password switch");
    assert!(!remember.is_active());
    remember.set_active(true);
    fill(&login, "Username", "home-admin<&>");
    password(&login, "weak");
    click(&login, "Create Account");
    wait_for(|| {
        find_widget::<gtk::Label>(&login, &|label| {
            label.text()
                == "Could not create the administrator. Check the account details and try again."
                && label.is_visible()
        })
        .is_some()
    });
    assert!(
        !runtime
            .block_on(Api::new(format!("http://{}", home.address).parse().unwrap()).has_admin())
            .unwrap()
    );
    password(&login, "home-password-42");
    language(window, "zh_CN");
    wait_for(|| login.title() == "创建管理员");
    assert_eq!(
        find_widget::<adw::EntryRow>(&login, &|row| row.title() == "用户名")
            .unwrap()
            .text(),
        "home-admin<&>"
    );
    assert_eq!(
        find_widget::<adw::PasswordEntryRow>(&login, &|_| true)
            .unwrap()
            .text(),
        "home-password-42"
    );
    click(&login, "创建账号");
    wait_for(|| connected(window, "Home"));
    assert!(window.visible_dialog().is_none());
    assert!(
        find_widget::<gtk::Button>(split.sidebar().as_ref().unwrap(), &|button| {
            button.icon_name().as_deref() == Some("system-log-out-symbolic")
        })
        .is_none(),
        "sign out belongs in the overview, not the sidebar"
    );
    let proxy = find_widget::<adw::SwitchRow>(split.sidebar().as_ref().unwrap(), &|row| {
        row.title() == "代理服务"
    })
    .expect("authenticated sidebar must show the proxy switch");
    assert!(proxy.is_visible());
    assert!(!proxy.is_sensitive(), "proxy control is a placeholder");
    let overview = find_widget::<gtk::ListBoxRow>(split.sidebar().as_ref().unwrap(), &|row| {
        find_widget::<gtk::Label>(row, &|label| label.text() == "概览").is_some()
    })
    .unwrap();
    overview.activate();
    wait_for(|| {
        find_widget::<adw::ActionRow>(window, &|row| {
            row.is_mapped()
                && row.title() == "登录状态"
                && row.subtitle().as_deref() == Some("已登录为 home-admin<&>")
        })
        .is_some()
    });
    capture(window, "connected-zh");
    assert_eq!(
        ServerSettings::load(settings.clone()).default_server_index(),
        Some(0)
    );
    runtime.block_on(async {
        let saved = credentials::load("Home").await.unwrap().unwrap();
        assert_eq!(saved.username, "home-admin<&>");
        assert_eq!(saved.password.as_deref(), Some("home-password-42"));
        assert!(saved.jwt.as_deref().is_some());
    });
    let menu = find_widget::<gtk::MenuButton>(window, &|button| {
        button.icon_name().as_deref() == Some("preferences-desktop-locale-symbolic")
    })
    .unwrap();
    assert_eq!(menu.tooltip_text().as_deref(), Some("语言"));
    language(window, "en");
    wait_for(|| connected(window, "Home"));
    let selector = find_widget::<gtk::MenuButton>(window, &|button| {
        button.tooltip_text().as_deref() == Some("Select Server")
    })
    .unwrap();
    selector.popup();
    let popover = selector.popover().unwrap();
    wait_for(|| popover.is_mapped());
    capture(&popover, "single-server-selector");
    let scroll = find_widget::<gtk::ScrolledWindow>(&popover, &|_| true).unwrap();
    let list = find_widget::<gtk::Box>(&scroll, &|list| {
        list.first_child()
            .is_some_and(|child| child.is::<gtk::Button>())
    })
    .unwrap();
    assert_eq!(
        scroll.measure(gtk::Orientation::Vertical, -1).1,
        list.measure(gtk::Orientation::Vertical, -1).1,
        "a single-server menu must fit its content without scrollbar padding"
    );
    selector.popdown();
    assert!(
        find_widget::<gtk::Label>(window, &|label| label.is_mapped()
            && label
                .tooltip_text()
                .is_some_and(|text| text.contains("Signed in as home-admin<&>")))
        .is_some()
    );

    click_icon(window, "network-server-symbolic");
    let home_row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Home").unwrap();
    assert_eq!(
        home_row.subtitle().as_deref(),
        Some(format!("http://localhost:{}/", home.address.port()).as_str())
    );
    click_icon(&home_row, "document-edit-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let edit = window.visible_dialog().unwrap();
    assert_eq!(
        find_widget::<gtk::Entry>(&edit, &|entry| {
            entry.tooltip_text().as_deref() == Some("Host")
        })
        .unwrap()
        .text(),
        "localhost"
    );
    assert_eq!(
        find_widget::<gtk::Entry>(&edit, &|entry| {
            entry.tooltip_text().as_deref() == Some("Port")
        })
        .unwrap()
        .text(),
        home.address.port().to_string()
    );
    click(&edit, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    click_icon(window, "list-add-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    fill(&dialog, "Server Name", "Home");
    fill(&dialog, "Host", "localhost");
    fill(&dialog, "Port", &office.address.port().to_string());
    click(&dialog, "Connect");
    wait_for(|| {
        find_widget::<gtk::Label>(&dialog, &|label| {
            label.text() == "A server with this name already exists."
        })
        .is_some()
    });
    assert_eq!(ServerSettings::load(settings.clone()).servers().len(), 1);
    fill(&dialog, "Server Name", "Office");
    click(&dialog, "Connect");

    wait_for(|| {
        window
            .visible_dialog()
            .is_some_and(|dialog| dialog.title() == "Sign In")
    });
    let login = window.visible_dialog().unwrap();
    assert!(
        find_widget::<adw::EntryRow>(&login, &|row| row.title() == "Username")
            .unwrap()
            .text()
            .is_empty()
    );
    password(&login, "discard-this-password");
    click(&login, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    assert!(
        find_widget::<adw::SwitchRow>(split.sidebar().as_ref().unwrap(), &|row| row.is_mapped())
            .is_none(),
        "proxy control must be hidden until signed in"
    );
    assert!(
        find_widget::<adw::StatusPage>(window, &|page| page.is_mapped()
            && page.title() == "Overview")
        .is_some(),
        "an unsigned server keeps the overview placeholder"
    );
    assert!(
        runtime
            .block_on(credentials::load("Office"))
            .unwrap()
            .is_none()
    );
    click(window, "Sign In");
    wait_for(|| window.visible_dialog().is_some());
    assert!(
        find_widget::<adw::PasswordEntryRow>(&login, &|_| true)
            .unwrap()
            .text()
            .is_empty()
    );
    fill(&login, "Username", "office-admin");
    password(&login, "incorrect-password");
    click(&login, "Sign In");
    wait_for(|| {
        find_widget::<gtk::Label>(&login, &|label| {
            label.text() == "Could not sign in. Check your username and password."
                && label.is_visible()
        })
        .is_some()
    });
    assert!(!connected(window, "Office"));
    assert!(
        runtime
            .block_on(credentials::load("Office"))
            .unwrap()
            .is_none()
    );
    password(&login, "office-password-42");
    find_widget::<adw::SwitchRow>(&login, &|row| row.title() == "Save Password")
        .unwrap()
        .set_active(true);
    click(&login, "Sign In");
    wait_for(|| connected(window, "Office"));
    click_icon(window, "network-server-symbolic");
    let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Office").unwrap();
    click_icon(&row, "non-starred-symbolic");
    wait_for(|| ServerSettings::load(settings.clone()).default_server_index() == Some(1));

    // The sidebar selector changes the managed server without opening management.
    navigation.pop_to_tag("home");
    let selector = find_widget::<gtk::MenuButton>(window, &|button| {
        button.tooltip_text().as_deref() == Some("Select Server")
    })
    .unwrap();
    wait_for(|| selector.is_mapped());
    assert!(
        !selector.is_active(),
        "closed selector must not stay active"
    );
    selector.popup();
    let popover = selector.popover().unwrap();
    wait_for(|| popover.is_mapped());
    capture(&popover, "server-selector");
    assert_eq!(navigation.visible_page_tag().as_deref(), Some("home"));
    assert!(
        find_widget::<gtk::Button>(&popover, &|button| {
            button.icon_name().as_deref() == Some("list-add-symbolic")
        })
        .is_none()
    );
    let home_button = find_widget::<gtk::Button>(&popover, &|button| {
        find_widget::<gtk::Label>(button, &|label| label.text() == "Home").is_some()
    })
    .unwrap();
    home_button.emit_clicked();
    wait_for(|| connected(window, "Home"));
    assert!(!popover.is_visible());
    assert_eq!(
        ServerSettings::load(settings.clone()).default_server_index(),
        Some(1)
    );
    assert!(window.visible_dialog().is_none());
    window.close();
    drop(controller);

    // Reopening the default server signs in with the credentials saved by the UI.
    settings.set_string("language", "zh_CN").unwrap();
    let controller = App::builder()
        .launch(ServerSettings::load(settings.clone()))
        .detach();
    let window = controller.widget();
    window.present();
    wait_for(|| connected(window, "Office"));
    assert!(window.visible_dialog().is_none());
    language(window, "en");
    wait_for(|| connected(window, "Office"));
    click_icon(window, "system-log-out-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    assert!(!connected(window, "Office"));
    let login = window.visible_dialog().unwrap();
    assert_eq!(login.title(), "Sign In");
    assert!(
        runtime
            .block_on(credentials::load("Office"))
            .unwrap()
            .unwrap()
            .jwt
            .as_deref()
            .is_none()
    );
    assert!(ServerSettings::load(settings.clone()).servers()[1].logged_out);
    assert_eq!(
        find_widget::<adw::PasswordEntryRow>(&login, &|_| true)
            .unwrap()
            .text(),
        "office-password-42"
    );
    click(&login, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    assert!(
        find_widget::<adw::StatusPage>(window, &|page| page.is_mapped()
            && page.title() == "Overview")
        .is_some(),
        "signing out must restore the overview placeholder"
    );
    assert!(find_widget::<adw::SwitchRow>(window, &|row| row.is_mapped()).is_none());
    window.close();
    drop(controller);

    let controller = App::builder()
        .launch(ServerSettings::load(settings.clone()))
        .detach();
    let window = controller.widget();
    window.present();
    wait_for(|| window.visible_dialog().is_some());
    assert!(!connected(window, "Office"));
    let login = window.visible_dialog().unwrap();
    assert_eq!(login.title(), "Sign In");
    assert_eq!(
        find_widget::<adw::PasswordEntryRow>(&login, &|_| true)
            .unwrap()
            .text(),
        "office-password-42"
    );
    let remember =
        find_widget::<adw::SwitchRow>(&login, &|row| row.title() == "Save Password").unwrap();
    assert!(remember.is_active());
    remember.set_active(false);
    click(&login, "Sign In");
    wait_for(|| connected(window, "Office"));
    assert!(!ServerSettings::load(settings.clone()).servers()[1].logged_out);
    let saved = runtime
        .block_on(credentials::load("Office"))
        .unwrap()
        .unwrap();
    assert!(saved.password.as_deref().is_none());
    assert!(saved.jwt.as_deref().is_some());
    window.close();
    drop(controller);

    // The saved JWT restores a session after opting out of password storage.
    let controller = App::builder()
        .launch(ServerSettings::load(settings.clone()))
        .detach();
    let window = controller.widget();
    window.present();
    wait_for(|| connected(window, "Office"));
    assert!(window.visible_dialog().is_none());
    click_icon(window, "system-log-out-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let login = window.visible_dialog().unwrap();
    assert_eq!(
        find_widget::<adw::PasswordEntryRow>(&login, &|_| true)
            .unwrap()
            .text(),
        ""
    );
    password(&login, "office-password-42");
    click(&login, "Sign In");
    wait_for(|| connected(window, "Office"));
    window.close();
    drop(controller);

    let office_admin = Credentials::new("office-admin", "stale-password");
    // A stale saved password opens the login form and is replaced only after success.
    runtime
        .block_on(
            Credentials {
                username: office_admin.username.clone(),
                password: office_admin.password.clone(),
                jwt: Some("rejected-token".into()),
            }
            .save("Office"),
        )
        .unwrap();
    let controller = App::builder()
        .launch(ServerSettings::load(settings.clone()))
        .detach();
    let window = controller.widget();
    window.present();
    wait_for(|| {
        window
            .visible_dialog()
            .is_some_and(|dialog| dialog.title() == "Sign In")
    });
    let login = window.visible_dialog().unwrap();
    assert_eq!(
        find_widget::<adw::EntryRow>(&login, &|row| row.title() == "Username")
            .unwrap()
            .text(),
        "office-admin"
    );
    assert!(
        find_widget::<adw::PasswordEntryRow>(&login, &|_| true)
            .unwrap()
            .text()
            .is_empty()
    );
    password(&login, "office-password-42");
    click(&login, "Sign In");
    wait_for(|| connected(window, "Office"));
    assert_eq!(
        runtime
            .block_on(credentials::load("Office"))
            .unwrap()
            .unwrap()
            .password
            .as_deref(),
        Some("office-password-42")
    );
    window.close();
    drop(controller);

    settings
        .set(
            "servers",
            vec![
                ("Broken", "not-a-url".to_string(), true, false),
                (
                    "Office",
                    format!("http://127.0.0.1:{}/", office.address.port()),
                    false,
                    false,
                ),
            ],
        )
        .unwrap();
    let controller = App::builder()
        .launch(ServerSettings::load(settings.clone()))
        .detach();
    let window = controller.widget();
    window.present();
    wait_for(|| {
        find_widget::<gtk::Label>(window, &|label| {
            label.is_mapped() && label.text() == "Invalid server URL"
        })
        .is_some()
    });
    wait_for(|| window.is_active());
    assert!(window.visible_dialog().is_none());
    let selector = find_widget::<gtk::MenuButton>(window, &|button| {
        button.tooltip_text().as_deref() == Some("Select Server")
    })
    .unwrap();
    selector.popup();
    let popover = selector.popover().unwrap();
    wait_for(|| popover.is_mapped());
    let invalid = find_widget::<gtk::Button>(&popover, &|button| {
        find_widget::<gtk::Label>(button, &|label| label.text() == "Broken").is_some()
    })
    .unwrap();
    assert!(!invalid.is_sensitive());
    assert_eq!(
        invalid.tooltip_text().as_deref(),
        Some("Invalid server URL")
    );
    selector.popdown();
    click_icon(window, "network-server-symbolic");
    let broken = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Broken").unwrap();
    assert!(!broken.is_activatable());
    adw::prelude::ActionRowExt::activate(&broken);
    let navigation = find_widget::<adw::NavigationView>(window, &|_| true).unwrap();
    assert_eq!(navigation.visible_page_tag().as_deref(), Some("servers"));
    click_icon(&broken, "document-edit-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    assert_eq!(
        find_widget::<gtk::Entry>(&dialog, &|entry| entry.tooltip_text().as_deref()
            == Some("Host"))
        .unwrap()
        .text(),
        "not-a-url"
    );
    fill(&dialog, "Host", "localhost");
    click(&dialog, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    assert_eq!(
        settings.get::<Vec<(String, String, bool, bool)>>("servers")[0].1,
        "not-a-url"
    );
    let broken = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Broken").unwrap();
    click_icon(&broken, "document-edit-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    fill(&dialog, "Server Name", "Home");
    fill(&dialog, "Host", "localhost");
    fill(&dialog, "Port", &home.address.port().to_string());
    click(&dialog, "Save");
    wait_for(|| window.visible_dialog().is_none());
    assert_eq!(navigation.visible_page_tag().as_deref(), Some("servers"));
    let repaired = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Home").unwrap();
    assert!(repaired.is_activatable());
    adw::prelude::ActionRowExt::activate(&repaired);
    wait_for(|| connected(window, "Home"));
    assert!(window.visible_dialog().is_none());
    drop(home);
    click_icon(window, "network-server-symbolic");
    let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Home").unwrap();
    adw::prelude::ActionRowExt::activate(&row);
    wait_for(|| {
        find_widget::<gtk::Button>(window, &|button| {
            button.is_mapped() && button.label().as_deref() == Some("Try Again")
        })
        .is_some()
    });
    assert!(!connected(window, "Home"));
    assert!(window.visible_dialog().is_none());
    assert!(
        find_widget::<gtk::Label>(window, &|label| label.is_mapped()
            && label.text() == "Server health check failed. Try again later.")
        .is_some()
    );
    click_icon(window, "network-server-symbolic");
    let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Home").unwrap();
    click_icon(&row, "user-trash-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let confirm = window.visible_dialog().unwrap();
    click(&confirm, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    assert_eq!(ServerSettings::load(settings.clone()).servers().len(), 2);
    assert!(
        runtime
            .block_on(credentials::load("Home"))
            .unwrap()
            .is_some()
    );
    let office_row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Office").unwrap();
    adw::prelude::ActionRowExt::activate(&office_row);
    wait_for(|| connected(window, "Office"));
    click_icon(window, "network-server-symbolic");
    let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Home").unwrap();
    click_icon(&row, "user-trash-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Delete");
    wait_for(|| {
        window.is_sensitive() && ServerSettings::load(settings.clone()).servers().len() == 1
    });
    let remaining = ServerSettings::load(settings.clone());
    assert_eq!(remaining.servers()[0].name, "Office");
    assert!(remaining.servers()[0].is_default);
    assert!(
        runtime
            .block_on(credentials::load("Home"))
            .unwrap()
            .is_none()
    );
    assert!(
        runtime
            .block_on(credentials::load("Office"))
            .unwrap()
            .is_some()
    );
    gtk::prelude::WidgetExt::activate_action(
        &navigation.visible_page().unwrap(),
        "navigation.pop",
        None,
    )
    .unwrap();
    wait_for(|| connected(window, "Office"));
    // Removing an earlier row must not break the active connection's messages.
    click_icon(window, "system-log-out-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let login = window.visible_dialog().unwrap();
    assert_eq!(login.title(), "Sign In");
    assert!(ServerSettings::load(settings.clone()).servers()[0].logged_out);
    click(&login, "Sign In");
    wait_for(|| connected(window, "Office"));
    click_icon(window, "network-server-symbolic");
    let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Office").unwrap();
    click_icon(&row, "user-trash-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Delete");
    wait_for(|| {
        window.is_sensitive() && ServerSettings::load(settings.clone()).servers().is_empty()
    });
    assert!(
        find_widget::<adw::StatusPage>(window, &|page| page.is_mapped()
            && page.title() == "Add your first server")
        .is_some()
    );
    assert!(
        runtime
            .block_on(credentials::load("Office"))
            .unwrap()
            .is_none()
    );
    window.close();
    drop(controller);

    runtime.block_on(async {
        let keyring = oo7::Keyring::new().await.unwrap();
        keyring
            .delete(&[
                ("application", settings::APPLICATION_ID),
                ("server", "Home"),
            ])
            .await
            .unwrap();
        keyring
            .delete(&[
                ("application", settings::APPLICATION_ID),
                ("server", "Office"),
            ])
            .await
            .unwrap();
    });
}
