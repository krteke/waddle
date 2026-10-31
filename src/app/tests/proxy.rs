use super::*;
use crate::ui::proxy::{Proxy, ProxyMsg};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::Notify,
};

struct ProxyServer {
    api: Api,
    changes: Arc<Mutex<Vec<bool>>>,
    release_change: Arc<Notify>,
    fail_change: Arc<AtomicBool>,
    fail_read: Arc<AtomicBool>,
    modified: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<()>,
}

impl ProxyServer {
    fn start(runtime: &tokio::runtime::Runtime) -> Self {
        let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
        let api = Api::new(
            format!("http://{}", listener.local_addr().unwrap())
                .parse()
                .unwrap(),
        )
        .with_token("proxy-test")
        .unwrap();
        let changes = Arc::new(Mutex::new(Vec::new()));
        let release_change = Arc::new(Notify::new());
        let fail_change = Arc::new(AtomicBool::new(false));
        let fail_read = Arc::new(AtomicBool::new(false));
        let modified = Arc::new(AtomicBool::new(false));
        let task = runtime.spawn({
            let changes = changes.clone();
            let release_change = release_change.clone();
            let fail_change = fail_change.clone();
            let fail_read = fail_read.clone();
            let modified = modified.clone();
            async move {
                let mut running = true;
                loop {
                    let (stream, _) = listener.accept().await.unwrap();
                    let mut stream = BufReader::new(stream);
                    let mut line = String::new();
                    stream.read_line(&mut line).await.unwrap();
                    assert_eq!(line.trim(), "POST /graphql HTTP/1.1");
                    let mut length = 0;
                    let mut authorized = false;
                    loop {
                        line.clear();
                        stream.read_line(&mut line).await.unwrap();
                        if line == "\r\n" { break; }
                        let header = line.to_ascii_lowercase();
                        if let Some(value) = header.strip_prefix("content-length:") {
                            length = value.trim().parse().unwrap();
                        }
                        if header.trim() == "authorization: bearer proxy-test" {
                            authorized = true;
                        }
                    }
                    assert!(authorized, "proxy requests must use the saved JWT");
                    let mut body = vec![0; length];
                    stream.read_exact(&mut body).await.unwrap();
                    let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
                    let body = match request["operationName"].as_str().unwrap() {
                        "ProxyStatus" if fail_read.load(Ordering::SeqCst) => serde_json::json!({"errors": [{"message": "private status failure"}]}),
                        "ProxyStatus" => serde_json::json!({"data": {"general": {"dae": {"running": running, "modified": running && modified.load(Ordering::SeqCst)}}}}),
                        "RunProxy" => {
                            let desired = !request["variables"]["dry"].as_bool().unwrap();
                            changes.lock().unwrap().push(desired);
                            release_change.notified().await;
                            if fail_change.load(Ordering::SeqCst) {
                                serde_json::json!({"errors": [
                                    {"message": "Failed to load <global> config\ninvalid routing rule: domain(suffix: example.com)"},
                                    {"message": "Interface eth0: permission denied"}
                                ]})
                            } else {
                                running = desired;
                                modified.store(false, Ordering::SeqCst);
                                serde_json::json!({"data": {"run": 1}})
                            }
                        }
                        operation => panic!("unexpected operation: {operation}"),
                    }.to_string();
                    stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
                }
            }
        });
        Self {
            api,
            changes,
            release_change,
            fail_change,
            fail_read,
            modified,
            task,
        }
    }
}

impl Drop for ProxyServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub fn proxy_control(runtime: &tokio::runtime::Runtime) {
    let server = ProxyServer::start(runtime);
    let status = adw::ActionRow::new();
    let controller = Proxy::builder()
        .launch((server.api.clone(), status.clone()))
        .detach();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.append(controller.widget());
    let status_list = gtk::ListBox::new();
    status_list.append(&status);
    content.append(&status_list);
    let window = adw::ApplicationWindow::builder().content(&content).build();
    window.present();
    let switch = find_widget::<adw::SwitchRow>(&window, &|_| true).unwrap();
    wait_for(|| switch.is_sensitive() && switch.is_active());
    assert_eq!(status.subtitle().as_deref(), Some("Running"));
    assert!(
        server.changes.lock().unwrap().is_empty(),
        "rendering the initial state must not send a mutation"
    );

    switch.set_active(false);
    wait_for(|| !switch.is_sensitive() && status.subtitle().as_deref() == Some("Stopping..."));
    controller.emit(ProxyMsg::SetRunning(true));
    server.release_change.notify_one();
    wait_for(|| switch.is_sensitive() && !switch.is_active());
    assert_eq!(status.subtitle().as_deref(), Some("Stopped"));
    assert_eq!(*server.changes.lock().unwrap(), [false]);

