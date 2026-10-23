use super::{
    widgets::{Choice, Widgets},
    *,
};
use crate::node::{TcpHeader, Transport, TrojanTransport};

pub struct HostPath {
    group: adw::PreferencesGroup,
    host: adw::EntryRow,
    path: adw::EntryRow,
}

impl HostPath {
    fn new(w: &mut Widgets) -> Self {
        let group = w.group("");
        let host = w.entry(&group, "node_editor.http_host", "", false);
        let path = w.entry(&group, "node_editor.path", "", false);

        Self { group, host, path }
    }

    fn fill(&self, host: &str, path: &str) {
        self.host.set_text(host);
        self.path.set_text(path);
    }
}

pub struct TransportFields {
    network: Choice,
    tcp: adw::PreferencesGroup,
    header: Choice,
    tcp_http: HostPath,
    websocket: HostPath,
    h2: HostPath,
    grpc: adw::PreferencesGroup,
    service_name: adw::EntryRow,
    upgrade: HostPath,
}

impl TransportFields {
    pub fn new(value: &Transport, w: &mut Widgets) -> Self {
        let group = w.group("transport");

        let index = match value {
            Transport::Tcp { .. } => 0,
            Transport::WebSocket { .. } => 1,
            Transport::H2 { .. } => 2,
            Transport::Grpc { .. } => 3,
            Transport::HttpUpgrade { .. } => 4,
        };

        let network = w.selection(
            &group,
            "node_editor.network",
            &["TCP", "WebSocket", "HTTP/2", "gRPC", "HTTP Upgrade"],
            index,
        );

        let tcp = w.group("");

        let header = w.selection(
            &tcp,
            "node_editor.header_type",
            &["", "HTTP"],
            u32::from(matches!(
                value,
                Transport::Tcp {
                    header: TcpHeader::Http { .. }
                }
            )),
        );

        let tcp_http = HostPath::new(w);
        let websocket = HostPath::new(w);
        let h2 = HostPath::new(w);

        let grpc = w.group("");
        let service_name = w.entry(&grpc, "node_editor.service_name", "", false);
        let upgrade = HostPath::new(w);

        match value {
            Transport::Tcp {
                header: TcpHeader::Http { host, path },
            } => tcp_http.fill(host, path),
            Transport::Tcp {
                header: TcpHeader::None,
            } => {}
            Transport::WebSocket { host, path } => websocket.fill(host, path),
            Transport::H2 { host, path } => h2.fill(host, path),
            Transport::Grpc {
                service_name: value,
            } => service_name.set_text(value),
            Transport::HttpUpgrade { host, path } => upgrade.fill(host, path),
        }

        Self {
            network,
            tcp,
            header,
            tcp_http,
            websocket,
            h2,
            grpc,
            service_name,
            upgrade,
        }
    }

    pub fn read(&self) -> Transport {
        match self.network.selected() {
            0 => Transport::Tcp {
                header: match self.header.selected() {
                    0 => TcpHeader::None,
                    1 => TcpHeader::Http {
                        host: self.tcp_http.host.text().into(),
                        path: self.tcp_http.path.text().into(),
                    },
                    _ => unreachable!(),
                },
            },
            1 => Transport::WebSocket {
                host: self.websocket.host.text().into(),
                path: self.websocket.path.text().into(),
            },
            2 => Transport::H2 {
                host: self.h2.host.text().into(),
                path: self.h2.path.text().into(),
            },
            3 => Transport::Grpc {
                service_name: self.service_name.text().into(),
            },
            4 => Transport::HttpUpgrade {
                host: self.upgrade.host.text().into(),
                path: self.upgrade.path.text().into(),
            },
            _ => unreachable!("transport selection is defined by its model"),
        }
    }

    pub fn update_visibility(&self) {
        let index = self.network.selected();

        self.tcp.set_visible(index == 0);
        self.tcp_http
            .group
            .set_visible(index == 0 && self.header.selected() == 1);

        self.websocket.group.set_visible(index == 1);
        self.h2.group.set_visible(index == 2);

        self.grpc.set_visible(index == 3);
        self.upgrade.group.set_visible(index == 4);
    }
}

pub struct TrojanTransportFields {
    network: Choice,
    websocket: HostPath,
    grpc: adw::PreferencesGroup,
    service_name: adw::EntryRow,
    upgrade: HostPath,
}

impl TrojanTransportFields {
    pub fn new(value: &TrojanTransport, w: &mut Widgets) -> Self {
        let group = w.group("transport");

        let index = match value {
            TrojanTransport::Original => 0,
            TrojanTransport::WebSocket { .. } => 1,
            TrojanTransport::Grpc { .. } => 2,
            TrojanTransport::HttpUpgrade { .. } => 3,
        };

        let network = w.selection(
            &group,
            "node_editor.network",
            &["Original", "WebSocket", "gRPC", "HTTP Upgrade"],
            index,
        );

        let websocket = HostPath::new(w);

        let grpc = w.group("");
        let service_name = w.entry(&grpc, "node_editor.service_name", "", false);

        let upgrade = HostPath::new(w);

        match value {
            TrojanTransport::Original => {}
            TrojanTransport::WebSocket { host, path } => websocket.fill(host, path),
            TrojanTransport::Grpc {
                service_name: value,
            } => service_name.set_text(value),
            TrojanTransport::HttpUpgrade { host, path } => upgrade.fill(host, path),
        }

        Self {
            network,
            websocket,
            grpc,
            service_name,
            upgrade,
        }
    }

    pub fn read(&self) -> TrojanTransport {
        match self.network.selected() {
            0 => TrojanTransport::Original,
            1 => TrojanTransport::WebSocket {
                host: self.websocket.host.text().into(),
                path: self.websocket.path.text().into(),
            },
            2 => TrojanTransport::Grpc {
                service_name: self.service_name.text().into(),
            },
            3 => TrojanTransport::HttpUpgrade {
                host: self.upgrade.host.text().into(),
                path: self.upgrade.path.text().into(),
            },
            _ => unreachable!(),
        }
    }

    pub fn update_visibility(&self) {
        self.websocket
            .group
            .set_visible(self.network.selected() == 1);

        self.grpc.set_visible(self.network.selected() == 2);
        self.upgrade.group.set_visible(self.network.selected() == 3);
    }
}
