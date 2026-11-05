use super::*;

pub fn manage(runtime: &tokio::runtime::Runtime, window: &adw::ApplicationWindow, wing: &Wing) {
    let api = runtime.block_on(async {
        let saved = credentials::load("Home").await.unwrap().unwrap();
        Api::new(format!("http://{}", wing.address).parse().unwrap())
            .with_token(saved.jwt.as_deref().unwrap())
            .unwrap()
    });
    navigation_row(window, "DNS").activate();
    wait_for(|| {
        find_widget::<adw::NavigationPage>(window, &|p| {
            p.is_mapped() && p.tag().as_deref() == Some("dns-list")
        })
        .is_some()
    });
    let page =
        find_widget::<adw::NavigationPage>(window, &|p| p.tag().as_deref() == Some("dns-list"))
            .unwrap();
    wait_for(|| {
        find_widget::<adw::StatusPage>(&page, &|p| {
            p.is_visible() && p.title() == "No DNS Configurations"
        })
        .is_some()
    });
    click_icon(&page, "list-add-symbolic");
    let editor = current_editor(window);
    assert!(!button(&editor, "Reset").is_sensitive());
    click(&editor, "Save");
    let name = find_widget::<adw::EntryRow>(&editor, &|r| r.title() == "Name").unwrap();
    wait_for(|| name.has_css_class("error"));
    name.set_text("dns-test");
    // Add/remove must discard both the controls and their data, including empty fields.
    let add = find_widget::<gtk::Button>(&editor, &|b| {
        b.tooltip_text().as_deref() == Some("Add Upstream")
    })
    .unwrap();
    add.emit_clicked();
    wait_for(|| find_widget::<adw::ExpanderRow>(&editor, &|r| r.title() == "Upstream").is_some());
    let extra = find_widget::<adw::ExpanderRow>(&editor, &|r| r.title() == "Upstream").unwrap();
    click_icon(&extra, "user-trash-symbolic");
    wait_for(|| extra.parent().is_none());
    themes(window, "dns-form");
    language(window, "zh_CN");
    wait_for(|| editor.title() == "添加 DNS");
    themes(window, "dns-form-zh");
    language(window, "en");
    wait_for(|| editor.title() == "Add DNS");
    click(&editor, "Save");
    wait_row(&page, "dns-test");
    let created = runtime.block_on(api.dnss()).unwrap().remove(0);
    assert_eq!(created.dns.upstream.len(), 2);
    assert_eq!(created.dns.routing.request.rules.len(), 1);
    adw::prelude::ActionRowExt::activate(&row(&page, "dns-test"));
    wait_for(|| {
        find_widget::<gtk::CheckButton>(&row(&page, "dns-test"), &|c| c.is_active()).is_some()
    });
    let actions = menu(&page, "dns-test");
    assert!(!button(&actions, "Delete").is_sensitive());
    click(&actions, "View Details");
    wait_for(|| window.visible_dialog().is_some());
    let detail = window.visible_dialog().unwrap();
    let code = find_widget::<gtk::TextView>(&detail, &|_| true).unwrap();
    assert!(!code.is_editable());
    assert_eq!(buffer_text(&code.buffer()), created.dns.string);
    detail.force_close();
    wait_for(|| window.visible_dialog().is_none());
    click_icon(&row(&page, "dns-test"), "document-edit-symbolic");
    let editor = current_editor(window);
    let mode = find_widget::<adw::ToggleGroup>(&editor, &|_| true).unwrap();
    assert_eq!(
        mode.active(),
        0,
        "saved form configurations must reopen as a form"
    );
    let name = find_widget::<adw::EntryRow>(&editor, &|r| r.title() == "Name").unwrap();
    assert!(name.is_editable());
    name.set_text("dns-renamed");
    wait_for(|| button(&editor, "Reset").is_sensitive());
    click(&editor, "Reset");
    wait_for(|| !button(&editor, "Reset").is_sensitive());
    assert_eq!(name.text(), "dns-test");
    name.set_text("dns-renamed");
    wait_for(|| button(&editor, "Save").is_sensitive());
    click(&editor, "Save");
    wait_row(&page, "dns-renamed");
    let renamed = runtime.block_on(api.dnss()).unwrap().remove(0);
    assert_eq!(renamed.id, created.id);
    assert!(renamed.selected);
    assert_eq!(renamed.dns.string, created.dns.string);

    click_icon(&row(&page, "dns-renamed"), "document-edit-symbolic");
    let editor = current_editor(window);
    let request =
        find_widget::<adw::PreferencesGroup>(&editor, &|g| g.title() == "Request Rules").unwrap();
    let add_rule = find_widget::<gtk::Button>(&request, &|b| {
        b.tooltip_text().as_deref() == Some("Add Rule")
    })
    .unwrap();
    add_rule.emit_clicked();
    wait_for(|| find_widget::<adw::ExpanderRow>(&request, &|r| r.title() == "Rule").is_some());
    let extra = find_widget::<adw::ExpanderRow>(&request, &|r| r.title() == "Rule").unwrap();
    click(&editor, "Save");
    let matcher = find_widget::<adw::EntryRow>(&extra, &|r| r.title() == "Condition").unwrap();
    wait_for(|| matcher.has_css_class("error"));
    let scroll = find_widget::<gtk::ScrolledWindow>(&editor, &|s| s.is_mapped()).unwrap();
    let adjustment = scroll.vadjustment();
    adjustment.set_value(adjustment.upper() - adjustment.page_size());
    themes(window, "dns-required");
    matcher.set_text("qtype(AAAA)");
    fill(&extra, "Target", "reject");
    let more = find_widget::<gtk::MenuButton>(&extra, &|_| true).unwrap();
    more.popup();
    let actions = more.popover().unwrap();
    click(&actions, "Move Up");
    let response =
        find_widget::<adw::PreferencesGroup>(&editor, &|g| g.title() == "Response Rules").unwrap();
    let add_response = find_widget::<gtk::Button>(&response, &|b| {
        b.tooltip_text().as_deref() == Some("Add Rule")
    })
    .unwrap();
    add_response.emit_clicked();
    wait_for(|| find_widget::<adw::ExpanderRow>(&response, &|r| r.title() == "Rule").is_some());
    let response_rule =
        find_widget::<adw::ExpanderRow>(&response, &|r| r.title() == "Rule").unwrap();
    fill(&response_rule, "Condition", "upstream(googledns)");
    fill(&response_rule, "Target", "accept");
    wait_for(|| response_rule.title() == "upstream(googledns)");
    add_response.emit_clicked();
    wait_for(|| find_widget::<adw::ExpanderRow>(&response, &|r| r.title() == "Rule").is_some());
    let discarded = find_widget::<adw::ExpanderRow>(&response, &|r| r.title() == "Rule").unwrap();
    let discarded_menu = find_widget::<gtk::MenuButton>(&discarded, &|_| true).unwrap();
    discarded_menu.popup();
    click(&discarded_menu.popover().unwrap(), "Delete");
    wait_for(|| discarded.parent().is_none());
    let mode = find_widget::<adw::ToggleGroup>(&editor, &|_| true).unwrap();
    mode.set_active(1);
    let code = find_widget::<gtk::TextView>(&editor, &|_| true).unwrap();
    wait_for(|| code.is_mapped());
    let text = buffer_text(&code.buffer());
    assert!(text.find("qtype(AAAA)").unwrap() < text.find("qname(geosite:cn)").unwrap());
    mode.set_active(0);
    wait_for(|| {
        find_widget::<adw::PreferencesGroup>(&editor, &|g| {
            g.is_mapped() && g.title() == "Request Rules"
        })
        .is_some()
    });
    click(&editor, "Save");
    wait_row(&page, "dns-renamed");
    wait_for(|| {
        find_widget::<adw::NavigationPage>(window, &|p| {
            p.is_mapped() && p.tag().as_deref() == Some("dns-editor")
        })
        .is_none()
    });
    let updated = runtime.block_on(api.dnss()).unwrap().remove(0);
    assert_eq!(updated.dns.routing.request.rules.len(), 2);
    assert_eq!(updated.dns.routing.response.rules.len(), 1);
    assert_eq!(
        updated.dns.routing.response.rules[0].outbound.name,
        "accept"
    );
    assert_eq!(
        updated.dns.routing.request.rules[0].conditions.and[0].name,
        "qtype"
    );

    click_icon(&row(&page, "dns-renamed"), "document-edit-symbolic");
    let editor = current_editor(window);
    let mode = find_widget::<adw::ToggleGroup>(&editor, &|_| true).unwrap();
    mode.set_active(1);
    let code = find_widget::<gtk::TextView>(&editor, &|_| true).unwrap();
    wait_for(|| code.is_mapped());
    code.buffer().set_text("not valid dns syntax");
    wait_for(|| button(&editor, "Save").is_sensitive());
    click(&editor, "Save");
    let error = alert(window, "Could not save DNS configuration.");
    assert!(
        find_widget::<gtk::Label>(&error, &|l| l.is_selectable() && !l.text().is_empty()).is_some()
    );
    click(&error, "Close");
    wait_for(|| window.visible_dialog().is_none());
    assert_eq!(buffer_text(&code.buffer()), "not valid dns syntax");
    let custom = format!(
        "# Keep this comment\nipversion_prefer: 4\n{}",
        updated.dns.string
    );
    code.buffer().set_text(&custom);
    mode.set_active(0);
    wait_for(|| find_widget::<adw::Banner>(&editor, &|b| b.is_revealed()).is_some());
    assert!(code.is_mapped());
    assert_eq!(mode.active(), 1);
    assert_eq!(buffer_text(&code.buffer()), custom);
    themes(window, "dns-advanced");
    click(&editor, "Save");
    wait_for(|| !editor.is_mapped());
    wait_row(&page, "dns-renamed");
    click_icon(&row(&page, "dns-renamed"), "document-edit-symbolic");
    let editor = current_editor(window);
    let mode = find_widget::<adw::ToggleGroup>(&editor, &|_| true).unwrap();
    assert_eq!(mode.active(), 1);
    let code = find_widget::<gtk::TextView>(&editor, &|_| true).unwrap();
    code.buffer().set_text("invalid");
    wait_for(|| button(&editor, "Reset").is_sensitive());
    click(&editor, "Reset");
    wait_for(|| !button(&editor, "Reset").is_sensitive());
    assert_eq!(mode.active(), 1);
    assert_eq!(buffer_text(&code.buffer()), custom);
    click_icon(&editor, "go-previous-symbolic");
    wait_for(|| !editor.is_mapped());

    let actions = menu(&page, "dns-renamed");
    click(&actions, "Create Copy");
    wait_row(&page, "dns-renamed (Copy)");
    let copy = runtime
        .block_on(api.dnss())
        .unwrap()
        .into_iter()
        .find(|d| !d.selected)
        .unwrap();
    assert_eq!(copy.dns.string, custom);
    themes(window, "dns-list");
    let actions = menu(&page, "dns-renamed (Copy)");
    click(&actions, "Rename");
    let rename = alert(window, "Rename DNS");
    find_widget::<gtk::Entry>(&rename, &|_| true)
        .unwrap()
        .set_text("dns-copy");
    click(&rename, "Rename");
    wait_row(&page, "dns-copy");
    let actions = menu(&page, "dns-copy");
    click(&actions, "Delete");
    let delete = alert(window, "Delete DNS Configuration?");
    click(&delete, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    assert_eq!(runtime.block_on(api.dnss()).unwrap().len(), 2);
    let actions = menu(&page, "dns-copy");
    click(&actions, "Delete");
    let delete = alert(window, "Delete DNS Configuration?");
    click(&delete, "Delete");
    wait_for(|| find_widget::<adw::ActionRow>(&page, &|r| r.title() == "dns-copy").is_none());
    runtime.block_on(api.remove_dns(created.id)).unwrap();
    click_icon(&page, "view-refresh-symbolic");
    wait_for(|| find_widget::<adw::ActionRow>(&page, &|r| r.title() == "dns-renamed").is_none());
}

fn current_editor(window: &adw::ApplicationWindow) -> adw::NavigationPage {
    wait_for(|| {
        find_widget::<adw::NavigationPage>(window, &|p| {
            p.tag().as_deref() == Some("dns-editor") && p.is_mapped()
        })
        .is_some()
    });
    find_widget::<adw::NavigationPage>(window, &|p| {
        p.tag().as_deref() == Some("dns-editor") && p.is_mapped()
    })
    .unwrap()
}
fn row(page: &adw::NavigationPage, name: &str) -> adw::ActionRow {
    find_widget::<adw::ActionRow>(page, &|r| r.title() == name).unwrap()
}
fn wait_row(page: &adw::NavigationPage, name: &str) {
    wait_for(|| {
        find_widget::<adw::ActionRow>(page, &|r| {
            r.is_mapped() && r.is_sensitive() && r.title() == name
        })
        .is_some()
    });
}
fn button(root: &impl IsA<gtk::Widget>, label: &str) -> gtk::Button {
    find_widget::<gtk::Button>(root, &|b| b.label().as_deref() == Some(label)).unwrap()
}
fn menu(page: &adw::NavigationPage, name: &str) -> gtk::Popover {
    let button = find_widget::<gtk::MenuButton>(&row(page, name), &|_| true).unwrap();
    button.popup();
    let popover = button.popover().unwrap();
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
        capture(window, &format!("{name}-{theme}"));
        window.set_default_size(460, 760);
        wait_for(|| window.width() == 460);
        capture(window, &format!("{name}-narrow-{theme}"));
        window.set_default_size(900, 760);
        wait_for(|| window.width() == 900);
    }
    manager.set_color_scheme(original);
}