    switch.set_active(true);
    wait_for(|| !switch.is_sensitive() && status.subtitle().as_deref() == Some("Starting..."));
    server.release_change.notify_one();
    wait_for(|| switch.is_sensitive() && switch.is_active());
    assert_eq!(status.subtitle().as_deref(), Some("Running"));
    assert_eq!(*server.changes.lock().unwrap(), [false, true]);

    server.fail_change.store(true, Ordering::SeqCst);
    switch.set_active(false);
    wait_for(|| !switch.is_sensitive());
    server.release_change.notify_one();
    wait_for(|| switch.is_sensitive() && switch.is_active());
    assert_eq!(status.subtitle().as_deref(), Some("Running"));
    assert!(
        find_widget::<gtk::Label>(&window, &|label| label.text()
            == "Could not change proxy status.")
        .is_some()
    );
    assert_eq!(*server.changes.lock().unwrap(), [false, true, false]);

    let dialog = window
        .visible_dialog()
        .expect("proxy changes must show their failure reason");
    let details = find_widget::<gtk::Label>(&dialog, &|label| label.text()
        == "Failed to load <global> config\ninvalid routing rule: domain(suffix: example.com)\n\nInterface eth0: permission denied")
        .expect("all backend messages must be displayed verbatim");
    assert!(details.is_selectable());
    assert!(!details.uses_markup());
    capture(&window, "proxy-error-details");
    click(&dialog, "Close");
    wait_for(|| window.visible_dialog().is_none());
    click_icon(&window, "dialog-information-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Close");
    wait_for(|| window.visible_dialog().is_none());

    server.fail_read.store(true, Ordering::SeqCst);
    click_icon(&window, "view-refresh-symbolic");
    wait_for(|| status.subtitle().as_deref() == Some("Unknown"));
    assert!(!switch.is_sensitive());
    assert!(
        find_widget::<gtk::Label>(&window, &|label| label.text()
            == "Could not read proxy status.")
        .is_some()
    );
    assert!(
        window.visible_dialog().is_none(),
        "status reads should not interrupt with a dialog"
    );
    click_icon(&window, "dialog-information-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    assert!(
        find_widget::<gtk::Label>(&dialog, &|label| label.text() == "private status failure")
            .is_some()
    );
    click(&dialog, "Close");
    wait_for(|| window.visible_dialog().is_none());
    server.fail_read.store(false, Ordering::SeqCst);
    click_icon(&window, "view-refresh-symbolic");
    wait_for(|| switch.is_sensitive() && switch.is_active());
    assert_eq!(status.subtitle().as_deref(), Some("Running"));
    assert_eq!(*server.changes.lock().unwrap(), [false, true, false]);
    window.close();
}

pub fn reload_control(runtime: &tokio::runtime::Runtime) {
    let server = ProxyServer::start(runtime);
    let status = adw::ActionRow::new();
    let controller = Proxy::builder()
        .launch((server.api.clone(), status.clone()))
        .detach();
    let window = adw::ApplicationWindow::builder()
        .content(controller.widget())
        .default_width(340)
        .build();
    window.present();
    let switch = find_widget::<adw::SwitchRow>(&window, &|_| true).unwrap();
    wait_for(|| switch.is_sensitive() && switch.is_active());
    assert!(
        find_widget::<gtk::Button>(&window, &|b| b.is_mapped()
            && b.tooltip_text().as_deref() == Some("Reload Configuration"))
        .is_none()
    );
    server.modified.store(true, Ordering::SeqCst);
    controller.emit(ProxyMsg::Refresh);
    wait_for(|| {
        find_widget::<gtk::Button>(&window, &|b| {
            b.is_mapped() && b.tooltip_text().as_deref() == Some("Reload Configuration")
        })
        .is_some()
    });
    let reload = find_widget::<gtk::Button>(&window, &|b| {
        b.tooltip_text().as_deref() == Some("Reload Configuration")
    })
    .unwrap();
    capture(&window, "proxy-reload-pending-light");
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
    capture(&window, "proxy-reload-pending-dark");
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::Default);
    reload.emit_clicked();
    wait_for(|| !reload.is_sensitive() && !switch.is_sensitive());
    wait_for(|| server.changes.lock().unwrap().as_slice() == [true]);
    server.release_change.notify_one();
    wait_for(|| switch.is_sensitive() && !reload.is_mapped());
    assert!(switch.is_active(), "reloading must not stop the proxy");

    server.modified.store(true, Ordering::SeqCst);
    controller.emit(ProxyMsg::Refresh);
    wait_for(|| reload.is_mapped());
    server.fail_change.store(true, Ordering::SeqCst);
    reload.emit_clicked();
    wait_for(|| server.changes.lock().unwrap().len() == 2);
    server.release_change.notify_one();
    wait_for(|| window.visible_dialog().is_some());
    let dialog = window.visible_dialog().unwrap();
    assert!(
        find_widget::<gtk::Label>(&dialog, &|l| l.text().contains("invalid routing rule")
            && l.text().contains("permission denied"))
        .is_some()
    );
    click(&dialog, "Close");
    wait_for(|| window.visible_dialog().is_none());
    assert!(
        reload.is_mapped() && reload.is_sensitive(),
        "failed reload must remain available for retry"
    );
    assert!(switch.is_active());
    assert_eq!(*server.changes.lock().unwrap(), [true, true]);
    window.close();
}

