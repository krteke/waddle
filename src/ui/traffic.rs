mod chart;

use adw::prelude::*;
use relm4::{adw, gtk, prelude::*};
use rust_i18n::t;
use std::time::Duration;

use crate::{
    api::{Api, traffic::TrafficStats},
    error::ApiError,
};
use chart::Chart;

const RANGES: [(i64, i64); 4] = [(60, 120), (600, 240), (1800, 360), (3600, 480)];

pub struct Traffic {
    api: Api,
    stats: Option<TrafficStats>,
    range: usize,
    busy: bool,
    failed: bool,
    timer: Option<gtk::glib::SourceId>,
    upload: Chart,
    download: Chart,
}

#[derive(Debug)]
pub enum TrafficMsg {
    Start,
    Stop,
    Refresh,
    Range(usize),
    Retranslate,
}

#[relm4::component(pub)]
impl Component for Traffic {
    type Init = Api;
    type Input = TrafficMsg;
    type Output = ();
    type CommandOutput = (usize, Result<TrafficStats, ApiError>);

    view! {
        adw::PreferencesGroup {
            #[watch]
            set_title: &t!("traffic.title"),
            connect_map => TrafficMsg::Start,
            connect_unmap => TrafficMsg::Stop,
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                #[name = "ranges"]
                adw::ToggleGroup {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_tooltip_text: Some(t!("traffic.range").as_ref()),
                    #[wrap(Clone::clone)]
                    add = &adw::Toggle {
                        #[watch]
                        set_label: Some(t!("traffic.range_1m").as_ref()),
                    },
                    #[wrap(Clone::clone)]
                    add = &adw::Toggle {
                        #[watch]
                        set_label: Some(t!("traffic.range_10m").as_ref()),
                    },
                    #[wrap(Clone::clone)]
                    add = &adw::Toggle {
                        #[watch]
                        set_label: Some(t!("traffic.range_30m").as_ref()),
                    },
                    #[wrap(Clone::clone)]
                    add = &adw::Toggle {
                        #[watch]
                        set_label: Some(t!("traffic.range_1h").as_ref()),
                    },
                    connect_active_notify[sender] => move |group| {
                        sender.input(TrafficMsg::Range(group.active() as usize));
                    },
                },
                gtk::Label {
                    set_xalign: 0.0,
                    set_wrap: true,
                    #[watch]
                    set_css_classes: if model.failed { &["caption", "error"] } else { &["caption", "dim-label"] },
                    #[watch]
                    set_visible: model.stats.is_none(),
                    #[watch]
                    set_label: if model.failed { t!("traffic.unavailable") } else { t!("traffic.loading") }.as_ref(),
                },
                #[local_ref]
                charts -> gtk::FlowBox {},
                adw::PreferencesGroup {
                    adw::ActionRow {
                        #[watch]
                        set_title: &t!("traffic.connections"),
                        add_prefix = &gtk::Image::from_icon_name("network-wired-symbolic"),
                        add_suffix = &gtk::Label {
                            add_css_class: "numeric",
                            #[watch]
                            set_label: &model.stats.as_ref().map(|s| s.active_connections.to_string()).unwrap_or_else(|| "—".into()),
                        },
                    },
                    adw::ActionRow {
                        #[watch]
                        set_title: &t!("traffic.udp"),
                        add_prefix = &gtk::Image::from_icon_name("network-transmit-receive-symbolic"),
                        add_suffix = &gtk::Label {
                            add_css_class: "numeric",
                            #[watch]
                            set_label: &model.stats.as_ref().map(|s| s.udp_sessions.to_string()).unwrap_or_else(|| "—".into()),
                        },
                    },
                },
            },
        }
    }

    fn init(api: Api, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self {
            api,
            stats: None,
            range: 0,
            busy: false,
            failed: false,
            timer: None,
            upload: Chart::new(true),
            download: Chart::new(false),
        };
        let charts = gtk::FlowBox::builder()
            .homogeneous(true)
            .selection_mode(gtk::SelectionMode::None)
            .min_children_per_line(1)
            .max_children_per_line(2)
            .column_spacing(12)
            .row_spacing(12)
            .build();
        for chart in [&model.upload, &model.download] {
            let child = gtk::FlowBoxChild::builder()
                .child(&chart.widget)
                .focusable(false)
                .build();
            charts.append(&child);
        }
        model.update_charts();
        let widgets = view_output!();
        widgets
            .ranges
            .upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Label(&t!("traffic.range"))]);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: TrafficMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            TrafficMsg::Start => {
                self.start_timer(&sender);
                sender.input(TrafficMsg::Refresh);
            }
            TrafficMsg::Stop => self.stop_timer(),
            TrafficMsg::Refresh => {
                if self.busy || !root.is_mapped() {
                    return;
                }
                self.busy = true;
                let api = self.api.clone();
                let range = self.range;
                let (seconds, points) = RANGES[range];
                sender.oneshot_command(async move { (range, api.traffic(seconds, points).await) });
            }
            TrafficMsg::Range(range) => {
                if self.range == range {
                    return;
                }
                self.range = range;
                self.stats = None;
                self.failed = false;
                self.update_charts();
                if root.is_mapped() {
                    self.start_timer(&sender);
                    sender.input(TrafficMsg::Refresh);
                }
            }
            TrafficMsg::Retranslate => self.update_charts(),
        }
    }

    fn update_cmd(
        &mut self,
        (range, result): Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.busy = false;
        if range != self.range {
            sender.input(TrafficMsg::Refresh);
            return;
        }
        match result {
            Ok(stats) => {
                self.stats = Some(stats);
                self.failed = false;
            }
            Err(ApiError::Unauthorized) => {
                self.stop_timer();
                self.stats = None;
                sender.output(()).unwrap();
            }
            Err(error) => {
                if !self.failed {
                    tracing::warn!(%error, "Could not load traffic statistics");
                }
                self.failed = true;
                self.stats = None;
            }
        }
        self.update_charts();
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        self.stop_timer();
    }
}

impl Traffic {
    fn update_charts(&self) {
        self.upload
            .update(self.stats.as_ref(), RANGES[self.range].0);
        self.download
            .update(self.stats.as_ref(), RANGES[self.range].0);
    }

    fn start_timer(&mut self, sender: &ComponentSender<Self>) {
        self.stop_timer();
        let interval = if self.range == 0 { 500 } else { 5000 };
        let sender = sender.input_sender().clone();
        self.timer = Some(gtk::glib::timeout_add_local(
            Duration::from_millis(interval),
            move || {
                sender.emit(TrafficMsg::Refresh);
                gtk::glib::ControlFlow::Continue
            },
        ));
    }

    fn stop_timer(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
    }
}
