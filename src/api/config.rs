use super::*;

type Duration = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/configs.graphql",
    response_derives = "Debug, Clone"
)]
struct Configs;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/select_config.graphql"
)]
struct SelectConfig;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/rename_config.graphql"
)]
struct RenameConfig;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/create_config.graphql"
)]
struct CreateConfig;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/remove_config.graphql"
)]
struct RemoveConfig;

pub use configs::ConfigsConfigs as Config;

impl Api {
    pub async fn duplicate_config(&self, config: Config) -> Result<(), ApiError> {
        let g = config.global;

        self.execute::<CreateConfig>(create_config::Variables {
            name: format!("{} (Copy)", config.name),
            global: create_config::globalInput {
                tproxy_port: Some(g.tproxy_port),
                tproxy_port_protect: Some(g.tproxy_port_protect),
                so_mark_from_dae: Some(g.so_mark_from_dae),
                log_level: Some(g.log_level),
                tcp_check_url: Some(g.tcp_check_url),
                tcp_check_http_method: Some(g.tcp_check_http_method),
                udp_check_dns: Some(g.udp_check_dns),
                check_interval: Some(g.check_interval),
                check_tolerance: Some(g.check_tolerance),
                lan_interface: Some(g.lan_interface),
                wan_interface: Some(g.wan_interface),
                allow_insecure: Some(g.allow_insecure),
                dial_mode: Some(g.dial_mode),
                disable_waiting_network: Some(g.disable_waiting_network),
                disable_thp: Some(g.disable_thp),
                enable_local_tcp_fast_redirect: Some(g.enable_local_tcp_fast_redirect),
                auto_config_kernel_parameter: Some(g.auto_config_kernel_parameter),
                auto_config_firewall_rule: Some(g.auto_config_firewall_rule),
                sniffing_timeout: Some(g.sniffing_timeout),
                auto_sniff_punt: Some(g.auto_sniff_punt),
                tls_implementation: Some(g.tls_implementation),
                utls_imitate: Some(g.utls_imitate),
                tls_fragment: Some(g.tls_fragment),
                tls_fragment_length: Some(g.tls_fragment_length),
                tls_fragment_interval: Some(g.tls_fragment_interval),
                pprof_port: Some(g.pprof_port),
                mptcp: Some(g.mptcp),
                bootstrap_resolver: Some(g.bootstrap_resolver),
                fallback_resolver: Some(g.fallback_resolver),
                bandwidth_max_tx: Some(g.bandwidth_max_tx),
                bandwidth_max_rx: Some(g.bandwidth_max_rx),
                udphop_interval: Some(g.udphop_interval),
                bpf_conn_state_map_size: Some(g.bpf_conn_state_map_size),
            },
        })
        .await?;

        Ok(())
    }

    pub async fn rename_config(&self, id: String, name: String) -> Result<(), ApiError> {
        self.execute::<RenameConfig>(rename_config::Variables { id, name })
            .await?;

        Ok(())
    }

    pub async fn remove_config(&self, id: String) -> Result<(), ApiError> {
        self.execute::<RemoveConfig>(remove_config::Variables { id })
            .await?;

        Ok(())
    }
    pub async fn configs(&self) -> Result<Vec<Config>, ApiError> {
        Ok(self
            .execute::<Configs>(configs::Variables {})
            .await?
            .configs)
    }

    pub async fn select_config(&self, id: String) -> Result<(), ApiError> {
        self.execute::<SelectConfig>(select_config::Variables { id })
            .await?;

        Ok(())
    }
}
