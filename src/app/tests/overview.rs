use super::*;
use crate::api::{group::Policy, node::ImportArgument};

pub fn summary_follows_backend(
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
    let config_id = runtime.block_on(async {
        let mut config = api.default_config().await.unwrap();
        config.name = "Summary <active>".into();
        api.save_config(config, None).await.unwrap();
        let id = api.configs().await.unwrap()[0].id.clone();
        api.select_config(id.clone()).await.unwrap();
        api.create_group("OverviewGroup".into(), Policy::Random)
            .await
            .unwrap();
        api.import_nodes(vec![ImportArgument {
            link: "socks5://127.0.0.1:1#Summary".into(),
            tag: None,
        }])
        .await
        .unwrap();
        id
    });
    navigation_row(window, "Overview").activate();
    let refresh = find_widget::<gtk::Button>(window, &|b| {
        b.tooltip_text().as_deref() == Some("Refresh Overview")
    })
    .expect("overview must provide a refresh action for its backend summary");
    refresh.emit_clicked();
    wait_for(|| {
        find_widget::<adw::ActionRow>(window, &|r| {
            r.is_mapped()
                && r.title() == "Config"
                && r.subtitle().as_deref() == Some("Summary <active>")
        })
        .is_some()
    });
    let config_row =
        find_widget::<adw::ActionRow>(window, &|r| r.is_mapped() && r.title() == "Config").unwrap();
    let overview = config_row
        .ancestor(gtk::ScrolledWindow::static_type())
        .unwrap();
    let nodes = find_widget::<adw::ActionRow>(&overview, &|r| r.title() == "Nodes").unwrap();
    let groups = find_widget::<adw::ActionRow>(&overview, &|r| r.title() == "Groups").unwrap();
    assert!(find_widget::<gtk::Label>(&nodes, &|l| l.text() == "1").is_some());
    assert!(find_widget::<gtk::Label>(&groups, &|l| l.text() == "1").is_some());
    assert!(
        find_widget::<adw::ActionRow>(&overview, &|r| r.title() == "Transparent Proxy Port")
            .is_none()
    );
    let manager = adw::StyleManager::default();
    let original_theme = manager.color_scheme();
    manager.set_color_scheme(adw::ColorScheme::ForceLight);
    capture(window, "overview-summary-light");
    manager.set_color_scheme(adw::ColorScheme::ForceDark);
    capture(window, "overview-summary-dark");
    window.set_default_size(460, 760);
    wait_for(|| window.width() <= 460);
    capture(window, "overview-summary-narrow-dark");
    manager.set_color_scheme(adw::ColorScheme::ForceLight);
    capture(window, "overview-summary-narrow-light");
    window.set_default_size(900, 760);
    wait_for(|| window.width() >= 900);
    manager.set_color_scheme(original_theme);
    runtime.block_on(async {
        api.remove_config(config_id).await.unwrap();
        api.remove_group(api.groups().await.unwrap()[0].id.clone())
            .await
            .unwrap();
        api.remove_nodes(
            api.nodes()
                .await
                .unwrap()
                .into_iter()
                .map(|n| n.id)
                .collect(),
        )
        .await
        .unwrap();
    });
    // Returning to Overview must refresh the summary, including empty collections.
    navigation_row(window, "Routing").activate();
    wait_for(|| !overview.is_mapped());
    navigation_row(window, "Overview").activate();
    wait_for(|| config_row.subtitle().as_deref() == Some("None selected"));
    assert!(find_widget::<gtk::Label>(&nodes, &|l| l.text() == "0").is_some());
    assert!(find_widget::<gtk::Label>(&groups, &|l| l.text() == "0").is_some());
}
