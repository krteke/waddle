use super::*;

pub fn create_and_edit(
    runtime: &tokio::runtime::Runtime,
    window: &adw::ApplicationWindow,
    wing: &Wing,
    page: &adw::NavigationPage,
) {
    click_icon(page, "list-add-symbolic");
    let editor = editor_page(window);
    wait_for(|| {
        find_widget::<adw::EntryRow>(&editor, &|r| {
            r.tooltip_text().as_deref() == Some("Name") && r.is_sensitive()
        })
        .is_some()
    });
    let defaults = runtime.block_on(request(
        wing,
        include_str!("../../../../graphql/queries/default_config.graphql"),
        json!({}),
    ));
    let defaults = &defaults["defaultConfig"]["global"];
    let lan = find_widget::<adw::ExpanderRow>(&editor, &|r| r.title() == "LAN Interfaces")
        .expect("LAN interfaces must be selectable from the backend interface list");
    lan.set_expanded(true);
    let loopback = find_widget::<adw::ActionRow>(&lan, &|r| r.title() == "lo").unwrap();
    assert!(loopback.subtitle().unwrap().contains("127.0.0.1/8"));
    let selected = find_widget::<gtk::CheckButton>(&loopback, &|_| true).unwrap();
    assert!(!selected.is_active());
    selected.set_active(true);
    wait_for(|| {
        find_widget::<gtk::Button>(&editor, &|b| b.label().as_deref() == Some("Reset"))
            .unwrap()
            .is_sensitive()
    });
    assert_eq!(lan.subtitle(), "lo");
    click(&editor, "Reset");
    wait_for(|| !selected.is_active());
    let interval_row =
        find_widget::<adw::EntryRow>(&editor, &|row| row.title().starts_with("Check Interval"))
            .unwrap();
    let list = interval_row
        .parent()
        .unwrap()
        .downcast::<gtk::ListBox>()
        .unwrap();
    assert!(
        list.ancestor(gtk::ListBoxRow::static_type()).is_none(),
        "a scalar input must use the settings list directly, without a second list background inside a row"
    );
    assert!(
        !find_widget::<gtk::Button>(&editor, &|b| b.label().as_deref() == Some("Reset"))
            .unwrap()
            .is_sensitive(),
        "an unchanged form must not offer Reset"
    );
    let port = find_widget::<adw::SpinRow>(&editor, &|r| {
        r.tooltip_text().as_deref() == Some("Transparent Proxy Port")
    })
    .unwrap();
    assert_eq!(
        port.value() as i64,
        defaults["tproxyPort"].as_i64().unwrap()
    );
    fill(&editor, "Name", "Discarded");
    port.set_value(24567.0);
    wait_for(|| {
        find_widget::<gtk::Button>(&editor, &|b| b.label().as_deref() == Some("Reset"))
            .unwrap()
            .is_sensitive()
    });
    click(&editor, "Reset");
    wait_for(|| {
        find_widget::<adw::EntryRow>(&editor, &|r| r.tooltip_text().as_deref() == Some("Name"))
            .unwrap()
            .text()
            .is_empty()
    });
    assert_eq!(
        port.value() as i64,
        defaults["tproxyPort"].as_i64().unwrap()
    );
    required_fields_block_save(&editor, window);
    lists_support_ordering(&editor, window);
    fill(&editor, "Name", "Discarded");
    wait_for(|| !editor.can_pop());
    click_icon(&editor, "go-previous-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    assert!(editor.is_mapped());
    assert_eq!(
        find_widget::<adw::EntryRow>(&editor, &|r| r.tooltip_text().as_deref() == Some("Name"))
            .unwrap()
            .text(),
        "Discarded"
    );
    click_icon(&editor, "go-previous-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Discard");
    wait_for(|| page.is_mapped());
    let saved = runtime.block_on(request(wing, "{configs {name}}", json!({})));
    assert!(
        !saved["configs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "Discarded")
    );

    click_icon(page, "list-add-symbolic");
    let editor = editor_page(window);
    wait_for(|| {
        find_widget::<adw::EntryRow>(&editor, &|r| {
            r.tooltip_text().as_deref() == Some("Name") && r.is_sensitive()
        })
        .is_some()
    });
    fill(&editor, "Name", "Created from defaults");
    capture(window, "config-add");
    capture_themes(&editor, window, "checks");
    click(&editor, "Save");
    wait_for(|| page.is_mapped());
    wait_for(|| {
        find_widget::<adw::ActionRow>(page, &|r| r.title() == "Created from defaults").is_some()
    });
    let saved = runtime.block_on(request(
        wing,
        include_str!("../../../../graphql/queries/configs.graphql"),
        json!({}),
    ));
    let created = saved["configs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Created from defaults")
        .unwrap();
    assert_eq!(
        &created["global"], defaults,
        "adding without changing settings must preserve all backend defaults"
    );
    let id = created["id"].as_str().unwrap();
    let row =
        find_widget::<adw::ActionRow>(page, &|r| r.title() == "Created from defaults").unwrap();
    click_icon(&row, "document-edit-symbolic");
    let editor = editor_page(window);
    assert_eq!(
        find_widget::<adw::EntryRow>(&editor, &|r| r.tooltip_text().as_deref() == Some("Name"))
            .unwrap()
            .text(),
        "Created from defaults"
    );
    assert!(
        !find_widget::<gtk::Button>(&editor, &|b| b.label().as_deref() == Some("Save"))
            .unwrap()
            .is_sensitive(),
        "unchanged configuration must not offer Save"
    );
    fill(&editor, "Name", "Edited config");
    find_widget::<adw::SpinRow>(&editor, &|r| {
        r.tooltip_text().as_deref() == Some("Transparent Proxy Port")
    })
    .unwrap()
    .set_value(24567.0);
    let protect = !defaults["tproxyPortProtect"].as_bool().unwrap();
    find_widget::<adw::SwitchRow>(&editor, &|r| {
        r.title() == "Transparent Proxy Port Protection"
    })
    .unwrap()
    .set_active(protect);
    fill(&editor, "Check Interval", "not-a-duration");
    wait_for(|| {
        find_widget::<gtk::Button>(&editor, &|b| {
            b.label().as_deref() == Some("Save") && b.is_sensitive()
        })
        .is_some()
    });
    click(&editor, "Save");
    wait_for(|| {
        window
            .visible_dialog()
            .is_some_and(|d| d.is::<adw::AlertDialog>())
    });
    let error = window.visible_dialog().unwrap();
    let error_text = find_widget::<gtk::Label>(
        &error
            .clone()
            .downcast::<adw::AlertDialog>()
            .unwrap()
            .extra_child()
            .unwrap(),
        &|_| true,
    )
    .unwrap()
    .text();
    assert!(error_text.contains("not-a-duration"), "{error_text}");
    click(&error, "Close");
    wait_for(|| window.visible_dialog().is_none());
    assert!(editor.is_mapped());
    assert_eq!(
        find_widget::<adw::EntryRow>(&editor, &|r| r.tooltip_text().as_deref() == Some("Name"))
            .unwrap()
            .text(),
        "Edited config"
    );
    fill(&editor, "Check Interval", "42s");
    language(window, "zh_CN");
    wait_for(|| editor.title() == "编辑配置");
    assert_eq!(
        find_widget::<adw::EntryRow>(&editor, &|r| r.tooltip_text().as_deref() == Some("名称"))
            .unwrap()
            .text(),
        "Edited config"
    );
    window.set_default_size(460, 640);
    wait_for(|| window.width() == 460);
    capture(window, "config-edit-narrow-zh");
    window.set_default_size(900, 640);
    wait_for(|| window.width() == 900);
    language(window, "en");
    wait_for(|| editor.title() == "Edit Configuration");
    let lan = find_widget::<adw::ExpanderRow>(&editor, &|r| r.title() == "LAN Interfaces").unwrap();
    lan.set_expanded(true);
    let loopback = find_widget::<adw::ActionRow>(&lan, &|r| r.title() == "lo").unwrap();
    adw::prelude::ActionRowExt::activate(&loopback);
    let wan = find_widget::<adw::ExpanderRow>(&editor, &|r| r.title() == "WAN Interfaces").unwrap();
    wan.set_expanded(true);
    let automatic = find_widget::<adw::ActionRow>(&wan, &|r| r.title() == "auto").unwrap();
    let automatic = find_widget::<gtk::CheckButton>(&automatic, &|_| true).unwrap();
    assert_eq!(
        automatic.is_active(),
        defaults["wanInterface"]
            .as_array()
            .unwrap()
            .contains(&json!("auto"))
    );
    automatic.set_active(true);
    let loopback = find_widget::<adw::ActionRow>(&wan, &|r| r.title() == "lo").unwrap();
    adw::prelude::ActionRowExt::activate(&loopback);
    wait_for(|| wan.subtitle() == "auto, lo");
    capture_interface_themes(&lan, window);

    let tcp = input_list(&editor, "TCP Check URLs");
    let source = find_widget::<gtk::Button>(&list_entry(&tcp, 3), &|image| {
        image.icon_name().as_deref() == Some("list-drag-handle-symbolic")
    })
    .unwrap()
    .observe_controllers()
    .item(0)
    .unwrap()
    .downcast::<gtk::DragSource>()
    .unwrap();
    let provider = source
        .emit_by_name::<Option<gtk::gdk::ContentProvider>>("prepare", &[&0.0f64, &0.0f64])
        .unwrap();
    let payload = glib::BoxedValue(provider.value(gtk::ListBoxRow::static_type()).unwrap());
    let target = drop_target(&list_entry(&tcp, 2));
    assert!(target.emit_by_name::<bool>("drop", &[&payload, &0.0f64, &0.0f64]));
    assert_eq!(list_entry(&tcp, 2).text(), "2606:4700:4700::1111");
    assert_eq!(list_entry(&tcp, 3).text(), "1.1.1.1");
    let udp = input_list(&editor, "UDP Check DNS");
    assert!(
        !drop_target(&list_entry(&udp, 1))
            .emit_by_name::<bool>("drop", &[&payload, &0.0f64, &0.0f64])
    );
    capture(window, "config-edit");
    click(&editor, "Save");
    wait_for(|| page.is_mapped());
    wait_for(|| find_widget::<adw::ActionRow>(page, &|r| r.title() == "Edited config").is_some());
    let saved = runtime.block_on(request(
        wing,
        include_str!("../../../../graphql/queries/configs.graphql"),
        json!({}),
    ));
    let edited = saved["configs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap();
    let mut expected = defaults.clone();
    expected["tproxyPort"] = json!(24567);
    expected["tproxyPortProtect"] = json!(protect);
    expected["checkInterval"] = json!("42s");
    expected["lanInterface"] = json!(["lo"]);
    expected["wanInterface"] = json!(["auto", "lo"]);
    expected["tcpCheckUrl"] = json!([
        "http://cp.cloudflare.com",
        "2606:4700:4700::1111",
        "1.1.1.1"
    ]);
    assert_eq!(
        edited["global"], expected,
        "editing must retain all untouched settings"
    );
    assert_eq!(edited["name"], "Edited config");
    assert_eq!(edited["selected"], created["selected"]);
    assert_eq!(
        saved["configs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["id"] == id)
            .count(),
        1
    );

    let row = find_widget::<adw::ActionRow>(page, &|r| r.title() == "Edited config").unwrap();
    click_icon(&row, "document-edit-symbolic");
    let editor = editor_page(window);
    let wan = find_widget::<adw::ExpanderRow>(&editor, &|r| r.title() == "WAN Interfaces").unwrap();
    assert_eq!(wan.subtitle(), "auto, lo");
    wan.set_expanded(true);
    let loopback = find_widget::<adw::ActionRow>(&wan, &|r| r.title() == "lo").unwrap();
    let selected = find_widget::<gtk::CheckButton>(&loopback, &|_| true).unwrap();
    assert!(selected.is_active());
    selected.set_active(false);
    wait_for(|| wan.subtitle() == "auto");
    click(&editor, "Reset");
    wait_for(|| selected.is_active() && editor.can_pop());
    assert_eq!(wan.subtitle(), "auto, lo");
    click_icon(&editor, "go-previous-symbolic");
    wait_for(|| page.is_mapped());
}

fn capture_interface_themes(row: &adw::ExpanderRow, window: &adw::ApplicationWindow) {
    let manager = adw::StyleManager::default();
    let original = manager.color_scheme();
    for (scheme, name) in [
        (adw::ColorScheme::ForceLight, "light"),
        (adw::ColorScheme::ForceDark, "dark"),
    ] {
        manager.set_color_scheme(scheme);
        let changed = std::time::Instant::now();
        wait_for(|| changed.elapsed() >= std::time::Duration::from_millis(300));
        scroll_to(row);
        capture(window, &format!("config-interfaces-{name}"));
        window.set_default_size(460, 640);
        wait_for(|| window.width() == 460);
        let resized = std::time::Instant::now();
        wait_for(|| resized.elapsed() >= std::time::Duration::from_millis(300));
        scroll_to(row);
        capture(window, &format!("config-interfaces-narrow-{name}"));
        window.set_default_size(900, 640);
        wait_for(|| window.width() == 900);
    }
    manager.set_color_scheme(original);
}

fn drop_target(entry: &adw::EntryRow) -> gtk::DropTarget {
    let mut widget: gtk::Widget = entry.clone().upcast();
    loop {
        let controllers = widget.observe_controllers();
        for i in 0..controllers.n_items() {
            if let Ok(target) = controllers.item(i).unwrap().downcast::<gtk::DropTarget>() {
                return target;
            }
        }
        widget = widget.parent().unwrap();
    }
}

fn input_list(editor: &adw::NavigationPage, title: &str) -> adw::PreferencesGroup {
    find_widget::<adw::PreferencesGroup>(editor, &|group| group.title() == title).unwrap()
}

fn list_entry(list: &adw::PreferencesGroup, index: usize) -> adw::EntryRow {
    find_widget::<adw::EntryRow>(list, &|r| {
        r.tooltip_text()
            .is_some_and(|title| title.ends_with(&format!("· {index}")))
    })
    .unwrap()
}

fn item_menu(entry: &adw::EntryRow) -> gtk::Popover {
    let button = find_widget::<gtk::MenuButton>(entry, &|_| true).unwrap();
    button.popup();
    let popover = button.popover().unwrap();
    wait_for(|| popover.is_mapped());
    popover
}

fn required_fields_block_save(editor: &adw::NavigationPage, window: &adw::ApplicationWindow) {
    click(editor, "Save");
    let name =
        find_widget::<adw::EntryRow>(editor, &|r| r.tooltip_text().as_deref() == Some("Name"))
            .unwrap();
    wait_for(|| name.has_css_class("error"));
    assert!(
        find_widget::<gtk::Label>(&name, &|label| label.text() == "*"
            && label.has_css_class("error"))
        .is_some(),
        "required fields must be marked before submitting"
    );
    fill(editor, "Name", "Discarded");
    wait_for(|| !name.has_css_class("error"));
    let interval = find_widget::<adw::EntryRow>(editor, &|r| {
        r.tooltip_text().as_deref() == Some("Check Interval")
    })
    .unwrap();
    interval.set_text("");
    click(editor, "Save");
    wait_for(|| interval.has_css_class("error"));
    interval.set_text("30s");
    wait_for(|| !interval.has_css_class("error"));
    let tcp = input_list(editor, "TCP Check URLs");
    while let Some(entry) = find_widget::<adw::EntryRow>(&tcp, &|_| true) {
        click(&item_menu(&entry), "Remove");
    }
    click(editor, "Save");
    let required_error =
        find_widget::<gtk::Label>(&tcp, &|l| l.text() == "This field is required.").unwrap();
    wait_for(|| required_error.get_visible());
    assert!(
        required_error.is_visible(),
        "empty required list must display its error"
    );
    assert!(window.visible_dialog().is_none());
    assert!(editor.is_mapped());
    click_icon(&tcp, "list-add-symbolic");
    let entry = list_entry(&tcp, 1);
    entry.set_text("   ");
    click(editor, "Save");
    wait_for(|| entry.has_css_class("error"));
    interval.set_text("");
    wait_for(|| interval.has_css_class("error"));
    capture(window, "config-required-error");
    capture_themes(editor, window, "error");
    interval.set_text("30s");
    entry.set_text("http://cp.cloudflare.com");
    wait_for(|| !entry.has_css_class("error"));
    let udp = input_list(editor, "UDP Check DNS");
    list_entry(&udp, 1).set_text("");
    click(editor, "Save");
    wait_for(|| list_entry(&udp, 1).has_css_class("error"));
    assert!(window.visible_dialog().is_none());
    assert!(editor.is_mapped());
    click(editor, "Reset");
    wait_for(|| {
        !find_widget::<gtk::Button>(editor, &|b| b.label().as_deref() == Some("Reset"))
            .unwrap()
            .is_sensitive()
    });
    assert!(!name.has_css_class("error"));
    assert_eq!(list_entry(&tcp, 1).text(), "http://cp.cloudflare.com");
}

fn lists_support_ordering(editor: &adw::NavigationPage, window: &adw::ApplicationWindow) {
    let tcp = input_list(editor, "TCP Check URLs");
    let first = list_entry(&tcp, 1);
    click(&item_menu(&first), "Move Down");
    assert_eq!(list_entry(&tcp, 1).text(), "1.1.1.1");
    assert_eq!(list_entry(&tcp, 2).text(), "http://cp.cloudflare.com");
    wait_for(|| {
        find_widget::<gtk::Button>(editor, &|b| b.label().as_deref() == Some("Reset"))
            .unwrap()
            .is_sensitive()
    });
    let menu = item_menu(&list_entry(&tcp, 3));
    assert!(
        !find_widget::<gtk::Button>(&menu, &|b| b.label().as_deref() == Some("Move Down"))
            .unwrap()
            .is_sensitive()
    );
    menu.popdown();
    click(&item_menu(&first), "Move Up");
    wait_for(|| {
        !find_widget::<gtk::Button>(editor, &|b| b.label().as_deref() == Some("Reset"))
            .unwrap()
            .is_sensitive()
    });
    let dial = find_widget::<adw::ActionRow>(editor, &|r| r.title() == "ip").unwrap();
    assert!(!dial.subtitle().unwrap().is_empty());
    find_widget::<gtk::CheckButton>(&dial, &|_| true)
        .unwrap()
        .set_active(true);
    wait_for(|| {
        find_widget::<gtk::Button>(editor, &|b| b.label().as_deref() == Some("Reset"))
            .unwrap()
            .is_sensitive()
    });
    dial.grab_focus();
    capture(window, "config-dial-options");
    click(editor, "Reset");
    wait_for(|| {
        !find_widget::<gtk::Button>(editor, &|b| b.label().as_deref() == Some("Reset"))
            .unwrap()
            .is_sensitive()
    });
}

fn capture_themes(editor: &adw::NavigationPage, window: &adw::ApplicationWindow, state: &str) {
    let manager = adw::StyleManager::default();
    let original = manager.color_scheme();
    let checks = find_widget::<adw::PreferencesGroup>(editor, &|group| {
        group.title() == "Node Connectivity Check"
    })
    .unwrap();
    manager.set_color_scheme(adw::ColorScheme::ForceLight);
    wait_for(|| !manager.is_dark());
    let changed = std::time::Instant::now();
    wait_for(|| changed.elapsed() >= std::time::Duration::from_millis(300));
    let software =
        find_widget::<adw::PreferencesGroup>(editor, &|group| group.title() == "Software Options")
            .unwrap();
    scroll_to(&software);
    capture(window, &format!("config-{state}-numbers-light"));
    scroll_to(&checks);
    capture(window, &format!("config-{state}-light"));
    scroll_to(&input_list(editor, "TCP Check URLs"));
    capture(window, &format!("config-{state}-list-light"));
    for (name, title) in [("tcp", "TCP Check URLs"), ("udp", "UDP Check DNS")] {
        let group = input_list(editor, title);
        scroll_to(&group);
        let menu = item_menu(&list_entry(&group, 1));
        capture(&menu, &format!("config-{state}-{name}-menu-light"));
        menu.popdown();
    }
    window.set_default_size(460, 640);
    wait_for(|| window.width() == 460);
    capture(window, &format!("config-{state}-narrow-light"));
    window.set_default_size(900, 640);
    wait_for(|| window.width() == 900);
    manager.set_color_scheme(adw::ColorScheme::ForceDark);
    wait_for(|| manager.is_dark());
    let changed = std::time::Instant::now();
    wait_for(|| changed.elapsed() >= std::time::Duration::from_millis(300));
    scroll_to(&software);
    capture(window, &format!("config-{state}-numbers-dark"));
    scroll_to(&checks);
    capture(window, &format!("config-{state}-dark"));
    scroll_to(&input_list(editor, "TCP Check URLs"));
    capture(window, &format!("config-{state}-list-dark"));
    for (name, title) in [("tcp", "TCP Check URLs"), ("udp", "UDP Check DNS")] {
        let group = input_list(editor, title);
        scroll_to(&group);
        let menu = item_menu(&list_entry(&group, 1));
        capture(&menu, &format!("config-{state}-{name}-menu-dark"));
        menu.popdown();
    }
    window.set_default_size(460, 640);
    wait_for(|| window.width() == 460);
    capture(window, &format!("config-{state}-narrow-dark"));
    window.set_default_size(900, 640);
    wait_for(|| window.width() == 900);
    manager.set_color_scheme(original);
}

fn scroll_to(widget: &impl IsA<gtk::Widget>) {
    let scroll = widget
        .as_ref()
        .ancestor(gtk::ScrolledWindow::static_type())
        .unwrap()
        .downcast::<gtk::ScrolledWindow>()
        .unwrap();
    let bounds = widget
        .as_ref()
        .compute_bounds(&scroll.child().unwrap())
        .unwrap();
    let adjustment = scroll.vadjustment();
    adjustment.set_value(adjustment.value() + bounds.y() as f64);
}

pub(super) fn editor_page(window: &adw::ApplicationWindow) -> adw::NavigationPage {
    wait_for(|| {
        find_widget::<adw::NavigationPage>(window, &|p| {
            p.tag().as_deref() == Some("config-editor")
                && p.is_mapped()
                && find_widget::<adw::PreferencesPage>(p, &|page| {
                    page.is_visible() && page.is_sensitive()
                })
                .is_some()
        })
        .is_some()
    });
    assert!(
        window.visible_dialog().is_none(),
        "Config editor must not open a dialog"
    );
    find_widget::<adw::NavigationPage>(window, &|p| {
        p.tag().as_deref() == Some("config-editor") && p.is_mapped()
    })
    .unwrap()
}
