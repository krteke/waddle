use super::*;
use crate::ui::overview::{Overview, OverviewMsg, OverviewOutput};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
    },
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};

// The runtime counters require real proxied traffic. Control only that HTTP boundary;
// the separate overview test uses dae-wing itself, including its idle statistics.
pub fn live_statistics(runtime: &tokio::runtime::Runtime) {
    let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let api = Api::new(
        format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap(),
    )
    .with_token("traffic-test")
    .unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let mode = Arc::new(AtomicU8::new(0));
    let task = runtime.spawn({
        let requests = requests.clone();
        let mode = mode.clone();
        async move {
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
                    authorized |= header.trim() == "authorization: bearer traffic-test";
                }
                assert!(authorized);
                let mut bytes = vec![0; length];
                stream.read_exact(&mut bytes).await.unwrap();
                let request: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                let body = match request["operationName"].as_str().unwrap() {
                    "OverviewSummary" => serde_json::json!({"data": {
                        "general": {"dae": {"version": "test"}}, "configs": [], "dnss": [],
                        "routings": [], "nodes": {"totalCount": 0}, "groups": [], "subscriptions": []
                    }}),
                    "TrafficOverview" => {
                        requests.lock().unwrap().push(request["variables"].clone());
                        match mode.load(Ordering::SeqCst) {
                            1 => serde_json::json!({"errors": [{"message": "private statistics failure"}]}),
                            2 => serde_json::json!({"errors": [{"message": "access denied"}]}),
                            _ => serde_json::json!({"data": {"general": {"runtimeOverview": {
                                "updatedAt": "2026-11-02T10:00:00Z",
                                "uploadRate": 2048.0, "downloadRate": 1048576.0,
                                "uploadTotal": "1073741824", "downloadTotal": "5368709120",
                                "activeConnections": 42, "udpSessions": 7,
                                "samples": [
                                    {"timestamp": "2026-11-02T09:59:10Z", "uploadRate": 0.0, "downloadRate": 1048576.0},
                                    {"timestamp": "2026-11-02T09:59:20Z", "uploadRate": 4096.0, "downloadRate": 524288.0},
                                    {"timestamp": "2026-11-02T09:59:40Z", "uploadRate": 1024.0, "downloadRate": 2097152.0},
                                    {"timestamp": "2026-11-02T10:00:00Z", "uploadRate": 2048.0, "downloadRate": 1048576.0}
                                ]
                            }}}}),
                        }
                    }
                    other => panic!("unexpected operation: {other}"),
                }.to_string();
                stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        }
    });
    let unauthorized = Rc::new(Cell::new(false));
    let controller = Overview::builder()
        .launch((api, "admin".into()))
        .connect_receiver({
            let unauthorized = unauthorized.clone();
            move |_, message| {
                assert!(matches!(message, OverviewOutput::Unauthorized));
                unauthorized.set(true);
            }
        });
    let window = adw::ApplicationWindow::builder()
        .default_width(820)
        .default_height(900)
        .content(controller.widget())
        .build();
    window.present();
    wait_for(|| find_widget::<gtk::Label>(&window, &|l| l.text() == "2.0 KiB/s").is_some());
    assert!(find_widget::<gtk::Label>(&window, &|l| l.text() == "1.0 MiB/s").is_some());
    assert!(find_widget::<gtk::Label>(&window, &|l| l.text() == "Total Upload: 1.0 GiB").is_some());
    assert!(
        find_widget::<gtk::Label>(&window, &|l| l.text() == "Total Download: 5.0 GiB").is_some()
    );
    let connections =
        find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Active Connections").unwrap();
    assert!(find_widget::<gtk::Label>(&connections, &|l| l.text() == "42").is_some());
    let udp = find_widget::<adw::ActionRow>(&window, &|r| r.title() == "UDP Sessions").unwrap();
    assert!(find_widget::<gtk::Label>(&udp, &|l| l.text() == "7").is_some());
    assert_eq!(
        requests.lock().unwrap()[0],
        serde_json::json!({"windowSec": 60, "maxPoints": 120})
    );
    let style = adw::StyleManager::default();
    let original = style.color_scheme();
    style.set_color_scheme(adw::ColorScheme::ForceLight);
    capture(&window, "traffic-light");
    style.set_color_scheme(adw::ColorScheme::ForceDark);
    capture(&window, "traffic-dark");
    window.set_default_size(390, 900);
    wait_for(|| window.width() <= 390);
    capture(&window, "traffic-narrow-dark");
    style.set_color_scheme(adw::ColorScheme::ForceLight);
    capture(&window, "traffic-narrow-light");
    rust_i18n::set_locale("zh_CN");
    controller.emit(OverviewMsg::Retranslate);
    wait_for(|| find_widget::<gtk::Label>(&window, &|l| l.text() == "累计上传：1.0 GiB").is_some());
    capture(&window, "traffic-narrow-zh-light");
    style.set_color_scheme(adw::ColorScheme::ForceDark);
    capture(&window, "traffic-narrow-zh-dark");
    rust_i18n::set_locale("en");
    controller.emit(OverviewMsg::Retranslate);
    style.set_color_scheme(original);
    let ranges = find_widget::<adw::ToggleGroup>(&window, &|_| true).unwrap();
    ranges.set_active(3);
    wait_for(|| requests.lock().unwrap().last().unwrap()["windowSec"] == 3600);
    assert_eq!(requests.lock().unwrap().last().unwrap()["maxPoints"], 480);
    ranges.set_active(0);
    wait_for(|| requests.lock().unwrap().last().unwrap()["windowSec"] == 60);
    mode.store(1, Ordering::SeqCst);
    wait_for(|| {
        find_widget::<gtk::Label>(&window, &|l| {
            l.is_visible() && l.text() == rust_i18n::t!("traffic.unavailable").as_ref()
        })
        .is_some()
    });
    assert!(
        find_widget::<gtk::Label>(&window, &|l| l.text() == "2.0 KiB/s").is_none(),
        "failed refresh must not present stale rates as live"
    );
    mode.store(0, Ordering::SeqCst);
    wait_for(|| find_widget::<gtk::Label>(&window, &|l| l.text() == "2.0 KiB/s").is_some());
    controller.widget().set_visible(false);
    wait_for(|| !controller.widget().is_mapped());
    // Let a request already in flight finish before checking that polling stopped.
    let until = Instant::now() + Duration::from_millis(150);
    wait_for(|| Instant::now() >= until);
    let paused_count = requests.lock().unwrap().len();
    let until = Instant::now() + Duration::from_millis(1100);
    wait_for(|| Instant::now() >= until);
    assert_eq!(requests.lock().unwrap().len(), paused_count);
    controller.widget().set_visible(true);
    wait_for(|| requests.lock().unwrap().len() > paused_count);
    mode.store(2, Ordering::SeqCst);
    wait_for(|| unauthorized.get());
    drop(controller);
    window.close();
    task.abort();
}
