#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Gfw,
    NonCn,
    CnOnly,
    Global,
    MacOnly,
}

impl Mode {
    pub const ALL: [Self; 5] = [
        Self::Gfw,
        Self::NonCn,
        Self::CnOnly,
        Self::Global,
        Self::MacOnly,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MacAction {
    #[default]
    Proxy,
    Direct,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Preset {
    pub mode: Mode,
    pub group: String,
    pub macs: Vec<String>,
    pub mac_action: MacAction,
}

impl Preset {
    pub fn text(&self) -> String {
        let mut lines = vec![
            "pname(NetworkManager, systemd-resolved, dnsmasq) -> must_direct".to_owned(),
            "dip(geoip:private) -> direct".to_owned(),
        ];

        if !self.macs.is_empty() {
            let addresses = self
                .macs
                .iter()
                .map(|mac| format!("'{mac}'"))
                .collect::<Vec<_>>()
                .join(", ");

            let target = match self.mac_action {
                MacAction::Proxy => &self.group,
                MacAction::Direct => "direct",
            };

            lines.push(format!("mac({addresses}) -> {target}"));
        }

        let group = &self.group;

        match self.mode {
            Mode::Gfw => lines.extend([
                format!("domain(geosite:gfw) -> {group}"),
                "fallback: direct".into(),
            ]),
            Mode::NonCn | Mode::MacOnly => lines.extend([
                "dip(geoip:cn) -> direct".into(),
                "domain(geosite:cn) -> direct".into(),
                format!(
                    "fallback: {}",
                    if self.mode == Mode::MacOnly {
                        "direct"
                    } else {
                        group
                    }
                ),
            ]),
            Mode::CnOnly => lines.extend([
                format!("dip(geoip:cn) -> {group}"),
                format!("domain(geosite:cn) -> {group}"),
                "fallback: direct".into(),
            ]),
            Mode::Global => lines.push(format!("fallback: {group}")),
        }

        lines.join("\n")
    }

    pub fn detect(text: &str, group: &str) -> Option<Self> {
        let lines: Vec<_> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();

        let mut preset = Self {
            group: group.to_string(),
            ..Default::default()
        };

        if let Some(line) = lines.get(2).and_then(|line| line.strip_prefix("mac(")) {
            let (addresses, target) = line.split_once(") -> ")?;

            preset.mac_action = if target == group {
                MacAction::Proxy
            } else if target == "direct" {
                MacAction::Direct
            } else {
                return None;
            };

            for address in addresses.split(',') {
                let address = address.trim().trim_matches(['\'', '"']);

                preset.macs.push(address.into());
            }
        }

        let normalized = lines.join("\n");

        for mode in Mode::ALL {
            preset.mode = mode;
            if preset.text() == normalized {
                return Some(preset);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests;
