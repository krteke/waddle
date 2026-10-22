use super::{
    choice_list::ChoiceList, editor::EditorMsg, field_note::FieldNote, input_list::InputList,
};
use crate::api::config::Config;
use adw::prelude::*;
use relm4::{Sender, adw, gtk};
use rust_i18n::t;

pub struct Fields {
    pub page: adw::PreferencesPage,

    name: adw::EntryRow,
    notes: Vec<FieldNote>,
    groups: Vec<(&'static str, adw::PreferencesGroup)>,
    log_level: adw::ComboRow,

    tproxy_port: adw::SpinRow,
    tproxy_port_protect: adw::SwitchRow,

    so_mark_from_dae: adw::SpinRow,
    pprof_port: adw::SpinRow,

    lan_interface: InputList,
    wan_interface: InputList,

    auto_config_kernel_parameter: adw::SwitchRow,
    auto_config_firewall_rule: adw::SwitchRow,

    disable_thp: adw::SwitchRow,
    mptcp: adw::SwitchRow,
    enable_local_tcp_fast_redirect: adw::SwitchRow,
    bpf_conn_state_map_size: adw::SpinRow,
    dial_mode: ChoiceList,
    allow_insecure: adw::SwitchRow,
    disable_waiting_network: adw::SwitchRow,
    udphop_interval: adw::EntryRow,

    check_interval: adw::EntryRow,
    check_tolerance: adw::EntryRow,

    sniffing_timeout: adw::EntryRow,
    auto_sniff_punt: adw::SwitchRow,

    tcp_check_http_method: adw::ComboRow,
    tcp_check_url: InputList,
    udp_check_dns: InputList,

    tls_implementation: ChoiceList,
    utls_imitate: adw::ComboRow,

    tls_fragment: adw::SwitchRow,
    tls_fragment_length: adw::EntryRow,
    tls_fragment_interval: adw::EntryRow,

    bootstrap_resolver: adw::EntryRow,
    fallback_resolver: adw::EntryRow,

    bandwidth_max_tx: adw::EntryRow,
    bandwidth_max_rx: adw::EntryRow,
}

impl Fields {
    pub fn new(sender: &Sender<EditorMsg>) -> Self {
        let mut notes = Vec::new();
        let mut groups = Vec::new();

        let name = adw::EntryRow::new();
        let identity = adw::PreferencesGroup::new();
        let note = FieldNote::new(&name, "name", true);

        let group = adw::PreferencesGroup::new();
        let page = adw::PreferencesPage::new();

        note.add_to(&identity);
        notes.push(note);
        page.add(&identity);

        let log_level = adw::ComboRow::builder()
            .model(&gtk::StringList::new(&[
                "error", "warn", "info", "debug", "trace",
            ]))
            .use_markup(false)
            .build();

        group.add(&log_level);

        let tproxy_port = adw::SpinRow::with_range(0.0, 65535.0, 1.0);
        tproxy_port.set_numeric(true);

        let note = FieldNote::new(&tproxy_port, "tproxy_port", true);
        note.add_to(&group);
        notes.push(note);

        let tproxy_port_protect = adw::SwitchRow::new();
        group.add(&tproxy_port_protect);

        let so_mark_from_dae = adw::SpinRow::with_range(i32::MIN as f64, i32::MAX as f64, 1.0);
        so_mark_from_dae.set_numeric(true);

        let note = FieldNote::new(&so_mark_from_dae, "so_mark_from_dae", true);
        note.add_to(&group);
        notes.push(note);

        let pprof_port = adw::SpinRow::with_range(0.0, 65535.0, 1.0);
        pprof_port.set_numeric(true);

        let note = FieldNote::new(&pprof_port, "pprof_port", true);

        note.add_to(&group);
        notes.push(note);

        page.add(&group);
        groups.push(("config.software", group));

        let group = adw::PreferencesGroup::new();

        let lan_interface = InputList::new("lan_interface", false, false, sender);
        let wan_interface = InputList::new("wan_interface", false, false, sender);

        let auto_config_kernel_parameter = adw::SwitchRow::new();
        group.add(&auto_config_kernel_parameter);

        let auto_config_firewall_rule = adw::SwitchRow::new();
        group.add(&auto_config_firewall_rule);

        let disable_thp = adw::SwitchRow::new();
        group.add(&disable_thp);

        let mptcp = adw::SwitchRow::new();
        group.add(&mptcp);

        let enable_local_tcp_fast_redirect = adw::SwitchRow::new();
        group.add(&enable_local_tcp_fast_redirect);

        let bpf_conn_state_map_size = adw::SpinRow::with_range(0.0, i32::MAX as f64, 1.0);
        bpf_conn_state_map_size.set_numeric(true);

        let note = FieldNote::new(&bpf_conn_state_map_size, "bpf_conn_state_map_size", true);

        note.add_to(&group);
        notes.push(note);
        page.add(&group);

        groups.push(("config.interfaces", group));
        page.add(lan_interface.group());
        page.add(wan_interface.group());

        let group = adw::PreferencesGroup::new();

        let dial_mode = ChoiceList::new(
            "dial_mode",
            &["ip", "domain", "domain+", "domain++"],
            sender,
        );

        page.add(&dial_mode.group);

        let allow_insecure = adw::SwitchRow::new();
        group.add(&allow_insecure);

        let disable_waiting_network = adw::SwitchRow::new();
        group.add(&disable_waiting_network);

        let udphop_interval = adw::EntryRow::new();

        let note = FieldNote::new(&udphop_interval, "udphop_interval", true);
        note.add_to(&group);
        notes.push(note);

        page.add(&group);
        groups.push(("config.connection", group));

        let group = adw::PreferencesGroup::new();
        let check_interval = adw::EntryRow::new();

        let note = FieldNote::new(&check_interval, "check_interval", true);
        note.add_to(&group);
        notes.push(note);

        let check_tolerance = adw::EntryRow::new();

        let note = FieldNote::new(&check_tolerance, "check_tolerance", true);
        note.add_to(&group);
        notes.push(note);

        let sniffing_timeout = adw::EntryRow::new();

        let note = FieldNote::new(&sniffing_timeout, "sniffing_timeout", true);
        note.add_to(&group);
        notes.push(note);

        let auto_sniff_punt = adw::SwitchRow::new();
        group.add(&auto_sniff_punt);

        let tcp_check_http_method = adw::ComboRow::builder()
            .model(&gtk::StringList::new(&[
                "HEAD", "GET", "CONNECT", "OPTIONS", "TRACE", "POST", "DELETE", "PATCH", "PUT",
            ]))
            .use_markup(false)
            .build();

        group.add(&tcp_check_http_method);

        let tcp_check_url = InputList::new("tcp_check_url", true, true, sender);
        let udp_check_dns = InputList::new("udp_check_dns", true, true, sender);

        page.add(&group);
        groups.push(("config.connectivity", group));

        page.add(tcp_check_url.group());
        page.add(udp_check_dns.group());

        let group = adw::PreferencesGroup::new();

        let tls_implementation = ChoiceList::new("tls_implementation", &["tls", "utls"], sender);
        page.add(&tls_implementation.group);

        let utls_imitate = adw::ComboRow::builder()
            .model(&gtk::StringList::new(&[
                "randomized",
                "randomizedalpn",
                "randomizednoalpn",
                "firefox_auto",
                "firefox_55",
                "firefox_56",
                "firefox_63",
                "firefox_65",
                "firefox_99",
                "firefox_102",
                "firefox_105",
                "chrome_auto",
                "chrome_58",
                "chrome_62",
                "chrome_70",
                "chrome_72",
                "chrome_83",
                "chrome_87",
                "chrome_96",
                "chrome_100",
                "chrome_102",
                "ios_auto",
                "ios_11_1",
                "ios_12_1",
                "ios_13",
                "ios_14",
                "android_11_okhttp",
                "edge_auto",
                "edge_85",
                "edge_106",
                "safari_auto",
                "safari_16_0",
                "360_auto",
                "360_7_5",
                "360_11_0",
                "qq_auto",
                "qq_11_1",
            ]))
            .use_markup(false)
            .build();

        group.add(&utls_imitate);

        let tls_fragment = adw::SwitchRow::new();
        group.add(&tls_fragment);

        let tls_fragment_length = adw::EntryRow::new();

        let note = FieldNote::new(&tls_fragment_length, "tls_fragment_length", false);
        note.add_to(&group);
        notes.push(note);

        let tls_fragment_interval = adw::EntryRow::new();

        let note = FieldNote::new(&tls_fragment_interval, "tls_fragment_interval", false);

        note.add_to(&group);
        notes.push(note);

        page.add(&group);
        groups.push(("config.tls", group));

        let group = adw::PreferencesGroup::new();
        let bootstrap_resolver = adw::EntryRow::new();

        let note = FieldNote::new(&bootstrap_resolver, "bootstrap_resolver", false);
        note.add_to(&group);
        notes.push(note);

        let fallback_resolver = adw::EntryRow::new();

        let note = FieldNote::new(&fallback_resolver, "fallback_resolver", false);
        note.add_to(&group);
        notes.push(note);

        page.add(&group);
        groups.push(("navigation.dns", group));

        let group = adw::PreferencesGroup::new();
        let bandwidth_max_tx = adw::EntryRow::new();

        let note = FieldNote::new(&bandwidth_max_tx, "bandwidth_max_tx", false);
        note.add_to(&group);
        notes.push(note);

        let bandwidth_max_rx = adw::EntryRow::new();

        let note = FieldNote::new(&bandwidth_max_rx, "bandwidth_max_rx", false);
        note.add_to(&group);
        notes.push(note);

        page.add(&group);
        groups.push(("config.bandwidth", group));

        let fields = Self {
            page,
            notes,
            name,
            groups,
            log_level,
            tproxy_port,
            tproxy_port_protect,
            so_mark_from_dae,
            pprof_port,
            lan_interface,
            wan_interface,
            auto_config_kernel_parameter,
            auto_config_firewall_rule,
            disable_thp,
            mptcp,
            enable_local_tcp_fast_redirect,
            bpf_conn_state_map_size,
            dial_mode,
            allow_insecure,
            disable_waiting_network,
            udphop_interval,
            check_interval,
            check_tolerance,
            sniffing_timeout,
            auto_sniff_punt,
            tcp_check_http_method,
            tcp_check_url,
            udp_check_dns,
            tls_implementation,
            utls_imitate,
            tls_fragment,
            tls_fragment_length,
            tls_fragment_interval,
            bootstrap_resolver,
            fallback_resolver,
            bandwidth_max_tx,
            bandwidth_max_rx,
        };

        fields.notes.iter().for_each(|note| {
            let sender = sender.clone();
            note.editable
                .connect_changed(move |_| sender.emit(EditorMsg::Changed));
        });

        let changed = sender.clone();

        fields
            .tproxy_port_protect
            .connect_active_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .auto_config_kernel_parameter
            .connect_active_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .auto_config_firewall_rule
            .connect_active_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .disable_thp
            .connect_active_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .mptcp
            .connect_active_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .enable_local_tcp_fast_redirect
            .connect_active_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .allow_insecure
            .connect_active_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .disable_waiting_network
            .connect_active_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .auto_sniff_punt
            .connect_active_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .tls_fragment
            .connect_active_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .log_level
            .connect_selected_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .tcp_check_http_method
            .connect_selected_notify(move |_| changed.emit(EditorMsg::Changed));

        let changed = sender.clone();

        fields
            .utls_imitate
            .connect_selected_notify(move |_| changed.emit(EditorMsg::Changed));

        fields.retranslate();

        fields
    }

