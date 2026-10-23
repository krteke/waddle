use super::*;

type Duration = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/configs.graphql",
    response_derives = "Debug, Clone, PartialEq"
)]
struct Configs;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/interfaces.graphql",
    response_derives = "Debug, Clone"
)]
struct Interfaces;

pub use interfaces::InterfacesGeneralInterfaces as Interface;

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
    query_path = "graphql/queries/default_config.graphql",
    response_type = "crate::api::config::Config"
)]
struct DefaultConfig;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/update_config.graphql",
    variable_types("crate::api::config::create_config::globalInput")
)]
struct UpdateConfig;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "graphql/queries/remove_config.graphql"
)]
struct RemoveConfig;

pub use configs::ConfigsConfigs as Config;

impl Api {
    pub async fn interfaces(&self) -> Result<Vec<Interface>, ApiError> {
        Ok(self
            .execute::<Interfaces>(interfaces::Variables {})
            .await?
            .general
            .interfaces)
    }

    pub async fn duplicate_config(&self, mut config: Config) -> Result<(), ApiError> {
        config.name = format!("{} (Copy)", config.name);
        self.save_config(config, None).await
    }

    pub async fn save_config(
        &self,
        config: Config,
        original_name: Option<&str>,
    ) -> Result<(), ApiError> {
        let rename = original_name.is_some_and(|name| name != config.name);
        let id = config.id;
        let name = config.name;
        let g = config.global;

        let global = create_config::globalInput {
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
        };

        if original_name.is_some() {
            self.execute::<UpdateConfig>(update_config::Variables {
                id: id.clone(),
                global,
            })
            .await?;
        } else {
            self.execute::<CreateConfig>(create_config::Variables {
                name: name.clone(),
                global,
            })
            .await?;
        }

        if rename {
            self.rename_config(id, name).await?;
        }

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

    pub async fn default_config(&self) -> Result<Config, ApiError> {
        Ok(self
            .execute::<DefaultConfig>(default_config::Variables {})
            .await?
            .default_config)
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
