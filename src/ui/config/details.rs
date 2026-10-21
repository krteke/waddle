use adw::prelude::*;
use relm4::{RelmWidgetExt, adw, gtk};
use rust_i18n::t;

use crate::api::config::Config;

pub struct Details {
    pub page: adw::NavigationPage,
    body: gtk::Box,
}

impl Details {
    pub fn new() -> Self {
        let body = gtk::Box::new(gtk::Orientation::Vertical, 24);
        body.set_margin_all(24);

        let clamp = adw::Clamp::builder().maximum_size(900).child(&body).build();

        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&clamp)
            .build();

        let toolbar = adw::ToolbarView::new();

        toolbar.add_top_bar(
            &adw::HeaderBar::builder()
                .show_start_title_buttons(false)
                .show_end_title_buttons(false)
                .build(),
        );

        toolbar.set_content(Some(&scroll));

        let page = adw::NavigationPage::builder()
            .tag("config-details")
            .title("")
            .child(&toolbar)
            .build();

        Self { page, body }
    }

    pub fn refresh(&self, config: &Config) {
        self.page.set_title(&config.name);

        while let Some(child) = self.body.first_child() {
            self.body.remove(&child);
        }

        let g = &config.global;

        self.section(
            "config.software",
            &[
                ("config.log_level", g.log_level.clone()),
                ("config.tproxy_port", g.tproxy_port.to_string()),
                ("config.tproxy_port_protect", enabled(g.tproxy_port_protect)),
                ("config.so_mark_from_dae", g.so_mark_from_dae.to_string()),
                ("config.pprof_port", g.pprof_port.to_string()),
            ],
        );

        self.section(
            "config.interfaces",
            &[
                ("config.lan_interface", g.lan_interface.join("\n")),
                ("config.wan_interface", g.wan_interface.join("\n")),
                (
                    "config.auto_config_kernel_parameter",
                    enabled(g.auto_config_kernel_parameter),
                ),
                (
                    "config.auto_config_firewall_rule",
                    enabled(g.auto_config_firewall_rule),
                ),
                ("config.disable_thp", enabled(g.disable_thp)),
                ("config.mptcp", enabled(g.mptcp)),
                (
                    "config.enable_local_tcp_fast_redirect",
                    enabled(g.enable_local_tcp_fast_redirect),
                ),
                (
                    "config.bpf_conn_state_map_size",
                    g.bpf_conn_state_map_size.to_string(),
                ),
            ],
        );

        self.section(
            "config.connection",
            &[
                ("config.dial_mode", g.dial_mode.clone()),
                ("config.allow_insecure", enabled(g.allow_insecure)),
                (
                    "config.disable_waiting_network",
                    enabled(g.disable_waiting_network),
                ),
                ("config.udphop_interval", g.udphop_interval.clone()),
            ],
        );

        self.section(
            "config.connectivity",
            &[
                ("config.check_interval", g.check_interval.clone()),
                ("config.check_tolerance", g.check_tolerance.clone()),
                ("config.sniffing_timeout", g.sniffing_timeout.clone()),
                ("config.auto_sniff_punt", enabled(g.auto_sniff_punt)),
                (
                    "config.tcp_check_http_method",
                    g.tcp_check_http_method.clone(),
                ),
                ("config.tcp_check_url", g.tcp_check_url.join("\n")),
                ("config.udp_check_dns", g.udp_check_dns.join("\n")),
            ],
        );

        self.section(
            "config.tls",
            &[
                ("config.tls_implementation", g.tls_implementation.clone()),
                ("config.utls_imitate", g.utls_imitate.clone()),
                ("config.tls_fragment", enabled(g.tls_fragment)),
                ("config.tls_fragment_length", g.tls_fragment_length.clone()),
                (
                    "config.tls_fragment_interval",
                    g.tls_fragment_interval.clone(),
                ),
            ],
        );

        self.section(
            "navigation.dns",
            &[
                ("config.bootstrap_resolver", g.bootstrap_resolver.clone()),
                ("config.fallback_resolver", g.fallback_resolver.clone()),
            ],
        );

        self.section(
            "config.bandwidth",
            &[
                ("config.bandwidth_max_tx", g.bandwidth_max_tx.clone()),
                ("config.bandwidth_max_rx", g.bandwidth_max_rx.clone()),
            ],
        );
    }

    fn section(&self, title: &str, values: &[(&str, String)]) {
        let group = adw::PreferencesGroup::builder()
            .title(t!(title).as_ref())
            .build();

        values.iter().for_each(|(key, value)| {
            let row = adw::ActionRow::builder()
                .title(t!(*key).as_ref())
                .use_markup(false)
                .build();

            let value = gtk::Label::builder()
                .label(if value.is_empty() { "—" } else { value })
                .use_markup(false)
                .selectable(true)
                .wrap(true)
                .wrap_mode(gtk::pango::WrapMode::WordChar)
                .width_chars(16)
                .max_width_chars(32)
                .xalign(1.0)
                .justify(gtk::Justification::Right)
                .valign(gtk::Align::Center)
                .build();

            row.add_suffix(&value);
            group.add(&row);
        });

        self.body.append(&group);
    }
}

fn enabled(value: bool) -> String {
    if value {
        t!("config.enabled")
    } else {
        t!("config.disabled")
    }
    .to_string()
}
