use super::*;
use crate::{
    api::node::ImportArgument,
    ui::nodes::{NodeMsg, Nodes},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::Semaphore,
};

// A local SOCKS5 endpoint answers the backend's HTTP check, without public network access.
struct ProbeEndpoint {
    address: std::net::SocketAddr,
    requests: Arc<AtomicUsize>,
    release: Arc<Semaphore>,
    task: tokio::task::JoinHandle<()>,
}

impl ProbeEndpoint {
    fn start(runtime: &tokio::runtime::Runtime) -> Self {
        let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let release = Arc::new(Semaphore::new(0));
        let task = runtime.spawn({
            let requests = requests.clone();
            let release = release.clone();
            async move {
                loop {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let mut greeting = [0; 3];
                    stream.read_exact(&mut greeting).await.unwrap();
                    assert_eq!(greeting, [5, 1, 0]);
                    stream.write_all(&[5, 0]).await.unwrap();
                    let mut header = [0; 4];
                    stream.read_exact(&mut header).await.unwrap();
                    let address_len = match header[3] {
                        1 => 4,
                        4 => 16,
                        3 => stream.read_u8().await.unwrap() as usize,
                        _ => panic!("unexpected SOCKS5 address type"),
                    };
                    stream
                        .read_exact(&mut vec![0; address_len + 2])
                        .await
                        .unwrap();
                    stream
                        .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 80])
                        .await
                        .unwrap();
                    let mut stream = BufReader::new(stream);
                    let mut line = String::new();
                    loop {
                        line.clear();
                        stream.read_line(&mut line).await.unwrap();
                        if line == "\r\n" {
                            break;
                        }
                    }
                    requests.fetch_add(1, Ordering::SeqCst);
                    release.acquire().await.unwrap().forget();
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        )
                        .await
                        .unwrap();
                }
            }
        });
        Self {
            address,
            requests,
            release,
            task,
        }
    }
}