    pub fn load(&self, config: &Config) {
        self.name.set_text(&config.name);

        let g = &config.global;

        set_choice(&self.log_level, &g.log_level);

        self.tproxy_port.set_value(g.tproxy_port as f64);
        self.tproxy_port_protect.set_active(g.tproxy_port_protect);

        self.so_mark_from_dae.set_value(g.so_mark_from_dae as f64);
        self.pprof_port.set_value(g.pprof_port as f64);

        self.lan_interface.load(&g.lan_interface);
        self.wan_interface.load(&g.wan_interface);

        self.auto_config_kernel_parameter
            .set_active(g.auto_config_kernel_parameter);
        self.auto_config_firewall_rule
            .set_active(g.auto_config_firewall_rule);

        self.disable_thp.set_active(g.disable_thp);
        self.mptcp.set_active(g.mptcp);

        self.enable_local_tcp_fast_redirect
            .set_active(g.enable_local_tcp_fast_redirect);
        self.bpf_conn_state_map_size
            .set_value(g.bpf_conn_state_map_size as f64);

        self.dial_mode.load(&g.dial_mode);

        self.allow_insecure.set_active(g.allow_insecure);
        self.disable_waiting_network
            .set_active(g.disable_waiting_network);

        self.udphop_interval.set_text(&g.udphop_interval);
        self.check_interval.set_text(&g.check_interval);
        self.check_tolerance.set_text(&g.check_tolerance);

        self.sniffing_timeout.set_text(&g.sniffing_timeout);
        self.auto_sniff_punt.set_active(g.auto_sniff_punt);

        set_choice(&self.tcp_check_http_method, &g.tcp_check_http_method);

        self.tcp_check_url.load(&g.tcp_check_url);
        self.udp_check_dns.load(&g.udp_check_dns);
        self.tls_implementation.load(&g.tls_implementation);

        set_choice(&self.utls_imitate, &g.utls_imitate);

        self.tls_fragment.set_active(g.tls_fragment);
        self.tls_fragment_length.set_text(&g.tls_fragment_length);
        self.tls_fragment_interval
            .set_text(&g.tls_fragment_interval);

        self.bootstrap_resolver.set_text(&g.bootstrap_resolver);
        self.fallback_resolver.set_text(&g.fallback_resolver);

        self.bandwidth_max_tx.set_text(&g.bandwidth_max_tx);
        self.bandwidth_max_rx.set_text(&g.bandwidth_max_rx);
    }

