use super::*;
use crate::api::node::Node;

const TROJAN: &str = "trojan://test-password@hk.example.com:443#Original%20Name";

pub(super) fn add_and_list(
    runtime: &tokio::runtime::Runtime,
    window: &adw::ApplicationWindow,
    wing: &Wing,
) {
    find_widget::<gtk::ListBoxRow>(window, &|row| {
        find_widget::<gtk::Label>(row, &|label| label.text() == "Nodes").is_some()
    })
    .unwrap()
    .activate();
    wait_for(|| {
        find_widget::<adw::StatusPage>(window, &|page| {
            page.is_mapped() && page.title() == "No Nodes"
        })
        .is_some()
    });
    let dialog = open_add(window);
    grows_until_scroll_limit(window, &dialog);
    height_animation_can_reverse(window, &dialog);
    required_and_reset(window, &dialog);
    fill(&dialog, "Link", TROJAN);
    fill(&dialog, "Tag", "Hong Kong");
    capture_themes(window, "nodes-add");
    click(&dialog, "Add");
    wait_for(|| window.visible_dialog().is_none());
    wait_for(|| {
        find_widget::<adw::ActionRow>(window, &|row| {
            row.title() == "Hong Kong"
                && row.subtitle().as_deref() == Some("TROJAN · Original Name")
        })
        .is_some()
    });
    let row = find_widget::<adw::ActionRow>(window, &|row| row.title() == "Hong Kong").unwrap();
    assert!(
        find_widget::<gtk::Button>(&row, &|button| {
            button.tooltip_text().as_deref() == Some("Show QR Code and Link")
        })
        .unwrap()
        .is_sensitive()
    );
    let records = records(runtime, wing);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].tag.as_deref(), Some("Hong Kong"));
    assert_eq!(records[0].name, "Original Name");
    partial_import_keeps_failed_entries(runtime, window, wing);
    duplicate_is_not_reported_as_success(runtime, window, wing);
    language(window, "zh_CN");
    wait_for(|| {
        find_widget::<gtk::Button>(window, &|b| {
            b.is_mapped() && b.tooltip_text().as_deref() == Some("添加节点")
        })
        .is_some()
    });
    capture_themes(window, "nodes-list-zh");
    language(window, "en");
    wait_for(|| {
        find_widget::<gtk::Button>(window, &|b| {
            b.is_mapped() && b.tooltip_text().as_deref() == Some("Add Nodes")
        })
        .is_some()
    });
    sign_out_restores_placeholder(window);
}

fn sign_out_restores_placeholder(window: &adw::ApplicationWindow) {
    find_widget::<gtk::ListBoxRow>(window, &|row| {
        find_widget::<gtk::Label>(row, &|label| label.text() == "Overview").is_some()
    })
    .unwrap()
    .activate();
    click_icon(window, "system-log-out-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    find_widget::<gtk::ListBoxRow>(window, &|row| {
        find_widget::<gtk::Label>(row, &|label| label.text() == "Nodes").is_some()
    })
    .unwrap()
    .activate();
    wait_for(|| {
        find_widget::<adw::StatusPage>(window, &|page| page.is_mapped() && page.title() == "Nodes")
            .is_some()
    });
    assert!(find_widget::<adw::ActionRow>(window, &|row| row.title() == "Hong Kong").is_none());
    click(window, "Sign In");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Sign In");
    wait_for(|| {
        find_widget::<adw::ActionRow>(window, &|row| row.is_mapped() && row.title() == "Hong Kong")
            .is_some()
    });
}

fn open_add(window: &adw::ApplicationWindow) -> adw::Dialog {
    let matches = |button: &gtk::Button| {
        button.is_mapped()
            && button.is_sensitive()
            && button.tooltip_text().as_deref() == Some("Add Nodes")
    };
    wait_for(|| find_widget::<gtk::Button>(window, &matches).is_some());
    find_widget::<gtk::Button>(window, &matches)
        .unwrap()
        .emit_clicked();
    wait_for(|| window.visible_dialog().is_some());
    window.visible_dialog().unwrap()
}

