use crate::api::node::NodeLatency;
use adw::prelude::*;
use relm4::{adw, gtk};
use rust_i18n::t;

pub enum Latency {
    Untested,
    Testing,
    Measured(NodeLatency),
    RequestFailed,
}

pub struct LatencyIndicator {
    pub widget: gtk::Box,
    spinner: adw::Spinner,
    icon: gtk::Image,
    label: gtk::Label,
}

impl LatencyIndicator {
    pub fn new() -> Self {
        let status = gtk::Box::builder()
            .spacing(6)
            .halign(gtk::Align::End)
            .valign(gtk::Align::Center)
            .build();

        let spinner = adw::Spinner::builder()
            .width_request(16)
            .height_request(16)
            .build();

        let icon = gtk::Image::new();
        let label = gtk::Label::builder()
            .css_classes(["caption", "numeric"])
            .build();

        status.append(&spinner);
        status.append(&icon);
        status.append(&label);

        Self {
            widget: status,
            spinner,
            icon,
            label,
        }
    }

    pub fn update(&self, latency: &Latency) {
        let testing = matches!(latency, Latency::Testing);

        self.spinner.set_visible(testing);
        self.icon.set_visible(!testing);

        let (text, icon, class) = match latency {
            Latency::Untested => (
                t!("nodes.untested"),
                "content-loading-symbolic",
                "dim-label",
            ),
            Latency::Testing => (t!("nodes.testing"), "", "accent"),
            Latency::Measured(result) if result.alive => (
                result
                    .latency_ms
                    .map(|ms| t!("nodes.latency", ms = ms))
                    .unwrap_or_else(|| t!("nodes.available")),
                "object-select-symbolic",
                "success",
            ),
            Latency::Measured(_) | Latency::RequestFailed => {
                (t!("nodes.test_failed"), "dialog-error-symbolic", "error")
            }
        };

        self.label.set_label(&text);

        self.icon.set_icon_name(Some(icon));
        self.icon.set_css_classes(&[class]);

        self.spinner.set_css_classes(&[class]);

        self.widget.set_tooltip_text(match latency {
            Latency::Measured(result) => result.message.as_deref(),
            _ => None,
        });
    }
}
