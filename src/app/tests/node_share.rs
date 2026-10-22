use super::*;
use crate::{api::node::ImportArgument, ui::nodes::Nodes};

const LINK: &str =
    "trojan://test-password@hk.example.com:443?allowInsecure=1&sni=hk.example.com#Original%20Name";

pub(super) fn shares_original_link(runtime: &tokio::runtime::Runtime) {
    let wing = Wing::start();
    let long_link = format!("trojan://{}@hk.example.com:443#Long", "x".repeat(4000));
    let api = runtime.block_on(async {
        let (api, _) = Api::new(format!("http://{}", wing.address).parse().unwrap())
            .authenticate(&Credentials::new("share-admin", "share-password-42"), true)
            .await
            .unwrap();
        api.import_nodes(vec![
            ImportArgument {
                link: LINK.into(),
                tag: Some("Hong Kong".into()),
            },
            ImportArgument {
                link: long_link.clone(),
                tag: Some("Long link".into()),
            },
        ])
        .await
        .unwrap();
        api
    });
    let controller = Nodes::builder().launch(api).detach();
    let window = adw::ApplicationWindow::builder()
        .content(controller.widget())
        .default_width(500)
        .default_height(700)
        .resizable(false)
        .build();
    window.present();
    wait_for(|| window.width() == 500);
    window.set_resizable(true);
    wait_for(|| find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Hong Kong").is_some());
    let row = find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Hong Kong").unwrap();
    click_icon(&row, "insert-link-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    let link = find_widget::<gtk::TextView>(&dialog, &|_| true).expect("a selectable link field");
    assert!(!link.is_editable());
    let buffer = link.buffer();
    assert_eq!(
        buffer.text(&buffer.start_iter(), &buffer.end_iter(), false),
        LINK
    );
    buffer.select_range(&buffer.iter_at_offset(0), &buffer.iter_at_offset(6));
    link.emit_by_name::<()>("copy-clipboard", &[]);
    let context = glib::MainContext::default();
    assert_eq!(
        context
            .block_on(link.clipboard().read_text_future())
            .unwrap()
            .as_deref(),
        Some("trojan")
    );
    click(&dialog, "Copy Link");
    assert_eq!(
        context
            .block_on(link.clipboard().read_text_future())
            .unwrap()
            .as_deref(),
        Some(LINK)
    );
    wait_for(|| {
        find_widget::<gtk::Button>(&dialog, &|b| b.label().as_deref() == Some("Copied")).is_some()
    });
    let qr = find_widget::<gtk::DrawingArea>(&dialog, &|_| true).expect("the QR drawing area");
    capture(&qr.parent().unwrap(), "node-share-qr");
    let manager = adw::StyleManager::default();
    let original = manager.color_scheme();
    manager.set_color_scheme(adw::ColorScheme::ForceLight);
    wait_for(|| !manager.is_dark());
    capture(&window, "node-share-light");
    manager.set_color_scheme(adw::ColorScheme::ForceDark);
    wait_for(|| manager.is_dark());
    let changed = Instant::now();
    wait_for(|| changed.elapsed() >= Duration::from_millis(300));
    capture(&window, "node-share-dark");
    dialog.force_close();
    wait_for(|| window.visible_dialog().is_none());
    let long_row = find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Long link").unwrap();
    click_icon(&long_row, "insert-link-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let long_dialog = window.visible_dialog().unwrap();
    assert!(find_widget::<gtk::DrawingArea>(&long_dialog, &|_| true).is_none());
    assert!(
        find_widget::<gtk::Label>(&long_dialog, &|l| l.text()
            == "Could not generate a QR code. You can still copy the link below.")
        .is_some()
    );
    let long_field = find_widget::<gtk::TextView>(&long_dialog, &|_| true).unwrap();
    let buffer = long_field.buffer();
    assert_eq!(
        buffer.text(&buffer.start_iter(), &buffer.end_iter(), false),
        long_link
    );
    click(&long_dialog, "Copy Link");
    assert_eq!(
        context
            .block_on(long_field.clipboard().read_text_future())
            .unwrap()
            .as_deref(),
        Some(long_link.as_str())
    );
    capture(&window, "node-share-long-link");
    long_dialog.force_close();
    wait_for(|| window.visible_dialog().is_none());
    rust_i18n::set_locale("zh_CN");
    click_icon(&row, "insert-link-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let chinese_dialog = window.visible_dialog().unwrap();
    click(&chinese_dialog, "复制链接");
    assert!(
        find_widget::<gtk::Button>(&chinese_dialog, &|b| b.label().as_deref() == Some("已复制"))
            .is_some()
    );
    window.set_default_size(360, 600);
    wait_for(|| window.width() == 360);
    capture(&window, "node-share-chinese-narrow-dark");
    manager.set_color_scheme(adw::ColorScheme::ForceLight);
    wait_for(|| !manager.is_dark());
    let changed = Instant::now();
    wait_for(|| changed.elapsed() >= Duration::from_millis(300));
    capture(&window, "node-share-chinese-narrow-light");
    rust_i18n::set_locale("en");
    window.set_content(None::<&gtk::Widget>);
    drop(controller);
    wait_for(|| window.visible_dialog().is_none());
    manager.set_color_scheme(original);
    window.close();
}