pub fn routing_changes_refresh_proxy_status(runtime: &tokio::runtime::Runtime) {
    use crate::ui::{ServerEvent, routing::Routings};
    let wing = Wing::start();
    let api = runtime.block_on(async {
        let (api, _) = Api::new(format!("http://{}", wing.address).parse().unwrap())
            .authenticate(
                &Credentials::new("reload-admin", "reload-password-42"),
                true,
            )
            .await
            .unwrap();
        api.create_routing("first".into(), "fallback: direct".into())
            .await
            .unwrap();
        api.create_routing("second".into(), "fallback: direct".into())
            .await
            .unwrap();
        api.select_routing(api.routings().await.unwrap().routings[0].id.clone())
            .await
            .unwrap();
        api
    });
    let server = ProxyServer::start(runtime);
    let proxy = Proxy::builder()
        .launch((server.api.clone(), adw::ActionRow::new()))
        .detach();
    let routings = Routings::builder()
        .launch(api.clone())
        .forward(proxy.sender(), |event| match event {
            ServerEvent::Changed => ProxyMsg::Refresh,
            ServerEvent::Unauthorized => panic!("routing request must remain authenticated"),
        });
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.append(proxy.widget());
    content.append(routings.widget());
    let window = adw::ApplicationWindow::builder()
        .content(&content)
        .default_width(760)
        .default_height(720)
        .build();
    window.present();
    wait_for(|| find_widget::<adw::ActionRow>(&window, &|r| r.title() == "second").is_some());
    let reload = find_widget::<gtk::Button>(&window, &|b| {
        b.tooltip_text().as_deref() == Some("Reload Configuration")
    })
    .unwrap();
    let switch = find_widget::<adw::SwitchRow>(&window, &|_| true).unwrap();
    wait_for(|| switch.is_sensitive());
    assert!(!reload.is_mapped());
    // API-only cannot load the kernel proxy. Only its runtime status/reload HTTP
    // boundary is simulated; route selection and editing use the real dae-wing.
    server.modified.store(true, Ordering::SeqCst);
    let second = find_widget::<adw::ActionRow>(&window, &|r| r.title() == "second").unwrap();
    adw::prelude::ActionRowExt::activate(&second);
    wait_for(|| reload.is_mapped() && reload.is_sensitive());
    let selected = runtime
        .block_on(api.routings())
        .unwrap()
        .routings
        .into_iter()
        .find(|r| r.selected)
        .unwrap();
    assert_eq!(selected.name, "second");
    reload.emit_clicked();
    wait_for(|| server.changes.lock().unwrap().len() == 1);
    server.release_change.notify_one();
    wait_for(|| !reload.is_mapped() && switch.is_sensitive());
    let second = find_widget::<adw::ActionRow>(&window, &|r| r.title() == "second").unwrap();
    click_icon(&second, "document-edit-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    let editor = window.visible_dialog().unwrap();
    let text = find_widget::<gtk::TextView>(&editor, &|_| true).unwrap();
    wait_for(|| text.is_mapped());
    text.buffer()
        .set_text("domain(example.com) -> direct\nfallback: direct");
    wait_for(|| {
        find_widget::<gtk::Button>(&editor, &|b| {
            b.label().as_deref() == Some("Save") && b.is_sensitive()
        })
        .is_some()
    });
    server.modified.store(true, Ordering::SeqCst);
    click(&editor, "Save");
    wait_for(|| window.visible_dialog().is_none());
    wait_for(|| reload.is_mapped() && reload.is_sensitive());
    let edited = runtime
        .block_on(api.routings())
        .unwrap()
        .routings
        .into_iter()
        .find(|r| r.id == selected.id)
        .unwrap();
    assert_eq!(
        edited.routing.string,
        "domain(example.com) -> direct\nfallback: direct"
    );
    assert_eq!(
        *server.changes.lock().unwrap(),
        [true],
        "saving must not apply the config automatically"
    );
    window.close();
}