    pub fn read(&self, original: &Config) -> Option<Config> {
        let mut config = original.clone();
        config.name = self.name.text().trim().to_owned();

        let g = &mut config.global;

        g.log_level = self
            .log_level
            .selected_item()
            .unwrap()
            .downcast::<gtk::StringObject>()
            .unwrap()
            .string()
            .to_string();

        g.tproxy_port = self.tproxy_port.text().parse().ok()?;
        g.tproxy_port_protect = self.tproxy_port_protect.is_active();

        g.so_mark_from_dae = self.so_mark_from_dae.text().parse().ok()?;
        g.pprof_port = self.pprof_port.text().parse().ok()?;

        g.lan_interface = self.lan_interface.values();
        g.wan_interface = self.wan_interface.values();

        g.auto_config_kernel_parameter = self.auto_config_kernel_parameter.is_active();
        g.auto_config_firewall_rule = self.auto_config_firewall_rule.is_active();

        g.disable_thp = self.disable_thp.is_active();
        g.mptcp = self.mptcp.is_active();

        g.enable_local_tcp_fast_redirect = self.enable_local_tcp_fast_redirect.is_active();
        g.bpf_conn_state_map_size = self.bpf_conn_state_map_size.text().parse().ok()?;

        g.dial_mode = self.dial_mode.value();
        g.allow_insecure = self.allow_insecure.is_active();

        g.disable_waiting_network = self.disable_waiting_network.is_active();
        g.udphop_interval = self.udphop_interval.text().trim().to_owned();

        g.check_interval = self.check_interval.text().trim().to_owned();
        g.check_tolerance = self.check_tolerance.text().trim().to_owned();

        g.sniffing_timeout = self.sniffing_timeout.text().trim().to_owned();
        g.auto_sniff_punt = self.auto_sniff_punt.is_active();

        g.tcp_check_http_method = self
            .tcp_check_http_method
            .selected_item()
            .unwrap()
            .downcast::<gtk::StringObject>()
            .unwrap()
            .string()
            .to_string();

        g.tcp_check_url = self.tcp_check_url.values();
        g.udp_check_dns = self.udp_check_dns.values();

        g.tls_implementation = self.tls_implementation.value();

        g.utls_imitate = self
            .utls_imitate
            .selected_item()
            .unwrap()
            .downcast::<gtk::StringObject>()
            .unwrap()
            .string()
            .to_string();

        g.tls_fragment = self.tls_fragment.is_active();
        g.tls_fragment_length = self.tls_fragment_length.text().trim().to_owned();
        g.tls_fragment_interval = self.tls_fragment_interval.text().trim().to_owned();

        g.bootstrap_resolver = self.bootstrap_resolver.text().trim().to_owned();
        g.fallback_resolver = self.fallback_resolver.text().trim().to_owned();

        g.bandwidth_max_tx = self.bandwidth_max_tx.text().trim().to_owned();
        g.bandwidth_max_rx = self.bandwidth_max_rx.text().trim().to_owned();

        Some(config)
    }

