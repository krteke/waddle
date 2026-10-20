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
    find_widget::<adw::EntryRow>(dialog, &|row| row.title() == title)
        .unwrap()
        .set_text(value);
}

fn password(dialog: &adw::Dialog, value: &str) {
    find_widget::<adw::PasswordEntryRow>(dialog, &|_| true)
        .unwrap()
        .set_text(value);
}

fn connected(window: &adw::ApplicationWindow, title: &str) -> bool {
    find_widget::<adw::StatusPage>(window, &|page| page.is_mapped() && page.title() == title)
        .is_some()
}

fn language(window: &adw::ApplicationWindow, language: &str) {
    gtk::prelude::WidgetExt::activate_action(
        window,
        "preferences.language",
        Some(&language.to_variant()),
    )
    .unwrap();
}

#[test]
#[ignore = "requires GTK, DAE_WING_BIN and isolated Secret Service; run scripts/test-e2e.sh"]
fn server_management_and_authentication_lifecycle() {
    let home = Wing::start();
    let office = Wing::start();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        Api::new(office.address)
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
    click_icon(window, "network-server-symbolic");
    let navigation = find_widget::<adw::NavigationView>(window, &|_| true).unwrap();
    assert_eq!(navigation.visible_page_tag().as_deref(), Some("servers"));
    gtk::prelude::WidgetExt::activate_action(
        &navigation.visible_page().unwrap(),
        "navigation.pop",
        None,
    )
    .unwrap();
    assert_eq!(navigation.visible_page_tag().as_deref(), Some("home"));
    click(window, "Add Server");
    wait_for(|| window.visible_dialog().is_some());
    fill(&dialog, "Server Name", "Home");
    fill(&dialog, "IP Address", "127.0.0.1");
    fill(&dialog, "Port", &home.address.port().to_string());
    click(&dialog, "Connect");

    wait_for(|| {
        window
            .visible_dialog()
            .is_some_and(|dialog| dialog.title() == "Create Administrator")
    });
    assert!(!connected(window, "Connected to Home"));
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
            .block_on(Api::new(home.address).has_admin())
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
    wait_for(|| connected(window, "已连接 Home"));
    assert!(window.visible_dialog().is_none());
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
    wait_for(|| connected(window, "Connected to Home"));
    assert!(
        find_widget::<gtk::Label>(window, &|label| label.is_mapped()
            && label.text().contains("Signed in as home-admin<&>"))
        .is_some()
    );

    click_icon(window, "network-server-symbolic");
    click_icon(window, "list-add-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    fill(&dialog, "Server Name", "Home");
    fill(&dialog, "IP Address", "127.0.0.1");
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
    assert!(!connected(window, "Connected to Office"));
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
    wait_for(|| connected(window, "Connected to Office"));
    click_icon(window, "network-server-symbolic");
    let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Office").unwrap();
    click_icon(&row, "non-starred-symbolic");
    wait_for(|| ServerSettings::load(settings.clone()).default_server_index() == Some(1));

    // Switching back uses Home's credentials, not Office's.
    let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Home").unwrap();
    adw::prelude::ActionRowExt::activate(&row);
    wait_for(|| connected(window, "Connected to Home"));
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
    wait_for(|| connected(window, "已连接 Office"));
    assert!(window.visible_dialog().is_none());
    language(window, "en");
    wait_for(|| connected(window, "Connected to Office"));
    click(window, "Sign Out");
    wait_for(|| window.visible_dialog().is_some());
    assert!(!connected(window, "Connected to Office"));
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
    window.close();
    drop(controller);

    let controller = App::builder()
        .launch(ServerSettings::load(settings.clone()))
        .detach();
    let window = controller.widget();
    window.present();
    wait_for(|| window.visible_dialog().is_some());
    assert!(!connected(window, "Connected to Office"));
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
    wait_for(|| connected(window, "Connected to Office"));
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
    wait_for(|| connected(window, "Connected to Office"));
    assert!(window.visible_dialog().is_none());
    click(window, "Sign Out");
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
    wait_for(|| connected(window, "Connected to Office"));
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
    wait_for(|| connected(window, "Connected to Office"));
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
                ("Broken", "not-an-ip", home.address.port(), true, false),
                ("Office", "127.0.0.1", office.address.port(), false, false),
            ],
        )
        .unwrap();
    let controller = App::builder()
        .launch(ServerSettings::load(settings.clone()))
        .detach();
    let window = controller.widget();
    window.present();
    wait_for(|| connected(window, "Broken · Invalid address"));
    assert!(window.visible_dialog().is_none());
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
        find_widget::<adw::EntryRow>(&dialog, &|row| row.title() == "IP Address")
            .unwrap()
            .text(),
        "not-an-ip"
    );
    fill(&dialog, "IP Address", "127.0.0.1");
    click(&dialog, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    assert_eq!(
        settings.get::<Vec<(String, String, u16, bool, bool)>>("servers")[0].1,
        "not-an-ip"
    );
    let broken = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Broken").unwrap();
    click_icon(&broken, "document-edit-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    fill(&dialog, "Server Name", "Home");
    fill(&dialog, "IP Address", "127.0.0.1");
    click(&dialog, "Save");
    wait_for(|| window.visible_dialog().is_none());
    assert_eq!(navigation.visible_page_tag().as_deref(), Some("servers"));
    let repaired = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Home").unwrap();
    assert!(repaired.is_activatable());
    adw::prelude::ActionRowExt::activate(&repaired);
    wait_for(|| connected(window, "Connected to Home"));
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
    assert!(!connected(window, "Connected to Home"));
    assert!(window.visible_dialog().is_none());
    assert!(
        find_widget::<adw::Banner>(window, &|banner| banner.is_revealed()
            && banner.title() == "Server health check failed. Try again later.")
        .is_some()
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