fn grows_until_scroll_limit(window: &adw::ApplicationWindow, dialog: &adw::Dialog) {
    let content = dialog.child().unwrap();
    wait_for(|| content.height() > 0);
    let initial_height = content.height();
    assert!(
        initial_height < 400,
        "one node should open a compact dialog, got {initial_height}"
    );
    capture_themes(window, "nodes-height-one");
    click_icon(dialog, "list-add-symbolic");
    wait_for(|| content.height() > initial_height);
    let two_height = content.height();
    capture_themes(window, "nodes-height-two");
    for number in 3..=6 {
        click_icon(dialog, "list-add-symbolic");
        wait_for(|| {
            find_widget::<adw::PreferencesGroup>(dialog, &|group| {
                group.title() == format!("Node {number}")
            })
            .is_some()
        });
    }
    let scroll = find_widget::<gtk::ScrolledWindow>(dialog, &|_| true).unwrap();
    wait_for(|| scroll.vadjustment().upper() > scroll.vadjustment().page_size());
    assert!(content.height() >= two_height);
    assert!(
        content.height() <= 600,
        "many nodes must not grow the dialog without a limit"
    );
    let capped_height = content.height();
    click_icon(dialog, "list-add-symbolic");
    wait_for(|| find_widget::<adw::PreferencesGroup>(dialog, &|g| g.title() == "Node 7").is_some());
    capture_themes(window, "nodes-height-capped");
    assert_eq!(
        content.height(),
        capped_height,
        "extra nodes must scroll, not enlarge the capped dialog"
    );
    click(dialog, "Reset");
    wait_for(|| content.height() == initial_height);
}

fn height_animation_can_reverse(window: &adw::ApplicationWindow, dialog: &adw::Dialog) {
    let settings = gtk::Settings::default().unwrap();
    let manager = adw::StyleManager::default();
    let original_scheme = manager.color_scheme();
    let content = dialog.child().unwrap();
    let initial = content.height();
    for (scheme, name) in [
        (adw::ColorScheme::ForceLight, "light"),
        (adw::ColorScheme::ForceDark, "dark"),
    ] {
        manager.set_color_scheme(scheme);
        wait_for(|| manager.is_dark() == (scheme == adw::ColorScheme::ForceDark));
        settings.set_gtk_enable_animations(true);
        click_icon(dialog, "list-add-symbolic");
        wait_for(|| {
            find_widget::<adw::PreferencesGroup>(dialog, &|g| g.title() == "Node 2").is_some()
        });
        let expanded = content
            .measure(gtk::Orientation::Vertical, content.width())
            .1;
        assert!(expanded > initial);
        wait_for(|| content.height() > initial && content.height() < expanded);
        // Reverse an in-progress transition; it must finish at the remaining content's height.
        click(dialog, "Reset");
        wait_for(|| content.height() == initial);
        click_icon(dialog, "list-add-symbolic");
        wait_for(|| content.height() > initial && content.height() < expanded);
        capture(window, &format!("nodes-growing-{name}"));
        wait_for(|| content.height() == expanded);
        click(dialog, "Reset");
        wait_for(|| content.height() > initial && content.height() < expanded);
        capture(window, &format!("nodes-shrinking-{name}"));
        wait_for(|| content.height() == initial);
        settings.set_gtk_enable_animations(false);
    }
    manager.set_color_scheme(original_scheme);
}

fn required_and_reset(window: &adw::ApplicationWindow, dialog: &adw::Dialog) {
    let reset =
        find_widget::<gtk::Button>(dialog, &|b| b.label().as_deref() == Some("Reset")).unwrap();
    assert!(!reset.is_sensitive());
    click(dialog, "Add");
    wait_for(|| {
        find_widget::<adw::EntryRow>(dialog, &|row| {
            row.title() == "Link" && row.has_css_class("error")
        })
        .is_some()
    });
    assert!(
        find_widget::<adw::EntryRow>(dialog, &|row| {
            row.title() == "Tag" && row.has_css_class("error")
        })
        .is_some()
    );
    capture_themes(window, "nodes-required");
    fill(dialog, "Link", "temporary");
    click_icon(dialog, "list-add-symbolic");
    wait_for(|| find_widget::<adw::PreferencesGroup>(dialog, &|g| g.title() == "Node 2").is_some());
    wait_for(|| reset.is_sensitive());
    click(dialog, "Reset");
    wait_for(|| !reset.is_sensitive());
    assert!(find_widget::<adw::PreferencesGroup>(dialog, &|g| g.title() == "Node 2").is_none());
    let link = find_widget::<adw::EntryRow>(dialog, &|r| r.title() == "Link").unwrap();
    assert!(link.text().is_empty());
    assert!(!link.has_css_class("error"));
    // Removing all rows must not allow an empty import.
    click_icon(dialog, "list-remove-symbolic");
    wait_for(|| {
        !find_widget::<gtk::Button>(dialog, &|b| b.label().as_deref() == Some("Add"))
            .unwrap()
            .is_sensitive()
    });
    click_icon(dialog, "list-add-symbolic");
    wait_for(|| find_widget::<adw::EntryRow>(dialog, &|r| r.title() == "Link").is_some());
}

