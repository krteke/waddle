use relm4::gtk::glib::{self, GString};

pub fn apply_language(preference: &str) {
    let system = glib::language_names();

    rust_i18n::set_locale(language(preference, &system));
}

fn language<'a>(preference: &'a str, system: &[GString]) -> &'a str {
    if !preference.is_empty() {
        return preference;
    }

    system
        .iter()
        .find_map(
            |language| match language.split(['_', '-', '.', '@']).next()? {
                "zh" => Some("zh_CN"),
                "en" | "C" | "POSIX" => Some("en"),
                _ => None,
            },
        )
        .unwrap_or("en")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_i18n::t;

    fn gstring(str: &[&'static str]) -> Vec<GString> {
        str.iter().map(|&s| GString::from(s)).collect()
    }

    #[test]
    fn unset_preference_uses_system_chinese() {
        assert_eq!(language("", &gstring(&["zh_CN.UTF-8", "zh", "C"])), "zh_CN");
    }

    #[test]
    fn unset_preference_uses_system_english() {
        assert_eq!(language("", &gstring(&["en_US.UTF-8", "en", "C"])), "en");
    }

    #[test]
    fn explicit_language_overrides_system_language() {
        assert_eq!(language("en", &gstring(&["zh_CN.UTF-8"])), "en");
    }

    #[test]
    fn unsupported_system_language_uses_english() {
        assert_eq!(language("", &gstring(&["de_DE.UTF-8", "de", "C"])), "en");
    }

    #[test]
    fn translations_interpolate_server_names() {
        assert_eq!(
            t!("server.connected", locale = "zh_CN", name = "Home").to_string(),
            "已连接 Home"
        );
        assert_eq!(
            t!("server.connected", locale = "en", name = "Home").to_string(),
            "Connected to Home"
        );
    }
}
