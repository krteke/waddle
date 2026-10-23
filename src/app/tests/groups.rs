use super::*;
use crate::api::{group::Policy, node::ImportArgument};

pub fn manage_groups(
    runtime: &tokio::runtime::Runtime,
    window: &adw::ApplicationWindow,
    wing: &Wing,
) {
    let api = runtime.block_on(async {
        let saved = credentials::load("Home").await.unwrap().unwrap();
        Api::new(format!("http://{}", wing.address).parse().unwrap())
            .with_token(saved.jwt.as_deref().unwrap())
            .unwrap()
    });
    let imported = runtime
        .block_on(api.import_nodes(vec![
            ImportArgument {
                link: "trojan://password@jp.group.test:443#Tokyo%20Original".into(),
                tag: Some("Tokyo".into()),
            },
            ImportArgument {
                link: "trojan://password@hk.group.test:443#Hong%20Kong%20Original".into(),
                tag: Some("Hong Kong".into()),
            },
        ]))
        .unwrap();
    navigation_row(window, "Groups").activate();
    wait_for(|| {
        find_widget::<adw::NavigationPage>(window, &|p| {
            p.tag().as_deref() == Some("group-list") && p.is_mapped()
        })
        .is_some()
    });
    let page =
        find_widget::<adw::NavigationPage>(window, &|p| p.tag().as_deref() == Some("group-list"))
            .unwrap();
    wait_for(|| {
        find_widget::<gtk::Button>(&page, &|b| {
            b.tooltip_text().as_deref() == Some("Add Group") && b.is_sensitive()
        })
        .is_some()
    });
    create(window, &page, "reviewA", 4);
    let groups = runtime.block_on(api.groups()).unwrap();
    let original = groups.iter().find(|g| g.name == "reviewA").unwrap();
    assert_eq!(original.policy, Policy::Fixed);
    assert_eq!(original.policy_params[0].val, "0");
    create(window, &page, "reviewB", 3);
    duplicate_name_preserves_form(window, &page);

    let row = group_row(&page, "reviewA");
    adw::prelude::ActionRowExt::activate(&row);
    let editor = editor_page(window);
    fill(&editor, "Name", "reviewC");
    find_widget::<adw::ComboRow>(&editor, &|_| true)
        .unwrap()
        .set_selected(3);
    let picker = open_picker(window, &editor);
    let search = find_widget::<gtk::SearchEntry>(&picker, &|_| true).unwrap();
    search.set_text("hong kong");
    wait_for(|| {
        find_widget::<adw::ActionRow>(&picker, &|r| r.title() == "Tokyo" && !r.is_visible())
            .is_some()
    });
    let hong_kong = find_widget::<adw::ActionRow>(&picker, &|r| r.title() == "Hong Kong").unwrap();
    adw::prelude::ActionRowExt::activate(&hong_kong);
    search.set_text("tokyo original");
    wait_for(|| {
        find_widget::<adw::ActionRow>(&picker, &|r| r.title() == "Tokyo" && r.is_visible())
            .is_some()
    });
    let tokyo = find_widget::<adw::ActionRow>(&picker, &|r| r.title() == "Tokyo").unwrap();
    adw::prelude::ActionRowExt::activate(&tokyo);
    capture(window, "groups-picker-search");
    click(&picker, "Add Selected");
    wait_for(|| window.visible_dialog().is_none());
    wait_for(|| find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Hong Kong").is_some());
    let server_groups = runtime.block_on(api.groups()).unwrap();
    assert!(
        server_groups
            .iter()
            .find(|g| g.id == original.id)
            .unwrap()
            .nodes
            .is_empty(),
        "node membership must remain a draft until Save"
    );
    let picker = open_picker(window, &editor);
    assert!(
        find_widget::<adw::ActionRow>(&picker, &|r| r.title() == "Hong Kong"
            || r.title() == "Tokyo")
        .is_none(),
        "existing draft members must be excluded from the picker"
    );
    click(&picker, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    click_icon(
        &find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Tokyo").unwrap(),
        "list-remove-symbolic",
    );
    wait_for(|| find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Tokyo").is_none());
    capture_themes(window, "groups-editor");
    language(window, "zh_CN");
    wait_for(|| find_widget::<adw::ComboRow>(&editor, &|r| r.title() == "节点选择策略").is_some());
    capture(window, "groups-editor-zh");
    language(window, "en");
    wait_for(|| {
        find_widget::<gtk::Button>(&editor, &|b| {
            b.label().as_deref() == Some("Save") && b.is_sensitive()
        })
        .is_some()
    });
    click(&editor, "Save");
    wait_for(|| page.is_mapped());
    wait_for(|| find_widget::<adw::ActionRow>(&page, &|r| r.title() == "reviewC").is_some());
    let saved = runtime.block_on(api.groups()).unwrap();
    let saved = saved.iter().find(|g| g.id == original.id).unwrap();
    assert_eq!(saved.name, "reviewC");
    assert_eq!(saved.policy, Policy::Random);
    assert!(saved.policy_params.is_empty());
    assert_eq!(saved.nodes.len(), 1);
    assert_eq!(saved.nodes[0].tag.as_deref(), Some("Hong Kong"));
    assert!(
        group_row(&page, "reviewC")
            .subtitle()
            .unwrap()
            .contains("1 nodes")
    );
    capture_themes(window, "groups-list");

    adw::prelude::ActionRowExt::activate(&group_row(&page, "reviewC"));
    let editor = editor_page(window);
    let member = find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Hong Kong").unwrap();
    click_icon(&member, "list-remove-symbolic");
    wait_for(|| !editor.can_pop());
    click_icon(&editor, "go-previous-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    click(&editor, "Reset");
    wait_for(|| find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Hong Kong").is_some());
    click_icon(
        &find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Hong Kong").unwrap(),
        "list-remove-symbolic",
    );
    wait_for(|| {
        find_widget::<gtk::Button>(&editor, &|b| {
            b.label().as_deref() == Some("Save") && b.is_sensitive()
        })
        .is_some()
    });
    click(&editor, "Save");
    wait_for(|| page.is_mapped());
    wait_for(|| {
        group_row(&page, "reviewC")
            .subtitle()
            .unwrap()
            .contains("0 nodes")
    });
    let groups = runtime.block_on(api.groups()).unwrap();
    assert!(
        groups
            .iter()
            .find(|g| g.id == original.id)
            .unwrap()
            .nodes
            .is_empty()
    );
    assert!(
        runtime
            .block_on(api.nodes())
            .unwrap()
            .iter()
            .any(|n| n.tag.as_deref() == Some("Hong Kong")),
        "removing group membership must not delete the node"
    );
    click_icon(&group_row(&page, "reviewC"), "user-trash-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    assert!(find_widget::<adw::ActionRow>(&page, &|r| r.title() == "reviewC").is_some());
    click_icon(&group_row(&page, "reviewC"), "user-trash-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Delete Group");
    wait_for(|| window.visible_dialog().is_none());
    wait_for(|| find_widget::<adw::ActionRow>(&page, &|r| r.title() == "reviewC").is_none());
    assert!(
        !runtime
            .block_on(api.groups())
            .unwrap()
            .iter()
            .any(|g| g.id == original.id)
    );
    runtime
        .block_on(
            api.remove_nodes(
                imported
                    .into_iter()
                    .map(|result| result.node.unwrap().id)
                    .collect(),
            ),
        )
        .unwrap();
}

fn create(window: &adw::ApplicationWindow, page: &adw::NavigationPage, name: &str, policy: u32) {
    click_icon(page, "list-add-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    assert!(
        !find_widget::<gtk::Button>(&dialog, &|b| b.label().as_deref() == Some("Create"))
            .unwrap()
            .is_sensitive()
    );
    fill(&dialog, "Name", name);
    find_widget::<adw::ComboRow>(&dialog, &|_| true)
        .unwrap()
        .set_selected(policy);
    click(&dialog, "Create");
    wait_for(|| window.visible_dialog().is_none());
    wait_for(|| find_widget::<adw::ActionRow>(page, &|r| r.title() == name).is_some());
}

fn duplicate_name_preserves_form(window: &adw::ApplicationWindow, page: &adw::NavigationPage) {
    click_icon(page, "list-add-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    fill(&dialog, "Name", "reviewA");
    click(&dialog, "Create");
    wait_for(|| {
        window
            .visible_dialog()
            .is_some_and(|d| d.is::<adw::AlertDialog>())
    });
    capture(window, "groups-create-error");
    click(&window.visible_dialog().unwrap(), "Close");
    wait_for(|| window.visible_dialog().is_some_and(|d| d == dialog));
    assert_eq!(
        find_widget::<adw::EntryRow>(&dialog, &|_| true)
            .unwrap()
            .text(),
        "reviewA"
    );
    click(&dialog, "Cancel");
    wait_for(|| window.visible_dialog().is_none());
}

fn editor_page(window: &adw::ApplicationWindow) -> adw::NavigationPage {
    wait_for(|| {
        find_widget::<adw::NavigationPage>(window, &|p| {
            p.tag().as_deref() == Some("group-editor") && p.is_mapped()
        })
        .is_some()
    });
    find_widget::<adw::NavigationPage>(window, &|p| {
        p.tag().as_deref() == Some("group-editor") && p.is_mapped()
    })
    .unwrap()
}

fn open_picker(window: &adw::ApplicationWindow, editor: &adw::NavigationPage) -> adw::Dialog {
    find_widget::<gtk::Button>(editor, &|b| {
        b.tooltip_text().as_deref() == Some("Add Nodes")
    })
    .unwrap()
    .emit_clicked();
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    wait_for(|| find_widget::<gtk::Spinner>(&dialog, &|s| !s.is_visible()).is_some());
    dialog
}

fn group_row(page: &adw::NavigationPage, name: &str) -> adw::ActionRow {
    find_widget::<adw::ActionRow>(page, &|r| r.title() == name).unwrap()
}

fn capture_themes(window: &adw::ApplicationWindow, name: &str) {
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
        wait_for(|| changed.elapsed() >= Duration::from_millis(300));
        capture(window, &format!("{name}-{theme}"));
        window.set_default_size(460, 640);
        wait_for(|| window.width() == 460);
        capture(window, &format!("{name}-narrow-{theme}"));
        window.set_default_size(900, 640);
        wait_for(|| window.width() == 900);
    }
    manager.set_color_scheme(original);
}
