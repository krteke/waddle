use super::*;
use crate::error::LinkError;
use url::Url;

#[test]
fn vmess_uri_from_daed_parser_examples_is_converted_to_backend_json() {
    let node = "vmess://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?type=ws&security=tls&host=example.com&path=%2Fws&encryption=auto#standard-vmess".decode::<Vmess>().unwrap();
    assert_eq!(
        node.protocol.transport,
        Transport::WebSocket {
            host: "example.com".into(),
            path: "/ws".into()
        }
    );
    let encoded = node.encode().unwrap();
    let json: serde_json::Value = serde_json::from_str(
        &uri::base64_decode(encoded.strip_prefix("vmess://").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(json["net"], "ws");
    assert_eq!(json["aid"], 0);
    assert_eq!(json["ps"], "standard-vmess");
}
#[test]
fn v2rayn_string_port_and_alter_id_are_accepted() {
    let json = r#"{"add":"example.com","port":"443","id":"b116693c-e4f4-4776-a80c-f310b03b36e0","aid":"0","net":"tcp","tls":"","allowInsecure":1}"#;
    let node = format!("vmess://{}", uri::base64_encode(json))
        .decode::<Vmess>()
        .unwrap();
    assert_eq!(node.port, 443);
    assert_eq!(node.protocol.tls, V2rayTls::None);
    assert!(node.protocol.insecure);
}
#[test]
fn legacy_shadowsocks_whole_authority_is_supported() {
    let node = "ss://YWVzLTI1Ni1nY206c2VjcmV0QGxvY2FsaG9zdDo4Mzg4#Legacy"
        .decode::<Shadowsocks>()
        .unwrap();
    assert_eq!(node.protocol.method, "aes-256-gcm");
    assert_eq!(node.protocol.password, "secret");
    assert_eq!(node.server, "localhost");
    assert_eq!(node.port, 8388);
}
#[test]
fn sip002_plain_userinfo_supports_2022_credentials() {
    let node = "ss://2022-blake3-aes-128-gcm:MDEyMzQ1Njc4OWFiY2RlZg%3D%3D@example.com:8388#SIP002"
        .decode::<Shadowsocks>()
        .unwrap();
    assert_eq!(node.protocol.method, "2022-blake3-aes-128-gcm");
    assert_eq!(node.protocol.password, "MDEyMzQ1Njc4OWFiY2RlZg==");
}
#[test]
fn hysteria_short_scheme_and_default_port() {
    let node = "hy2://auth@example.com#test".decode::<Protocol>().unwrap();
    assert_eq!(node.port, 443);
    let Protocol::Hysteria2(protocol) = node.protocol else {
        panic!("expected Hysteria2")
    };
    assert_eq!(protocol.auth, "auth");
}
#[test]
fn socks_credentials_and_ipv6_are_encoded_for_backend() {
    let node = NodeConfig {
        name: String::new(),
        port: 1080,
        server: "::1".into(),
        protocol: Socks5 {
            username: "user%41@".into(),
            password: "p%20:@/#密".into(),
        },
    };
    let link = node.encode().unwrap();
    let url = Url::parse(&link).unwrap();
    assert_eq!(url.host_str(), Some("[::1]"));
    assert_eq!(url.username(), "user%2541%40");
    assert_eq!(
        uri::unescape(url.password().unwrap()).unwrap(),
        "p%20:@/#密"
    );
}
#[test]
fn unknown_query_parameters_are_not_reemitted() {
    let node = "trojan://secret@example.com:443?sni=tls.example.com&custom=unused#test"
        .decode::<Trojan>()
        .unwrap();
    assert_eq!(node.protocol.sni, "tls.example.com");
    assert!(!node.encode().unwrap().contains("custom"));
}
#[test]
fn conflicting_sni_aliases_are_not_guessed() {
    assert!(matches!(
        "trojan://secret@example.com:443?sni=one&peer=two".decode::<Trojan>(),
        Err(LinkError::Structure)
    ));
}
#[test]
fn duplicate_query_parameters_follow_daed_first_value() {
    let node = "trojan://secret@example.com:443?sni=first&sni=second"
        .decode::<Trojan>()
        .unwrap();
    assert_eq!(node.protocol.sni, "first");
}
macro_rules! rejected {
    ($name:ident, $link:literal, $error:pat) => {
        #[test]
        fn $name() {
            assert!(matches!($link.decode::<Protocol>(), Err($error)));
        }
    };
}
rejected!(
    unsupported_vless_transport_is_not_presented_as_editable,
    "vless://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?type=xhttp",
    LinkError::Unsupported
);
rejected!(
    incomplete_vless_link_cannot_prefill_a_valid_node,
    "vless://example.com:443",
    LinkError::Structure
);
rejected!(
    unknown_protocol_is_unsupported,
    "wireguard://example.com:443",
    LinkError::Unsupported
);
rejected!(
    zero_port_is_invalid,
    "trojan://secret@example.com:0",
    LinkError::Structure
);
rejected!(
    out_of_range_port_is_invalid,
    "trojan://secret@example.com:99999",
    LinkError::Url(_)
);
rejected!(
    invalid_base64_is_rejected,
    "vmess://!!!",
    LinkError::Base64(_)
);
rejected!(
    unsupported_plugin_is_rejected,
    "ss://YWVzLTEyOC1nY206cGFzcw@example.com:8388?plugin=unknown",
    LinkError::Unsupported
);
rejected!(
    malformed_ssr_structure_is_rejected,
    "ssr://YWJj",
    LinkError::Structure
);
rejected!(
    ambiguous_single_auth_is_rejected,
    "trojan://user:password@example.com:443",
    LinkError::Structure
);
#[test]
fn typed_decoder_rejects_other_protocol() {
    assert!(matches!(
        "trojan://secret@example.com:443".decode::<Vless>(),
        Err(LinkError::Unsupported)
    ));
}
#[test]
fn non_aead_vmess_is_unsupported() {
    let json = r#"{"add":"example.com","port":443,"id":"uuid","aid":64}"#;
    assert!(matches!(
        format!("vmess://{}", uri::base64_encode(json)).decode::<Vmess>(),
        Err(LinkError::Unsupported)
    ));
}
#[test]
fn server_field_rejects_embedded_url_path() {
    let node = NodeConfig {
        name: String::new(),
        port: 443,
        server: "example.com/path".into(),
        protocol: Trojan {
            password: "secret".into(),
            ..Default::default()
        },
    };
    assert!(node.encode().is_err());
}

#[test]
fn unsupported_shadowsocks_cipher_is_not_presented_as_editable() {
    assert!(matches!(
        "ss://unsupported:password@example.com:8388".decode::<Shadowsocks>(),
        Err(LinkError::Unsupported)
    ));
}

#[test]
fn ssr_ipv6_address_does_not_shift_protocol_fields() {
    let body = "2001:db8::1:8388:auth_aes128_sha1:aes-128-cfb:tls1.2_ticket_auth:c2VjcmV0/?remarks=TmFtZQ&protoparam=dXNlcg&obfsparam=aG9zdA";
    let node = format!("ssr://{}", uri::base64_url(body))
        .decode::<ShadowsocksR>()
        .unwrap();
    assert_eq!(node.server, "2001:db8::1");
    assert_eq!(node.port, 8388);
    assert_eq!(node.protocol.protocol, "auth_aes128_sha1");
    assert_eq!(node.protocol.password, "secret");
}

#[test]
fn vless_reality_fields_can_be_edited() {
    let mut node = "vless://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?type=tcp&security=reality&flow=xtls-rprx-vision&sni=www.example.com&fp=chrome&pbk=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA&sid=0123&spx=%2F".decode::<Vless>().unwrap();
    assert_eq!(node.protocol.flow, "xtls-rprx-vision");
    let V2rayTls::Reality {
        short_id, spider_x, ..
    } = &mut node.protocol.tls
    else {
        panic!("expected Reality")
    };
    assert_eq!(short_id, "0123");
    assert_eq!(spider_x, "/");
    *short_id = "abcd".into();
    let link = Url::parse(&node.encode().unwrap()).unwrap();
    assert!(link.query_pairs().any(|(k, v)| k == "sid" && v == "abcd"));
    assert!(
        link.query_pairs()
            .any(|(k, v)| k == "pbk" && v == "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
    );
}

#[test]
fn hysteria_user_password_auth_becomes_one_form_credential() {
    let node = "hy2://user:pass%40word@example.com:443"
        .decode::<Hysteria2>()
        .unwrap();
    assert_eq!(node.protocol.auth, "user:pass@word");
    let url = Url::parse(&node.encode().unwrap()).unwrap();
    assert_eq!(uri::unescape(url.username()).unwrap(), "user:pass@word");
    assert!(url.password().is_none());
}

#[test]
fn vmess_without_tls_does_not_emit_inactive_tls_fields() {
    let node = "vmess://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?security=none&sni=unused.example.com&pbk=unused".decode::<Vmess>().unwrap();
    let link = node.encode().unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&uri::base64_decode(link.strip_prefix("vmess://").unwrap()).unwrap())
            .unwrap();
    assert_eq!(json["sni"], "");
    assert_eq!(json["pbk"], "");
}

#[test]
fn grpc_service_name_uses_uri_service_name_and_vmess_json_path() {
    let node = "vmess://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?type=grpc&serviceName=my-service&host=unused&path=unused".decode::<Vmess>().unwrap();
    assert_eq!(
        node.protocol.transport,
        Transport::Grpc {
            service_name: "my-service".into()
        }
    );
    let encoded = node.encode().unwrap();
    let json: serde_json::Value = serde_json::from_str(
        &uri::base64_decode(encoded.strip_prefix("vmess://").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(json["net"], "grpc");
    assert_eq!(json["path"], "my-service");
    assert_eq!(json["host"], "");
}

#[test]
fn tcp_without_http_header_drops_http_options() {
    let node = "vless://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?type=tcp&headerType=none&host=unused&path=unused".decode::<Vless>().unwrap();
    assert_eq!(
        node.protocol.transport,
        Transport::Tcp {
            header: TcpHeader::None
        }
    );
    let link = Url::parse(&node.encode().unwrap()).unwrap();
    assert!(!link.query_pairs().any(|(k, _)| k == "host" || k == "path"));
}

#[test]
fn tcp_http_header_retains_its_host_and_path() {
    let node = "vless://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?type=tcp&headerType=http&host=cdn.example.com&path=%2Fproxy".decode::<Vless>().unwrap();
    assert_eq!(
        node.protocol.transport,
        Transport::Tcp {
            header: TcpHeader::Http {
                host: "cdn.example.com".into(),
                path: "/proxy".into()
            }
        }
    );
    let link = Url::parse(&node.encode().unwrap()).unwrap();
    assert!(
        link.query_pairs()
            .any(|(k, v)| k == "headerType" && v == "http")
    );
    assert!(
        link.query_pairs()
            .any(|(k, v)| k == "path" && v == "/proxy")
    );
}

#[test]
fn ordinary_tls_does_not_carry_reality_options() {
    let node = "vless://b116693c-e4f4-4776-a80c-f310b03b36e0@example.com:443?security=tls&sni=tls.example.com&alpn=h2&pbk=unused&sid=unused".decode::<Vless>().unwrap();
    assert_eq!(
        node.protocol.tls,
        V2rayTls::Tls {
            sni: "tls.example.com".into(),
            alpn: "h2".into()
        }
    );
    let link = Url::parse(&node.encode().unwrap()).unwrap();
    assert!(!link.query_pairs().any(|(k, _)| k == "pbk" || k == "sid"));
}

#[test]
fn trojan_go_grpc_and_shadowsocks_encryption_are_distinct_payloads() {
    let node = "trojan-go://secret@example.com:443?type=grpc&serviceName=proxy&encryption=ss%3Baes-128-gcm%3Bpassword".decode::<TrojanGo>().unwrap();
    assert_eq!(
        node.protocol.transport,
        TrojanTransport::Grpc {
            service_name: "proxy".into()
        }
    );
    assert_eq!(
        node.protocol.encryption,
        TrojanEncryption::Shadowsocks {
            cipher: "aes-128-gcm".into(),
            password: "password".into()
        }
    );
    let link = Url::parse(&node.encode().unwrap()).unwrap();
    assert!(
        link.query_pairs()
            .any(|(k, v)| k == "serviceName" && v == "proxy")
    );
    assert!(
        link.query_pairs()
            .any(|(k, v)| k == "encryption" && v == "ss;aes-128-gcm;password")
    );
}
