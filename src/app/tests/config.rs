use super::*;
use serde_json::{Value, json};

async fn request(wing: &Wing, query: &str, variables: Value) -> Value {
    let saved = credentials::load("Home").await.unwrap().unwrap();
    let result: Value = reqwest::Client::new()
        .post(format!("http://{}/graphql", wing.address))
        .bearer_auth(saved.jwt.unwrap())
        .json(&json!({"query": query, "variables": variables}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(result.get("errors").is_none(), "{result}");
    result["data"].clone()
}

pub(super) fn selection_and_details(
    runtime: &tokio::runtime::Runtime,
    window: &adw::ApplicationWindow,
    wing: &Wing,
) {
    let nav = find_widget::<gtk::ListBoxRow>(window, &|row| {
        find_widget::<gtk::Label>(row, &|label| label.text() == "Config").is_some()
    })
    .unwrap();
    nav.activate();
    wait_for(|| {
        find_widget::<adw::StatusPage>(window, &|page| {
            page.is_mapped() && page.title() == "No Configurations"
        })
        .is_some()
    });

    let data = runtime.block_on(request(wing, r#"mutation {
        first: createConfig(name: "global <main>", global: {logLevel: "debug", tproxyPort: 23456, lanInterface: ["br0", "eth1"], checkInterval: "45s", allowInsecure: true}) {id}
        second: createConfig(name: "global (copy)", global: {tproxyPort: 23457}) {id}
    }"#, json!({})));
    let first = data["first"]["id"].as_str().unwrap();
    let second = data["second"]["id"].as_str().unwrap();
    let page = find_widget::<adw::NavigationPage>(window, &|page| {
        page.tag().as_deref() == Some("config-list")
    })
    .unwrap();
    click_icon(&page, "view-refresh-symbolic");
    wait_for(|| {
        find_widget::<adw::ActionRow>(&page, &|row| row.title() == "global <main>").is_some()
    });
    let row = find_widget::<adw::ActionRow>(&page, &|row| row.title() == "global <main>").unwrap();
    adw::prelude::ActionRowExt::activate(&row);
    wait_for(|| {
        find_widget::<adw::ActionRow>(&page, &|row| {
            row.title() == "global <main>"
                && find_widget::<gtk::CheckButton>(row, &|check| check.is_active()).is_some()
        })
        .is_some()
    });
    let selected = runtime.block_on(request(wing, "{ configs(selected: true) {id} }", json!({})));
    assert_eq!(selected["configs"], json!([{"id": first}]));
    let row = find_widget::<adw::ActionRow>(&page, &|row| row.title() == "global <main>").unwrap();
    assert!(
        row.subtitle().is_none_or(|s| s.is_empty()),
        "selection must not add a subtitle"
    );
    capture(window, "config-list");
    let copy = find_widget::<adw::ActionRow>(&page, &|row| row.title() == "global (copy)").unwrap();
    click_icon(&copy, "document-edit-symbolic");
    let copy_menu = find_widget::<gtk::MenuButton>(&copy, &|_| true).unwrap();
    copy_menu.popup();
    let copy_popover = copy_menu.popover().unwrap();
    wait_for(|| copy_popover.is_mapped());
    assert!(
        find_widget::<gtk::Button>(&copy_popover, &|button| button.label().as_deref()
            == Some("Delete"))
        .unwrap()
        .is_sensitive()
    );
    copy_menu.popdown();
    let selected = runtime.block_on(request(wing, "{ configs(selected: true) {id} }", json!({})));
    assert_eq!(
        selected["configs"],
        json!([{"id": first}]),
        "row actions must not select another configuration"
    );
    let menu = find_widget::<gtk::MenuButton>(&row, &|_| true).unwrap();
    menu.popup();
    let popover = menu.popover().unwrap();
    wait_for(|| popover.is_mapped());
    assert!(
        !find_widget::<gtk::Button>(&popover, &|b| b.label().as_deref() == Some("Delete"))
            .unwrap()
            .is_sensitive()
    );
    click(&popover, "View Details");
    wait_for(|| {
        find_widget::<adw::NavigationPage>(window, &|p| {
            p.is_mapped() && p.tag().as_deref() == Some("config-details")
        })
        .is_some()
    });
    let details = find_widget::<adw::NavigationPage>(window, &|p| {
        p.tag().as_deref() == Some("config-details")
    })
    .unwrap();
    assert_eq!(details.title(), "global <main>");
    assert!(find_widget::<gtk::Label>(&details, &|l| l.text() == "23456").is_some());
    assert!(find_widget::<gtk::Label>(&details, &|l| l.text() == "br0\neth1").is_some());
    assert!(find_widget::<gtk::Label>(&details, &|l| l.text() == "45s").is_some());
    capture(window, "config-details");
    language(window, "zh_CN");
    wait_for(|| {
        find_widget::<adw::PreferencesGroup>(&details, &|g| g.title() == "软件选项").is_some()
    });
    capture(window, "config-details-zh");
    window.set_default_size(460, 640);
    wait_for(|| window.width() <= 460);
    capture(window, "config-details-narrow");
    window.set_default_size(900, 640);
    wait_for(|| window.width() >= 900);
    language(window, "en");
    let navigation = details
        .ancestor(adw::NavigationView::static_type())
        .unwrap()
        .downcast::<adw::NavigationView>()
        .unwrap();
    navigation.pop();
    wait_for(|| page.is_mapped());
    let copy = find_widget::<adw::ActionRow>(&page, &|row| row.title() == "global (copy)").unwrap();
    adw::prelude::ActionRowExt::activate(&copy);
    wait_for(|| {
        find_widget::<adw::ActionRow>(&page, &|row| {
            row.title() == "global (copy)"
                && find_widget::<gtk::CheckButton>(row, &|c| c.is_active()).is_some()
        })
        .is_some()
    });
    let selected = runtime.block_on(request(wing, "{ configs(selected: true) {id} }", json!({})));
    assert_eq!(selected["configs"], json!([{"id": second}]));

    // A config may be removed by another client between listing and selecting it.
    runtime.block_on(request(
        wing,
        "mutation($id: ID!) { removeConfig(id: $id) }",
        json!({"id": first}),
    ));
    adw::prelude::ActionRowExt::activate(
        &find_widget::<adw::ActionRow>(&page, &|row| row.title() == "global <main>").unwrap(),
    );
    wait_for(|| window.visible_dialog().is_some());
    let error = window.visible_dialog().unwrap();
    assert!(find_widget::<gtk::Label>(&error, &|l| l.text().contains("no such config")).is_some());
    click(&error, "Close");
    wait_for(|| window.visible_dialog().is_none());
    assert!(
        find_widget::<adw::ActionRow>(&page, &|row| row.title() == "global (copy)"
            && find_widget::<gtk::CheckButton>(row, &|c| c.is_active()).is_some())
        .is_some()
    );
    find_widget::<gtk::ListBoxRow>(window, &|row| {
        find_widget::<gtk::Label>(row, &|l| l.text() == "Overview").is_some()
    })
    .unwrap()
    .activate();
}
