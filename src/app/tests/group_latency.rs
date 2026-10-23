use super::node_actions::ProbeEndpoint;
use super::*;
use crate::{
    api::{group::Policy, node::ImportArgument},
    ui::groups::Groups,
};
use std::sync::atomic::Ordering;

pub fn member_latency_and_removal(runtime: &tokio::runtime::Runtime) {
    let wing = Wing::start();
    let probe = ProbeEndpoint::start(runtime);
    let api = runtime.block_on(async {
        let (api, _) = Api::new(format!("http://{}", wing.address).parse().unwrap())
            .authenticate(&Credentials::new("group-admin", "group-password-42"), true)
            .await
            .unwrap();
        let mut config = api.default_config().await.unwrap();
        config.name = "Local probe".into();
        config.global.tcp_check_url = vec!["http://127.0.0.1/".into()];
        api.save_config(config, None).await.unwrap();
        api.select_config(api.configs().await.unwrap()[0].id.clone())
            .await
            .unwrap();
        api.import_nodes(vec![
            ImportArgument {
                link: format!("socks5://{}#Local", probe.address),
                tag: Some("Reachable".into()),
            },
            ImportArgument {
                link: "socks5://127.0.0.1:1#Closed".into(),
                tag: Some("Offline".into()),
            },
        ])
        .await
        .unwrap();
        api.create_group("LatencyGroup".into(), Policy::Random)
            .await
            .unwrap();
        let original = api.groups().await.unwrap().remove(0);
        let mut edited = original.clone();
        edited.nodes = api
            .nodes()
            .await
            .unwrap()
            .into_iter()
            .map(Into::into)
            .collect();
        api.save_group(&original, &edited).await.unwrap();
        api
    });
    let controller = Groups::builder().launch(api.clone()).detach();
    let window = adw::ApplicationWindow::builder()
        .content(controller.widget())
        .default_width(700)
        .default_height(640)
        .build();
    window.present();
    wait_for(|| find_widget::<adw::ActionRow>(&window, &|r| r.title() == "LatencyGroup").is_some());
    adw::prelude::ActionRowExt::activate(
        &find_widget::<adw::ActionRow>(&window, &|r| r.title() == "LatencyGroup").unwrap(),
    );
    wait_for(|| {
        find_widget::<adw::NavigationPage>(&window, &|p| p.tag().as_deref() == Some("group-editor"))
            .is_some()
    });
    let editor = find_widget::<adw::NavigationPage>(&window, &|p| {
        p.tag().as_deref() == Some("group-editor")
    })
    .unwrap();
    wait_for(|| find_widget::<gtk::Label>(&editor, &|l| l.text() == "Testing…").is_some());
    wait_for(|| probe.requests.load(Ordering::SeqCst) == 1);
    let test = find_widget::<gtk::Button>(&editor, &|b| {
        b.icon_name().as_deref() == Some("network-transmit-receive-symbolic")
    })
    .unwrap();
    assert!(!test.is_sensitive());
    probe.release.add_permits(1);
    wait_for(|| test.is_sensitive());
    let reachable = find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Reachable").unwrap();
    assert!(find_widget::<gtk::Label>(&reachable, &|l| l.text().ends_with(" ms")).is_some());
    let offline = find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Offline").unwrap();
    assert!(find_widget::<gtk::Label>(&offline, &|l| l.text() == "Test failed").is_some());
    assert!(
        editor.can_pop(),
        "latency results must not dirty the group form"
    );
    let manager = adw::StyleManager::default();
    let original_theme = manager.color_scheme();
    manager.set_color_scheme(adw::ColorScheme::ForceLight);
    capture(&window, "group-latency-light");
    manager.set_color_scheme(adw::ColorScheme::ForceDark);
    capture(&window, "group-latency-dark");
    window.set_default_size(400, 640);
    wait_for(|| window.width() <= 400);
    capture(&window, "group-latency-narrow-dark");
    manager.set_color_scheme(adw::ColorScheme::ForceLight);
    capture(&window, "group-latency-narrow-light");
    test.emit_clicked();
    wait_for(|| probe.requests.load(Ordering::SeqCst) == 2);
    click_icon(&reachable, "list-remove-symbolic");
    wait_for(|| find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Reachable").is_none());
    probe.release.add_permits(1);
    wait_for(|| test.is_sensitive());
    assert!(
        find_widget::<adw::ActionRow>(&editor, &|r| r.title() == "Reachable").is_none(),
        "late results must not restore removed members"
    );
    click(&editor, "Save");
    wait_for(|| !editor.is_mapped());
    let saved = runtime.block_on(api.groups()).unwrap().remove(0);
    assert_eq!(saved.nodes.len(), 1);
    assert_eq!(saved.nodes[0].tag.as_deref(), Some("Offline"));
    manager.set_color_scheme(original_theme);
    window.close();
    drop(controller);
}
