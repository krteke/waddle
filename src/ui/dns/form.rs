use crate::api::dns::{DnsDocument, Fallback, Function, RuleSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Upstream {
    pub name: String,
    pub address: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub matcher: String,
    pub target: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rules {
    pub entries: Vec<Rule>,
    pub fallback: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Form {
    pub upstreams: Vec<Upstream>,
    pub request: Rules,
    pub response: Rules,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            upstreams: vec![
                Upstream {
                    name: "alidns".into(),
                    address: "udp://223.5.5.5:53".into(),
                },
                Upstream {
                    name: "googledns".into(),
                    address: "tcp+udp://8.8.8.8:53".into(),
                },
            ],
            request: Rules {
                entries: vec![Rule {
                    matcher: "qname(geosite:cn)".into(),
                    target: "alidns".into(),
                }],
                fallback: "googledns".into(),
            },
            response: Rules {
                entries: vec![],
                fallback: "accept".into(),
            },
        }
    }
}

impl Form {
    pub fn text(&self) -> String {
        let mut text = String::new();
        if !self.upstreams.is_empty() {
            text.push_str("upstream {\n");
            for upstream in &self.upstreams {
                text.push_str(&format!(
                    "  {}: {}\n",
                    upstream.name,
                    quoted(&upstream.address)
                ));
            }
            text.push_str("}\n");
        }
        text.push_str("routing {\n");
        for (name, rules) in [("request", &self.request), ("response", &self.response)] {
            if rules.entries.is_empty() && rules.fallback.is_empty() {
                continue;
            }
            text.push_str(&format!("  {name} {{\n"));
            for rule in &rules.entries {
                text.push_str(&format!("    {} -> {}\n", rule.matcher, rule.target));
            }
            if !rules.fallback.is_empty() {
                text.push_str(&format!("    fallback: {}\n", rules.fallback));
            }
            text.push_str("  }\n");
        }
        text.push('}');
        text
    }

    pub fn from_document(raw: &str, document: &DnsDocument) -> Option<Self> {
        let mut form = Self {
            upstreams: document
                .upstream
                .iter()
                .map(|p| Upstream {
                    name: p.key.clone(),
                    address: p.val.clone(),
                })
                .collect(),
            request: rules(&document.routing.request),
            response: rules(&document.routing.response),
        };
        let compact = |text: &str| text.split_whitespace().collect::<String>();
        let original = compact(raw);
        for omit_request in [false, true] {
            for omit_response in [false, true] {
                let mut candidate = form.clone();
                if omit_request && candidate.request.entries.is_empty() {
                    candidate.request.fallback.clear();
                }
                if omit_response && candidate.response.entries.is_empty() {
                    candidate.response.fallback.clear();
                }
                if compact(&candidate.text()) == original {
                    return Some(candidate);
                }
            }
        }

        if form.request.entries.is_empty() && form.response.entries.is_empty() {
            form.request.fallback.clear();
            form.response.fallback.clear();
            if compact(&form.text()).trim_end_matches("routing{}") == original {
                return Some(form);
            }
        }

        None
    }
}

fn rules(set: &RuleSet) -> Rules {
    Rules {
        entries: set
            .rules
            .iter()
            .map(|r| Rule {
                matcher: r
                    .conditions
                    .and
                    .iter()
                    .map(function)
                    .collect::<Vec<_>>()
                    .join(" && "),
                target: function(&r.outbound),
            })
            .collect(),
        fallback: match &set.fallback {
            Fallback::Plaintext { val } => val.clone(),
            Fallback::Function(f) => function(f),
        },
    }
}

fn function(f: &Function) -> String {
    let params = f
        .params
        .iter()
        .map(|p| {
            let val = if p
                .val
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-.:/".contains(c))
                && !p.val.is_empty()
            {
                p.val.clone()
            } else {
                quoted(&p.val)
            };
            if p.key.is_empty() {
                val
            } else {
                format!("{}:{val}", p.key)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    if f.params.is_empty() && !f.not {
        return f.name.clone();
    }
    format!("{}{}({params})", if f.not { "!" } else { "" }, f.name)
}

fn quoted(value: &str) -> String {
    if value.contains('\'') {
        format!("\"{value}\"")
    } else {
        format!("'{value}'")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{api::Api, credentials::Credentials, test_support::Wing};

    fn session(wing: &Wing, runtime: &tokio::runtime::Runtime) -> Api {
        runtime.block_on(async {
            Api::new(format!("http://{}", wing.address).parse().unwrap())
                .authenticate(&Credentials::new("dns-admin", "dns-test-password42"), true)
                .await
                .unwrap()
                .0
        })
    }

    #[test]
    #[ignore = "requires DAE_WING_BIN; run scripts/test-e2e.sh"]
    fn daed_default_dns_can_populate_the_form() {
        let wing = Wing::start();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let api = session(&wing, &runtime);
        // daed's DEFAULT_DNS, including its omitted response section.
        let text = "upstream {\n  alidns: 'udp://223.5.5.5:53'\n  googledns: 'tcp+udp://8.8.8.8:53'\n}\nrouting {\n  request {\n    qname(geosite:cn) -> alidns\n    fallback: googledns\n  }\n}";
        let document = runtime.block_on(api.parse_dns(text.into())).unwrap();
        let form = Form::from_document(text, &document).expect("daed's default must fit the form");
        assert_eq!(form.upstreams[0].address, "udp://223.5.5.5:53");
        assert_eq!(form.request.entries[0].matcher, "qname(geosite:cn)");
        assert_eq!(form.request.fallback, "googledns");
        assert!(form.response.entries.is_empty());
        assert!(form.response.fallback.is_empty());
        runtime
            .block_on(api.create_dns("Imported".into(), form.text()))
            .unwrap();
        let saved = runtime.block_on(api.dnss()).unwrap().remove(0);
        assert_eq!(saved.dns.upstream.len(), 2);
    }

    #[test]
    #[ignore = "requires DAE_WING_BIN; run scripts/test-e2e.sh"]
    fn unexposed_options_require_advanced_editing() {
        let wing = Wing::start();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let api = session(&wing, &runtime);
        let text = format!("ipversion_prefer: 4\n{}", Form::default().text());
        let document = runtime.block_on(api.parse_dns(text.clone())).unwrap();
        assert!(Form::from_document(&text, &document).is_none());
    }

    #[test]
    #[ignore = "requires DAE_WING_BIN; run scripts/test-e2e.sh"]
    fn comments_remain_in_advanced_editing() {
        let wing = Wing::start();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let api = session(&wing, &runtime);
        let text = format!("# Preserve my policy\n{}", Form::default().text());
        let document = runtime.block_on(api.parse_dns(text.clone())).unwrap();
        assert!(Form::from_document(&text, &document).is_none());
    }
}
