use super::*;
const HEADER: &str =
    "pname(NetworkManager, systemd-resolved, dnsmasq) -> must_direct\ndip(geoip:private) -> direct";
fn preset(mode: Mode) -> Preset {
    Preset {
        mode,
        group: "proxy".into(),
        macs: vec![],
        mac_action: MacAction::Proxy,
    }
}
#[test]
fn gfw_template_matches_daed() {
    assert_eq!(
        preset(Mode::Gfw).text(),
        format!("{HEADER}\ndomain(geosite:gfw) -> proxy\nfallback: direct")
    );
}
#[test]
fn non_cn_template_matches_daed() {
    assert_eq!(
        preset(Mode::NonCn).text(),
        format!("{HEADER}\ndip(geoip:cn) -> direct\ndomain(geosite:cn) -> direct\nfallback: proxy")
    );
}
#[test]
fn cn_only_template_matches_daed() {
    assert_eq!(
        preset(Mode::CnOnly).text(),
        format!("{HEADER}\ndip(geoip:cn) -> proxy\ndomain(geosite:cn) -> proxy\nfallback: direct")
    );
}
#[test]
fn global_template_matches_daed() {
    assert_eq!(
        preset(Mode::Global).text(),
        format!("{HEADER}\nfallback: proxy")
    );
}
#[test]
fn mac_only_template_preserves_direct_fallback() {
    assert_eq!(
        preset(Mode::MacOnly).text(),
        format!(
            "{HEADER}\ndip(geoip:cn) -> direct\ndomain(geosite:cn) -> direct\nfallback: direct"
        )
    );
}
#[test]
fn mac_rule_precedes_country_rules() {
    let value = Preset {
        group: "home".into(),
        macs: vec!["AA:BB:CC:DD:EE:FF".into()],
        mac_action: MacAction::Direct,
        ..preset(Mode::NonCn)
    };
    assert_eq!(
        value.text(),
        format!(
            "{HEADER}\nmac('AA:BB:CC:DD:EE:FF') -> direct\ndip(geoip:cn) -> direct\ndomain(geosite:cn) -> direct\nfallback: home"
        )
    );
}
#[test]
fn detects_daed_preset_with_mac_control() {
    let text = format!(
        "# LAN devices\n{HEADER}\nmac('aa:bb:cc:dd:ee:ff', '11:22:33:44:55:66') -> home\ndip(geoip:cn) -> direct\ndomain(geosite:cn) -> direct\nfallback: direct"
    );
    assert_eq!(
        Preset::detect(&text, "home"),
        Some(Preset {
            mode: Mode::MacOnly,
            group: "home".into(),
            macs: vec!["aa:bb:cc:dd:ee:ff".into(), "11:22:33:44:55:66".into()],
            mac_action: MacAction::Proxy
        })
    );
}
#[test]
fn custom_rule_with_known_prefix_stays_advanced() {
    assert_eq!(
        Preset::detect(
            &format!("{HEADER}\npname(custom) -> block\nfallback: proxy"),
            "proxy"
        ),
        None
    );
}
#[test]
fn reordered_mac_rule_stays_advanced() {
    assert_eq!(
        Preset::detect(
            &format!(
                "{HEADER}\ndip(geoip:cn) -> direct\nmac('aa:bb:cc:dd:ee:ff') -> proxy\ndomain(geosite:cn) -> direct\nfallback: direct"
            ),
            "proxy"
        ),
        None
    );
}
