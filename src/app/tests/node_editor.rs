use super::*;
use crate::{
    api::node::ImportArgument,
    ui::nodes::{NodeMsg, Nodes},
};

pub fn create_and_edit(runtime: &tokio::runtime::Runtime) {
    let wing = Wing::start();
    let api = runtime.block_on(async {
        Api::new(format!("http://{}", wing.address).parse().unwrap())
            .authenticate(
                &Credentials::new("editor-admin", "editor-password-42"),
                true,
            )
            .await
            .unwrap()
            .0
    });
    let controller = Nodes::builder().launch(api.clone()).detach();
    let window = adw::ApplicationWindow::builder()
        .content(controller.widget())
        .default_width(700)
        .default_height(720)
        .resizable(false)
        .build();
    window.present();
    wait_for(|| window.width() == 700);
    window.set_resizable(true);
    let configure = find_widget::<gtk::Button>(&window, &|b| {
        b.tooltip_text().as_deref() == Some("Configure Node Manually")
    })
    .unwrap();
    assert!(
        configure.is_sensitive(),
        "Manual node configuration must be available"
    );
    configure.emit_clicked();
    wait_for(|| find_widget::<adw::ComboRow>(&window, &|r| r.title() == "Protocol").is_some());
    assert!(
        window.visible_dialog().is_none(),
        "Node configuration is an editor page"
    );
    let protocol = find_widget::<adw::ComboRow>(&window, &|r| r.title() == "Protocol").unwrap();
    let choices = protocol
        .model()
        .unwrap()
        .downcast::<gtk::StringList>()
        .unwrap();
    let index = (0..choices.n_items())
        .find(|i| choices.string(*i).as_deref() == Some("Trojan"))
        .unwrap();
    find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Name")
        .unwrap()
        .set_text("Preserved");
    find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Server")
        .unwrap()
        .set_text("127.0.0.2");
    find_widget::<adw::SpinRow>(&window, &|r| r.title() == "Port")
        .unwrap()
        .set_value(8443.0);
    protocol.set_selected(index);
    wait_for(|| {
        find_widget::<adw::PasswordEntryRow>(&window, &|r| r.title() == "Password").is_some()
    });
    assert_eq!(
        find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Name")
            .unwrap()
            .text(),
        "Preserved"
    );
    assert_eq!(
        find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Server")
            .unwrap()
            .text(),
        "127.0.0.2"
    );
    assert_eq!(
        find_widget::<adw::SpinRow>(&window, &|r| r.title() == "Port")
            .unwrap()
            .value(),
        8443.0
    );
    find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Server")
        .unwrap()
        .set_text("");
    click(&window, "Save");
    wait_for(|| {
        find_widget::<adw::EntryRow>(&window, &|r| {
            r.title() == "Server" && r.has_css_class("error")
        })
        .is_some()
    });
    for (title, value) in [
        ("Tag", "Manual"),
        ("Name", "Original"),
        ("Server", "127.0.0.1"),
    ] {
        find_widget::<adw::EntryRow>(&window, &|r| r.title() == title)
            .unwrap()
            .set_text(value);
    }
    find_widget::<adw::PasswordEntryRow>(&window, &|r| r.title() == "Password")
        .unwrap()
        .set_text("secret:@/# 密码");
    click(&window, "Save");
    wait_for(|| find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Manual").is_some());
    let node = runtime.block_on(api.nodes()).unwrap().remove(0);
    assert_eq!(node.name, "Original");
    assert_eq!(node.protocol, "trojan");
    let row = find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Manual").unwrap();
    wait_for(|| {
        find_widget::<gtk::Button>(&row, &|b| {
            b.icon_name().as_deref() == Some("document-edit-symbolic") && b.is_sensitive()
        })
        .is_some()
    });
    click_icon(&row, "document-edit-symbolic");
    wait_for(|| {
        find_widget::<adw::PasswordEntryRow>(&window, &|r| r.text() == "secret:@/# 密码").is_some()
    });
    assert_eq!(
        find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Server")
            .unwrap()
            .text(),
        "127.0.0.1"
    );
    let tag = find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Tag").unwrap();
    tag.set_text("Renamed");
    wait_for(|| {
        find_widget::<gtk::Button>(&window, &|b| {
            b.label().as_deref() == Some("Reset") && b.is_sensitive()
        })
        .is_some()
    });
    click(&window, "Reset");
    wait_for(|| {
        find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Tag" && r.text() == "Manual")
            .is_some()
    });
    let tag = find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Tag").unwrap();
    tag.set_text("Renamed");
    find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Name")
        .unwrap()
        .set_text("Edited");
    let manager = adw::StyleManager::default();
    let original_theme = manager.color_scheme();
    for (name, scheme) in [
        ("light", adw::ColorScheme::ForceLight),
        ("dark", adw::ColorScheme::ForceDark),
    ] {
        manager.set_color_scheme(scheme);
        let changed = Instant::now();
        wait_for(|| changed.elapsed() >= Duration::from_millis(300));
        capture(&window, &format!("node-editor-{name}"));
    }
    window.set_default_size(390, 640);
    wait_for(|| window.width() == 390);
    rust_i18n::set_locale("zh_CN");
    controller.emit(NodeMsg::Retranslate);
    wait_for(|| find_widget::<adw::EntryRow>(&window, &|r| r.title() == "服务器地址").is_some());
    for (name, scheme) in [
        ("dark", adw::ColorScheme::ForceDark),
        ("light", adw::ColorScheme::ForceLight),
    ] {
        manager.set_color_scheme(scheme);
        let changed = Instant::now();
        wait_for(|| changed.elapsed() >= Duration::from_millis(300));
        capture(&window, &format!("node-editor-narrow-zh-{name}"));
    }
    rust_i18n::set_locale("en");
    controller.emit(NodeMsg::Retranslate);
    wait_for(|| find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Server").is_some());
    window.set_default_size(700, 720);
    wait_for(|| window.width() == 700);
    manager.set_color_scheme(original_theme);
    wait_for(|| {
        find_widget::<gtk::Button>(&window, &|b| {
            b.label().as_deref() == Some("Save") && b.is_sensitive()
        })
        .is_some()
    });
    click(&window, "Save");
    wait_for(|| find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Renamed").is_some());
    let edited = runtime.block_on(api.nodes()).unwrap().remove(0);
    assert_eq!(
        edited.id, node.id,
        "Editing must retain node identity and group references"
    );
    assert_eq!(edited.name, "Edited");
    assert_eq!(edited.tag.as_deref(), Some("Renamed"));
    // A real uniqueness error after updateNode must retain the edit and allow retrying the tag.
    runtime
        .block_on(api.import_nodes(vec![ImportArgument {
            link: "socks5://127.0.0.1:1080#Other".into(),
            tag: Some("Reserved".into()),
        }]))
        .unwrap();
    let row = find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Renamed").unwrap();
    wait_for(|| {
        find_widget::<gtk::Button>(&row, &|b| {
            b.icon_name().as_deref() == Some("document-edit-symbolic") && b.is_sensitive()
        })
        .is_some()
    });
    click_icon(&row, "document-edit-symbolic");
    wait_for(|| {
        find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Tag" && r.is_mapped()).is_some()
    });
    find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Tag")
        .unwrap()
        .set_text("Reserved");
    find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Name")
        .unwrap()
        .set_text("Partially saved");
    wait_for(|| {
        find_widget::<gtk::Button>(&window, &|b| {
            b.label().as_deref() == Some("Save") && b.is_sensitive()
        })
        .is_some()
    });
    click(&window, "Save");
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    assert_eq!(
        dialog
            .clone()
            .downcast::<adw::AlertDialog>()
            .unwrap()
            .heading()
            .as_deref(),
        Some("Node configuration saved, but its tag could not be updated")
    );
    capture(&window, "node-editor-partial-save");
    let records = runtime.block_on(api.nodes()).unwrap();
    let actual = records.iter().find(|n| n.id == node.id).unwrap();
    assert_eq!(actual.name, "Partially saved");
    assert_eq!(actual.tag.as_deref(), Some("Renamed"));
    click(&dialog, "Close");
    find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Tag")
        .unwrap()
        .set_text("Recovered");
    wait_for(|| {
        find_widget::<gtk::Button>(&window, &|b| {
            b.label().as_deref() == Some("Save") && b.is_sensitive()
        })
        .is_some()
    });
    click(&window, "Save");
    wait_for(|| find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Recovered").is_some());
    let records = runtime.block_on(api.nodes()).unwrap();
    assert_eq!(
        records
            .iter()
            .find(|n| n.id == node.id)
            .unwrap()
            .tag
            .as_deref(),
        Some("Recovered")
    );

    // Leaving a dirty editor requires confirmation and does not write changes.
    let row = find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Recovered").unwrap();
    wait_for(|| {
        find_widget::<gtk::Button>(&row, &|b| {
            b.icon_name().as_deref() == Some("document-edit-symbolic") && b.is_sensitive()
        })
        .is_some()
    });
    click_icon(&row, "document-edit-symbolic");
    wait_for(|| {
        find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Tag" && r.is_mapped()).is_some()
    });
    find_widget::<adw::EntryRow>(&window, &|r| r.title() == "Tag")
        .unwrap()
        .set_text("Unsaved");
    wait_for(|| {
        find_widget::<gtk::Button>(&window, &|b| {
            b.label().as_deref() == Some("Reset") && b.is_sensitive()
        })
        .is_some()
    });
    click_icon(&window, "go-previous-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Discard");
    wait_for(|| {
        controller
            .widget()
            .visible_page()
            .is_some_and(|p| p.title() == "Nodes")
    });
    let records = runtime.block_on(api.nodes()).unwrap();
    assert_eq!(
        records
            .iter()
            .find(|n| n.id == node.id)
            .unwrap()
            .tag
            .as_deref(),
        Some("Recovered")
    );
    // Exercise protocol-dependent native controls and import the generated SIP003 link.
    configure.emit_clicked();
    wait_for(|| {
        find_widget::<adw::ComboRow>(&window, &|r| r.title() == "Protocol" && r.is_mapped())
            .is_some()
    });
    let protocol = find_widget::<adw::ComboRow>(&window, &|r| r.title() == "Protocol").unwrap();
    let choices = protocol
        .model()
        .unwrap()
        .downcast::<gtk::StringList>()
        .unwrap();
    protocol.set_selected(
        (0..choices.n_items())
            .find(|i| choices.string(*i).as_deref() == Some("Shadowsocks"))
            .unwrap(),
    );
    wait_for(|| find_widget::<adw::ComboRow>(&window, &|r| r.title() == "Plugin").is_some());
    find_widget::<adw::ComboRow>(&window, &|r| r.title() == "Plugin")
        .unwrap()
        .set_selected(2);
    wait_for(|| {
        find_widget::<adw::SwitchRow>(&window, &|r| r.title() == "Enable TLS" && r.is_visible())
            .is_some()
    });
    for (title, value) in [
        ("Tag", "Plugin node"),
        ("Server", "127.0.0.1"),
        ("Name", "SS"),
    ] {
        find_widget::<adw::EntryRow>(&window, &|r| r.title() == title)
            .unwrap()
            .set_text(value);
    }
    find_widget::<adw::PasswordEntryRow>(&window, &|r| r.title() == "Password" && r.is_visible())
        .unwrap()
        .set_text("password");
    let page = find_widget::<adw::PreferencesPage>(&window, &|p| p.is_mapped()).unwrap();
    let scroll = find_widget::<gtk::ScrolledWindow>(&page, &|_| true).unwrap();
    let adjustment = scroll.vadjustment();
    wait_for(|| {
        find_widget::<adw::SwitchRow>(&window, &|r| {
            r.title() == "Enable TLS" && r.is_mapped() && r.height() > 0
        })
        .is_some()
    });
    adjustment.set_value(adjustment.upper());
    for (name, scheme) in [
        ("light", adw::ColorScheme::ForceLight),
        ("dark", adw::ColorScheme::ForceDark),
    ] {
        manager.set_color_scheme(scheme);
        let changed = Instant::now();
        wait_for(|| changed.elapsed() >= Duration::from_millis(300));
        capture(&window, &format!("node-editor-plugin-{name}"));
    }
    manager.set_color_scheme(original_theme);
    click(&window, "Save");
    wait_for(|| find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Plugin node").is_some());
    let records = runtime.block_on(api.nodes()).unwrap();
    let plugin_node = records
        .iter()
        .find(|n| n.tag.as_deref() == Some("Plugin node"))
        .unwrap();
    let link = reqwest::Url::parse(&plugin_node.link).unwrap();
    assert_eq!(
        link.query_pairs().find(|(k, _)| k == "plugin").unwrap().1,
        "v2ray-plugin"
    );
    crate::ui::nodes::form_tests::form_links_import_into_backend(&api, runtime);
    window.set_content(None::<&gtk::Widget>);
    drop(controller);
    window.close();
}