    pub fn validate(&self, focus: bool) -> bool {
        let mut valid = true;

        self.notes.iter().for_each(|note| {
            if !note.validate() {
                if valid && focus {
                    note.focus();
                }
                valid = false;
            }
        });

        [
            &self.lan_interface,
            &self.wan_interface,
            &self.tcp_check_url,
            &self.udp_check_dns,
        ]
        .iter()
        .for_each(|list| {
            if !list.validate() {
                if valid && focus {
                    list.focus_error();
                }
                valid = false;
            }
        });

        valid
    }

    pub fn clear_errors(&self) {
        self.notes.iter().for_each(|note| {
            note.clear_error();
        });

        [
            &self.lan_interface,
            &self.wan_interface,
            &self.tcp_check_url,
            &self.udp_check_dns,
        ]
        .iter()
        .for_each(|list| {
            list.clear_errors();
        });
    }

    pub fn update_conditions(&self) {
        self.utls_imitate
            .set_sensitive(self.tls_implementation.value() == "utls");

        self.notes.iter().for_each(|note| {
            if matches!(note.key, "tls_fragment_length" | "tls_fragment_interval") {
                note.set_required(self.tls_fragment.is_active());
                note.row.set_sensitive(self.tls_fragment.is_active());
            }
        });
    }