impl Drop for ProbeEndpoint {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub(super) fn latency_and_deletion(runtime: &tokio::runtime::Runtime) {
    let wing = Wing::start();
    let probe = ProbeEndpoint::start(runtime);
    let api = runtime.block_on(async {
        let (api, _) = Api::new(format!("http://{}", wing.address).parse().unwrap())
            .authenticate(&Credentials::new("node-admin", "node-password-42"), true)
            .await
            .unwrap();
        let mut config = api.default_config().await.unwrap();
        config.name = "Local latency check".into();
        config.global.tcp_check_url = vec!["http://127.0.0.1/".into()];
        api.save_config(config, None).await.unwrap();
        api.select_config(api.configs().await.unwrap()[0].id.clone())
            .await
            .unwrap();
        api.import_nodes(vec![
            ImportArgument {
                link: format!("socks5://{}#Local%20endpoint", probe.address),
                tag: Some("Reachable".into()),
            },
            ImportArgument {
                link: "socks5://127.0.0.1:1#Closed%20port".into(),
                tag: Some("Offline".into()),
            },
        ])
        .await
        .unwrap();
        api
    });
    let controller = Nodes::builder().launch(api.clone()).detach();
    let stack = gtk::Stack::new();
    stack.add_named(&gtk::Label::new(Some("Other page")), Some("other"));
    stack.add_named(controller.widget(), Some("nodes"));
    let window = adw::ApplicationWindow::builder()
        .content(&stack)
        .default_width(700)
        .default_height(500)
        .resizable(false)
        .build();
    window.present();
    wait_for(|| find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Reachable").is_some());
    assert_eq!(
        probe.requests.load(Ordering::SeqCst),
        0,
        "hidden node pages must not probe"
    );
    stack.set_visible_child_name("nodes");
    wait_for(|| controller.widget().is_mapped());
    wait_for(|| {
        find_widget::<gtk::Label>(&window, &|l| l.is_mapped() && l.text() == "Testing…").is_some()
    });
    wait_for(|| probe.requests.load(Ordering::SeqCst) == 1);
    capture(&window, "nodes-testing");
    probe.release.add_permits(1);
    wait_for(|| {
        find_widget::<gtk::Label>(&window, &|l| l.is_mapped() && l.text().ends_with(" ms"))
            .is_some()
    });
    let offline = find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Offline").unwrap();
    assert!(find_widget::<gtk::Label>(&offline, &|l| l.text() == "Test failed").is_some());
    let manager = adw::StyleManager::default();
    let original_theme = manager.color_scheme();
    manager.set_color_scheme(adw::ColorScheme::ForceLight);
    wait_for(|| !manager.is_dark());
    capture(&window, "nodes-latency-light");
    manager.set_color_scheme(adw::ColorScheme::ForceDark);
    wait_for(|| manager.is_dark());
    capture(&window, "nodes-latency-dark");
    click_icon(&window, "network-transmit-receive-symbolic");
    wait_for(|| probe.requests.load(Ordering::SeqCst) == 2);
    capture(&window, "nodes-testing-dark");
    probe.release.add_permits(1);
    wait_for(|| {
        find_widget::<gtk::Label>(&window, &|l| l.is_mapped() && l.text().ends_with(" ms"))
            .is_some()
    });
    stack.set_visible_child_name("other");
    wait_for(|| !controller.widget().is_mapped());
    stack.set_visible_child_name("nodes");
    wait_for(|| probe.requests.load(Ordering::SeqCst) == 3);
    probe.release.add_permits(1);
    wait_for(|| {
        find_widget::<gtk::Label>(&window, &|l| l.is_mapped() && l.text().ends_with(" ms"))
            .is_some()
    });
    rust_i18n::set_locale("zh_CN");
    controller.emit(NodeMsg::Retranslate);
    wait_for(|| find_widget::<gtk::Label>(&offline, &|l| l.text() == "测速失败").is_some());
    window.set_default_size(400, 500);
    wait_for(|| window.width() == 400);
    capture(&window, "nodes-latency-narrow-dark");
    manager.set_color_scheme(adw::ColorScheme::ForceLight);
    wait_for(|| !manager.is_dark());
    capture(&window, "nodes-latency-narrow-light");
    rust_i18n::set_locale("en");
    controller.emit(NodeMsg::Retranslate);
    wait_for(|| find_widget::<gtk::Label>(&offline, &|l| l.text() == "Test failed").is_some());
    window.set_default_size(700, 500);
    wait_for(|| window.width() == 700);
    // AdwDialog uses a separate window when its parent is not resizable.
    window.set_resizable(true);
    click_icon(&offline, "user-trash-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Cancel");
    wait_for(|| window.visible_dialog().is_none());
    assert_eq!(runtime.block_on(api.nodes()).unwrap().len(), 2);
    click_icon(&offline, "user-trash-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    capture(&window, "nodes-delete-confirmation");
    click(&window.visible_dialog().unwrap(), "Delete");
    wait_for(|| find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Offline").is_none());
    assert_eq!(runtime.block_on(api.nodes()).unwrap().len(), 1);
    // A concurrent deletion must show the backend error and preserve the row until refresh.
    let remaining = runtime.block_on(api.nodes()).unwrap().remove(0);
    runtime
        .block_on(api.remove_nodes(vec![remaining.id]))
        .unwrap();
    let reachable = find_widget::<adw::ActionRow>(&window, &|r| r.title() == "Reachable").unwrap();
    click_icon(&reachable, "user-trash-symbolic");
    wait_for(|| window.visible_dialog().is_some());
    click(&window.visible_dialog().unwrap(), "Delete");
    wait_for(|| {
        window.visible_dialog().is_some_and(|d| {
            find_widget::<gtk::Label>(&d, &|l| l.text().contains("not found")).is_some()
        })
    });
    assert!(
        reachable.parent().is_some(),
        "failed deletion must retain its row"
    );
    capture(&window, "nodes-delete-error");
    click(&window.visible_dialog().unwrap(), "Close");
    wait_for(|| window.visible_dialog().is_none());
    click(&window, "Try Again");
    wait_for(|| {
        find_widget::<adw::StatusPage>(&window, &|p| p.is_mapped() && p.title() == "No Nodes")
            .is_some()
    });
    manager.set_color_scheme(original_theme);
    window.close();
    drop(controller);
}