fn partial_import_keeps_failed_entries(
    runtime: &tokio::runtime::Runtime,
    window: &adw::ApplicationWindow,
    wing: &Wing,
) {
    let dialog = open_add(window);
    fill(&dialog, "Link", "socks5://127.0.0.1:1080#Local%20proxy");
    fill(&dialog, "Tag", "Local");
    click_icon(&dialog, "list-add-symbolic");
    wait_for(|| {
        find_widget::<adw::PreferencesGroup>(&dialog, &|g| g.title() == "Node 2").is_some()
    });
    let second = find_widget::<adw::PreferencesGroup>(&dialog, &|g| g.title() == "Node 2").unwrap();
    find_widget::<adw::EntryRow>(&second, &|r| r.title() == "Link")
        .unwrap()
        .set_text("invalid-link");
    find_widget::<adw::EntryRow>(&second, &|r| r.title() == "Tag")
        .unwrap()
        .set_text("Retry");
    click(&dialog, "Add");
    wait_for(|| {
        find_widget::<adw::ActionRow>(&second, &|r| {
            r.is_visible() && r.has_css_class("error") && !r.title().is_empty()
        })
        .is_some()
    });
    assert_eq!(
        records(runtime, wing).len(),
        2,
        "valid entries must survive a partial failure"
    );
    assert!(
        find_widget::<adw::EntryRow>(&dialog, &|r| r.text().starts_with("socks5:")).is_none(),
        "successfully imported entries must not be resubmitted"
    );
    capture_themes(window, "nodes-partial-error");
    fill(
        &dialog,
        "Link",
        "vless://00000000-0000-0000-0000-000000000001@v6.example.com:443?security=tls#v6-2",
    );
    fill(&dialog, "Tag", "IPv6");
    language(window, "zh_CN");
    wait_for(|| dialog.title() == "添加节点");
    assert!(
        find_widget::<adw::EntryRow>(&dialog, &|r| r.title() == "标签" && r.text() == "IPv6")
            .is_some()
    );
    language(window, "en");
    wait_for(|| dialog.title() == "Add Nodes");
    click(&dialog, "Add");
    wait_for(|| window.visible_dialog().is_none());
    wait_for(|| {
        find_widget::<adw::ActionRow>(window, &|r| {
            r.title() == "IPv6" && r.subtitle().as_deref() == Some("VLESS · v6-2")
        })
        .is_some()
    });
    assert_eq!(records(runtime, wing).len(), 3);
}

fn duplicate_is_not_reported_as_success(
    runtime: &tokio::runtime::Runtime,
    window: &adw::ApplicationWindow,
    wing: &Wing,
) {
    let dialog = open_add(window);
    fill(&dialog, "Link", TROJAN);
    fill(&dialog, "Tag", "Duplicate");
    click(&dialog, "Add");
    wait_for(|| {
        find_widget::<adw::ActionRow>(&dialog, &|r| {
            r.is_visible() && r.title() == "node already exists"
        })
        .is_some()
    });
    assert_eq!(records(runtime, wing).len(), 3);
    click(&dialog, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
}

fn records(runtime: &tokio::runtime::Runtime, wing: &Wing) -> Vec<Node> {
    runtime.block_on(async {
        let saved = credentials::load("Home").await.unwrap().unwrap();
        Api::new(format!("http://{}", wing.address).parse().unwrap())
            .with_token(saved.jwt.as_deref().unwrap())
            .unwrap()
            .nodes()
            .await
            .unwrap()
    })
}

fn capture_themes(window: &adw::ApplicationWindow, name: &str) {
    if std::env::var_os("WADDLE_TEST_SCREENSHOTS").is_none() {
        return;
    }
    let manager = adw::StyleManager::default();
    let original = manager.color_scheme();
    let split = find_widget::<adw::OverlaySplitView>(window, &|_| true).unwrap();
    let sidebar = split.shows_sidebar();
    for (theme, dark) in [("light", false), ("dark", true)] {
        manager.set_color_scheme(if dark {
            adw::ColorScheme::ForceDark
        } else {
            adw::ColorScheme::ForceLight
        });
        wait_for(|| manager.is_dark() == dark);
        let changed = Instant::now();
        wait_for(|| changed.elapsed() >= Duration::from_millis(300));
        capture(window, &format!("{name}-{theme}"));
        window.set_default_size(460, 640);
        split.set_show_sidebar(false);
        wait_for(|| window.width() == 460);
        capture(window, &format!("{name}-narrow-{theme}"));
        window.set_default_size(900, 640);
        wait_for(|| window.width() == 900);
        split.set_show_sidebar(sidebar);
    }
    manager.set_color_scheme(original);
}