    pub fn retranslate(&self) {
        self.utls_imitate.set_title(&t!("config.utls_imitate"));
        self.utls_imitate
            .set_subtitle(&t!("config_help.utls_imitate"));

        self.notes.iter().for_each(|note| {
            note.retranslate();
        });

        self.groups.iter().for_each(|(key, group)| {
            group.set_title(t!(*key).as_ref());
        });

        self.log_level.set_title(t!("config.log_level").as_ref());
        self.log_level.set_use_markup(false);
        self.log_level.set_subtitle(&t!("config_help.log_level"));

        self.tproxy_port_protect
            .set_title(t!("config.tproxy_port_protect").as_ref());
        self.tproxy_port_protect.set_use_markup(false);
        self.tproxy_port_protect
            .set_subtitle(&t!("config_help.tproxy_port_protect"));

        self.lan_interface.retranslate();
        self.wan_interface.retranslate();

        self.auto_config_kernel_parameter
            .set_title(t!("config.auto_config_kernel_parameter").as_ref());

        self.auto_config_kernel_parameter.set_use_markup(false);

        self.auto_config_kernel_parameter
            .set_subtitle(&t!("config_help.auto_config_kernel_parameter"));
        self.auto_config_firewall_rule
            .set_title(t!("config.auto_config_firewall_rule").as_ref());

        self.auto_config_firewall_rule.set_use_markup(false);

        self.auto_config_firewall_rule
            .set_subtitle(&t!("config_help.auto_config_firewall_rule"));

        self.disable_thp
            .set_title(t!("config.disable_thp").as_ref());

        self.disable_thp.set_use_markup(false);
        self.disable_thp
            .set_subtitle(&t!("config_help.disable_thp"));

        self.mptcp.set_title(t!("config.mptcp").as_ref());
        self.mptcp.set_use_markup(false);
        self.mptcp.set_subtitle(&t!("config_help.mptcp"));

        self.enable_local_tcp_fast_redirect
            .set_title(t!("config.enable_local_tcp_fast_redirect").as_ref());
        self.enable_local_tcp_fast_redirect.set_use_markup(false);
        self.enable_local_tcp_fast_redirect
            .set_subtitle(&t!("config_help.enable_local_tcp_fast_redirect"));

        self.dial_mode.retranslate();

        self.allow_insecure
            .set_title(t!("config.allow_insecure").as_ref());

        self.allow_insecure.set_use_markup(false);
        self.allow_insecure
            .set_subtitle(&t!("config_help.allow_insecure"));

        self.disable_waiting_network
            .set_title(t!("config.disable_waiting_network").as_ref());
        self.disable_waiting_network.set_use_markup(false);
        self.disable_waiting_network
            .set_subtitle(&t!("config_help.disable_waiting_network"));

        self.auto_sniff_punt
            .set_title(t!("config.auto_sniff_punt").as_ref());
        self.auto_sniff_punt.set_use_markup(false);
        self.auto_sniff_punt
            .set_subtitle(&t!("config_help.auto_sniff_punt"));

        self.tcp_check_http_method
            .set_title(t!("config.tcp_check_http_method").as_ref());
        self.tcp_check_http_method.set_use_markup(false);
        self.tcp_check_http_method
            .set_subtitle(&t!("config_help.tcp_check_http_method"));

        self.tcp_check_url.retranslate();
        self.udp_check_dns.retranslate();
        self.tls_implementation.retranslate();

        self.tls_fragment
            .set_title(t!("config.tls_fragment").as_ref());
        self.tls_fragment.set_use_markup(false);
        self.tls_fragment
            .set_subtitle(&t!("config_help.tls_fragment"));
    }
}

fn set_choice(row: &adw::ComboRow, value: &str) {
    let options = row.model().unwrap().downcast::<gtk::StringList>().unwrap();

    let selected = (0..options.n_items())
        .find(|&i| options.string(i).as_deref() == Some(value))
        .unwrap_or_else(|| {
            options.append(value);
            options.n_items() - 1
        });

    row.set_selected(selected);
}
