use adw::prelude::*;
use relm4::{RelmWidgetExt, adw, gtk};
use rust_i18n::t;

use crate::api::traffic::TrafficStats;

pub struct Chart {
    pub widget: gtk::Box,
    title: gtk::Label,
    rate: gtk::Label,
    total: gtk::Label,
    ceiling: gtk::Label,
    start: gtk::Label,
    end: gtk::Label,
    area: gtk::DrawingArea,
    upload: bool,
}

impl Chart {
    pub fn new(upload: bool) -> Self {
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 0);
        widget.add_css_class("card");
        widget.set_overflow(gtk::Overflow::Hidden);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
        content.set_margin_all(16);
        widget.append(&content);
        let heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        heading.append(&gtk::Image::from_icon_name(if upload {
            "go-up-symbolic"
        } else {
            "go-down-symbolic"
        }));
        let title = gtk::Label::builder()
            .xalign(0.0)
            .css_classes(["heading"])
            .build();
        heading.append(&title);
        content.append(&heading);
        let rate = gtk::Label::builder()
            .xalign(0.0)
            .css_classes(["title-1", "numeric"])
            .build();
        content.append(&rate);
        let ceiling = gtk::Label::builder()
            .xalign(0.0)
            .css_classes(["caption", "dim-label", "numeric"])
            .build();
        content.append(&ceiling);
        let area = gtk::DrawingArea::builder()
            .content_width(220)
            .content_height(120)
            .hexpand(true)
            .accessible_role(gtk::AccessibleRole::Img)
            .css_classes(["accent"])
            .build();
        content.append(&area);
        let axis = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let start = gtk::Label::builder()
            .xalign(0.0)
            .hexpand(true)
            .css_classes(["caption", "dim-label", "numeric"])
            .build();
        let end = gtk::Label::builder()
            .xalign(1.0)
            .css_classes(["caption", "dim-label", "numeric"])
            .build();
        axis.append(&start);
        axis.append(&end);
        content.append(&axis);
        let total = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["caption", "dim-label", "numeric"])
            .build();
        content.append(&total);
        Self {
            widget,
            title,
            rate,
            total,
            ceiling,
            start,
            end,
            area,
            upload,
        }
    }

    pub fn update(&self, stats: Option<&TrafficStats>, window_sec: i64) {
        let title = if self.upload {
            t!("traffic.upload")
        } else {
            t!("traffic.download")
        };
        self.title.set_label(&title);
        let total = if self.upload {
            "traffic.upload_total"
        } else {
            "traffic.download_total"
        };
        let Some(stats) = stats else {
            self.rate.set_label("—");
            self.total.set_label(&t!(total, value = "—"));
            self.ceiling.set_label("—");
            self.start.set_label("—");
            self.end.set_label("—");
            self.area.unset_draw_func();
            self.area
                .update_property(&[gtk::accessible::Property::Label(&title)]);
            self.area.queue_draw();
            return;
        };
        let rate = if self.upload {
            stats.upload_rate
        } else {
            stats.download_rate
        };
        let bytes = if self.upload {
            stats.upload_total
        } else {
            stats.download_total
        };
        let rate = format!("{}/s", format_bytes(rate));
        self.rate.set_label(&rate);
        self.total
            .set_label(&t!(total, value = format_bytes(bytes as f64)));
        self.area
            .update_property(&[gtk::accessible::Property::Label(&format!(
                "{title}: {rate}"
            ))]);
        let end = stats.updated_at;
        let start = end - window_sec as f64;
        let time_label = |time: f64| {
            gtk::glib::DateTime::from_unix_local(time as i64)
                .unwrap()
                .format("%H:%M:%S")
                .unwrap()
        };
        self.start.set_label(&time_label(start));
        self.end.set_label(&time_label(end));
        let points: Vec<_> = stats
            .samples
            .iter()
            .map(|s| {
                (
                    s.timestamp,
                    if self.upload {
                        s.upload_rate
                    } else {
                        s.download_rate
                    },
                )
            })
            .collect();
        let peak = points.iter().map(|p| p.1).fold(1024.0, f64::max);
        self.ceiling.set_label(&format!("{}/s", format_bytes(peak)));
        self.area.set_draw_func(move |area, cr, width, height| {
            let color = area.color();
            let (r, g, b) = (
                color.red() as f64,
                color.green() as f64,
                color.blue() as f64,
            );
            let width = width as f64;
            let height = height as f64 - 2.0;
            cr.set_line_width(1.0);
            cr.set_source_rgba(r, g, b, 0.15);
            for fraction in [0.0, 0.5, 1.0] {
                let y = 1.0 + fraction * height;
                cr.move_to(0.0, y);
                cr.line_to(width, y);
            }
            cr.stroke().expect("traffic chart grid");
            if points.is_empty() {
                return;
            }
            let x = |time: f64| (time - start) / (end - start) * width;
            let y = |rate: f64| 1.0 + height * (1.0 - rate / peak);
            cr.rectangle(0.0, 0.0, width, height + 2.0);
            cr.clip();
            cr.move_to(x(points[0].0), y(points[0].1));
            for &(time, rate) in &points[1..] {
                cr.line_to(x(time), y(rate));
            }
            cr.set_source_rgba(r, g, b, 1.0);
            cr.set_line_width(2.0);
            cr.stroke_preserve().expect("traffic chart line");
            let last = points.last().unwrap();
            cr.line_to(x(last.0), height + 1.0);
            cr.line_to(x(points[0].0), height + 1.0);
            cr.close_path();
            cr.set_source_rgba(r, g, b, 0.12);
            cr.fill().expect("traffic chart area");
            cr.arc(x(last.0), y(last.1), 2.5, 0.0, std::f64::consts::TAU);
            cr.set_source_rgba(r, g, b, 1.0);
            cr.fill().expect("traffic chart latest point");
        });
        self.area.queue_draw();
    }
}

fn format_bytes(bytes: f64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    let mut value = bytes;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 || value >= 10.0 {
        format!("{value:.0} {}", units[unit])
    } else {
        format!("{value:.1} {}", units[unit])
    }
}
