use super::*;
use crate::{
    api::group::Policy,
    routing::{MacAction, Mode, Preset},
};

pub fn manage(runtime: &tokio::runtime::Runtime, window: &adw::ApplicationWindow, wing: &Wing) {
    let api = runtime.block_on(async {
        let saved = credentials::load("Home").await.unwrap().unwrap();
        Api::new(format!("http://{}", wing.address).parse().unwrap())
            .with_token(saved.jwt.as_deref().unwrap())
            .unwrap()
    });
    runtime.block_on(async {
        api.create_group("proxy".into(), Policy::Random)
            .await
            .unwrap();
        api.create_routing("existing-custom".into(), "fallback: direct".into())
            .await
            .unwrap();
    });
    navigation_row(window, "Routing").activate();
    wait_for(|| {
        find_widget::<adw::NavigationPage>(window, &|p| {
            p.is_mapped() && p.tag().as_deref() == Some("routing-list")
        })
        .is_some()
    });
    let page =
        find_widget::<adw::NavigationPage>(window, &|p| p.tag().as_deref() == Some("routing-list"))
            .unwrap();
    // Observe catalog contents, not the brief in-flight state of a local HTTP request.
    click_icon(&page, "view-refresh-symbolic");
    wait_row(&page, "existing-custom");
    let actions = menu(&page, "existing-custom");
    click(&actions, "Delete");
    let deletion = alert(window, "Delete Routing?");
    click(&deletion, "Delete");
    wait_for(|| window.visible_dialog().is_none());
    wait_for(|| {
        find_widget::<adw::StatusPage>(&page, &|p| p.is_mapped() && p.title() == "No Routings")
            .is_some()
    });
    click_icon(&page, "list-add-symbolic");
    let editor = current_editor(window);
    assert!(!button(&editor, "Reset").is_sensitive());
    click(&editor, "Save");
    wait_for(|| {
        find_widget::<adw::EntryRow>(&editor, &|r| {
            r.title() == "Name" && r.has_css_class("error")
        })
        .is_some()
    });
    fill(&editor, "Name", "route-test");
    themes(window, "routing-presets");
    click(&editor, "Save");
    wait_for(|| window.visible_dialog().is_none());
    wait_row(&page, "route-test");
    let created = runtime.block_on(api.routings()).unwrap().routings.remove(0);
    let gfw = Preset {
        mode: Mode::Gfw,
        group: "proxy".into(),
        macs: vec![],
        mac_action: MacAction::Proxy,
    };
    assert_eq!(created.routing.string.trim(), gfw.text());
    adw::prelude::ActionRowExt::activate(&row(&page, "route-test"));
    wait_for(|| {
        find_widget::<gtk::CheckButton>(&row(&page, "route-test"), &|c| c.is_active()).is_some()
    });
    assert!(runtime.block_on(api.routings()).unwrap().routings[0].selected);
    let actions = menu(&page, "route-test");
    assert!(!button(&actions, "Delete").is_sensitive());
    click(&actions, "View Details");
    wait_for(|| window.visible_dialog().is_some());
    let details = window.visible_dialog().unwrap();
    let text = find_widget::<gtk::TextView>(&details, &|_| true).unwrap();
    assert!(!text.is_editable());
    assert_eq!(buffer_text(&text.buffer()).trim(), gfw.text());
    details.close();
    wait_for(|| window.visible_dialog().is_none());

    click_icon(&row(&page, "route-test"), "document-edit-symbolic");
    let editor = current_editor(window);
    assert!(!button(&editor, "Save").is_sensitive());
    let outside =
        find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Outside China").unwrap();
    adw::prelude::ActionRowExt::activate(&outside);
    find_widget::<adw::SwitchRow>(&editor, &|r| r.title() == "MAC Rules")
        .unwrap()
        .set_active(true);
    wait_for(|| {
        find_widget::<gtk::Button>(&editor, &|b| {
            b.is_mapped() && b.tooltip_text().as_deref() == Some("Add MAC Address")
        })
        .is_some()
    });
    click_icon(&editor, "list-add-symbolic");
    wait_for(|| find_widget::<adw::EntryRow>(&editor, &|r| r.title() == "MAC").is_some());
    fill(&editor, "MAC", "invalid");
    click(&editor, "Save");
    wait_for(|| {
        find_widget::<adw::EntryRow>(&editor, &|r| r.title() == "MAC" && r.has_css_class("error"))
            .is_some()
    });
    assert_eq!(
        runtime.block_on(api.routings()).unwrap().routings[0]
            .routing
            .string
            .trim(),
        gfw.text()
    );
    let scroll = find_widget::<gtk::ScrolledWindow>(&editor, &|s| {
        find_widget::<adw::EntryRow>(s, &|r| r.title() == "MAC").is_some()
    })
    .unwrap();
    let adjustment = scroll.vadjustment();
    adjustment.set_value(adjustment.upper() - adjustment.page_size());
    language(window, "zh_CN");
    wait_for(|| find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "设备流量").is_some());
    themes(window, "routing-mac-zh");
    language(window, "en");
    wait_for(|| editor.title() == "Edit Routing");
    fill(&editor, "MAC", "AA:BB:CC:DD:EE:FF");
    click_icon(&editor, "list-add-symbolic");
    wait_for(|| {
        find_widget::<adw::EntryRow>(&editor, &|r| r.title() == "MAC" && r.text().is_empty())
            .is_some()
    });
    let extra =
        find_widget::<adw::EntryRow>(&editor, &|r| r.title() == "MAC" && r.text().is_empty())
            .unwrap();
    extra.set_text("11:22:33:44:55:66");
    click_icon(&extra, "list-remove-symbolic");
    wait_for(|| extra.parent().is_none());
    click(&editor, "Save");
    wait_for(|| window.visible_dialog().is_none());
    let expected = Preset {
        mode: Mode::NonCn,
        macs: vec!["AA:BB:CC:DD:EE:FF".into()],
        ..gfw
    };
    wait_for(|| button(&page, "Add Routing").is_sensitive());
    assert_eq!(
        runtime.block_on(api.routings()).unwrap().routings[0]
            .routing
            .string
            .trim(),
        expected.text()
    );

    click_icon(&row(&page, "route-test"), "document-edit-symbolic");
    let editor = current_editor(window);
    let modes = find_widget::<adw::ToggleGroup>(&editor, &|g| {
        g.toggle(0).unwrap().label().as_deref() == Some("Presets")
    })
    .unwrap();
    modes.set_active(1);
    let text = find_widget::<gtk::TextView>(&editor, &|_| true).unwrap();
    wait_for(|| text.is_mapped());
    assert_eq!(buffer_text(&text.buffer()).trim(), expected.text());
    text.buffer()
        .set_text("domain(example.com) -> direct\nfallback: proxy");
    wait_for(|| button(&editor, "Reset").is_sensitive());
    modes.set_active(0);
    let confirm = alert(window, "Replace Routing Rules?");
    click(&confirm, "Cancel");
    wait_for(|| window.visible_dialog().as_ref() == Some(&editor));
    assert_eq!(
        buffer_text(&text.buffer()),
        "domain(example.com) -> direct\nfallback: proxy"
    );
    modes.set_active(0);
    let confirm = alert(window, "Replace Routing Rules?");
    click(&confirm, "Replace");
    wait_for(|| window.visible_dialog().as_ref() == Some(&editor) && modes.active() == 0);
    assert_eq!(buffer_text(&text.buffer()), expected.text());
    adw::prelude::ActionRowExt::activate(
        &find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Global Proxy").unwrap(),
    );
    wait_for(|| button(&editor, "Reset").is_sensitive());
    click(&editor, "Reset");
    wait_for(|| !button(&editor, "Reset").is_sensitive());
    modes.set_active(1);
    wait_for(|| text.is_mapped());
    text.buffer().set_text("not valid routing syntax");
    wait_for(|| button(&editor, "Save").is_sensitive());
    click(&editor, "Save");
    let error = alert(window, "Could not save routing.");
    assert!(
        find_widget::<gtk::Label>(&error.extra_child().unwrap(), &|l| l.is_selectable()
            && !l.text().is_empty())
        .is_some()
    );
    click(&error, "Close");
    wait_for(|| window.visible_dialog().as_ref() == Some(&editor));
    assert_eq!(buffer_text(&text.buffer()), "not valid routing syntax");
    let custom = "# Keep my comment\ndomain(example.com) -> direct\nfallback: proxy";
    text.buffer().set_text(custom);
    themes(window, "routing-advanced");
    click(&editor, "Save");
    wait_for(|| window.visible_dialog().is_none());
    wait_for(|| button(&page, "Add Routing").is_sensitive());
    assert_eq!(
        runtime.block_on(api.routings()).unwrap().routings[0]
            .routing
            .string
            .trim(),
        custom
    );
    click_icon(&row(&page, "route-test"), "document-edit-symbolic");
    let editor = current_editor(window);
    wait_for(|| find_widget::<gtk::TextView>(&editor, &|t| t.is_mapped()).is_some());
    assert!(!button(&editor, "Reset").is_sensitive());
    click(&editor, "Cancel");
    wait_for(|| window.visible_dialog().is_none());

    let actions = menu(&page, "route-test");
    click(&actions, "Rename");
    let rename = alert(window, "Rename Routing");
    find_widget::<gtk::Entry>(&rename, &|_| true)
        .unwrap()
        .set_text("route-renamed");
    click(&rename, "Rename");
    wait_for(|| window.visible_dialog().is_none());
    wait_row(&page, "route-renamed");
    let actions = menu(&page, "route-renamed");
    click(&actions, "Create Copy");
    wait_row(&page, "route-renamed (Copy)");
    let catalog = runtime.block_on(api.routings()).unwrap();
    let copy = catalog
        .routings
        .iter()
        .find(|r| r.name == "route-renamed (Copy)")
        .unwrap();
    assert_eq!(copy.routing.string.trim(), custom);
    assert!(!copy.selected);
    themes(window, "routing-list");
    let actions = menu(&page, "route-renamed (Copy)");
    click(&actions, "Delete");
    let deletion = alert(window, "Delete Routing?");
    click(&deletion, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    assert_eq!(runtime.block_on(api.routings()).unwrap().routings.len(), 2);
    let actions = menu(&page, "route-renamed (Copy)");
    click(&actions, "Delete");
    let deletion = alert(window, "Delete Routing?");
    click(&deletion, "Delete");
    wait_for(|| window.visible_dialog().is_none());
    wait_for(|| {
        find_widget::<adw::ActionRow>(&page, &|r| r.title() == "route-renamed (Copy)").is_none()
    });
    assert_eq!(runtime.block_on(api.routings()).unwrap().routings.len(), 1);
    runtime.block_on(async {
        api.remove_routing(created.id).await.unwrap();
        api.remove_group(api.groups().await.unwrap()[0].id.clone())
            .await
            .unwrap();
    });
}
fn current_editor(window: &adw::ApplicationWindow) -> adw::Dialog {
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    wait_for(|| find_widget::<adw::EntryRow>(&dialog, &|r| r.title() == "Name").is_some());
    dialog
}
fn row(page: &adw::NavigationPage, name: &str) -> adw::ActionRow {
    find_widget::<adw::ActionRow>(page, &|r| r.title() == name).unwrap()
}
fn wait_row(page: &adw::NavigationPage, name: &str) {
    wait_for(|| find_widget::<adw::ActionRow>(page, &|r| r.title() == name).is_some());
}
fn button(root: &impl IsA<gtk::Widget>, label: &str) -> gtk::Button {
    find_widget::<gtk::Button>(root, &|b| {
        b.label().as_deref() == Some(label) || b.tooltip_text().as_deref() == Some(label)
    })
    .unwrap()
}
fn menu(page: &adw::NavigationPage, name: &str) -> gtk::Popover {
    let menu = find_widget::<gtk::MenuButton>(&row(page, name), &|_| true).unwrap();
    menu.popup();
    let popover = menu.popover().unwrap();
    wait_for(|| popover.is_mapped());
    popover
}
fn alert(window: &adw::ApplicationWindow, heading: &str) -> adw::AlertDialog {
    wait_for(|| {
        window
            .visible_dialog()
            .and_then(|d| d.downcast::<adw::AlertDialog>().ok())
            .is_some_and(|a| a.heading().as_deref() == Some(heading))
    });
    window.visible_dialog().unwrap().downcast().unwrap()
}
fn buffer_text(buffer: &gtk::TextBuffer) -> String {
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .to_string()
}
fn themes(window: &adw::ApplicationWindow, name: &str) {
    if std::env::var_os("WADDLE_TEST_SCREENSHOTS").is_none() {
        return;
    }
    let manager = adw::StyleManager::default();
    let original = manager.color_scheme();
    for (scheme, theme) in [
        (adw::ColorScheme::ForceLight, "light"),
        (adw::ColorScheme::ForceDark, "dark"),
    ] {
        manager.set_color_scheme(scheme);
        let changed = Instant::now();
        wait_for(|| changed.elapsed() > Duration::from_millis(300));
        capture(window, &format!("{name}-{theme}"));
        window.set_default_size(460, 640);
        wait_for(|| window.width() == 460);
        capture(window, &format!("{name}-narrow-{theme}"));
        window.set_default_size(900, 760);
        wait_for(|| window.width() == 900);
    }
    manager.set_color_scheme(original);
}
