/// Struct that represents configuration for DHT that, in general, does
/// not change after the DHT is started.
///
/// You may need one of these to pass into [DHTBuilder](crate::dht::DHTBuilder).
///
/// Use [DHTSettings::default()](crate::dht::DHTSettings::default) to create an instance with the
/// 'recommended' defaults (which can be customized). Or use [DHTSettingsBuilder](crate::dht::DHTSettingsBuilder)
/// to construct a customized one. DHTSettings has the [non_exhaustive](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute)
/// attribute and can't be constructed directly.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Settings {
    pub ctx_index: i32,

    pub block_list_max_size: usize,

    /// block duration
    pub timeout_block_duration_sec: u64,

    /// block duration
    pub reply_error_block_duration_sec: u64,

    /// block duration
    pub firewall_block_duration_sec: u64,

    /// Number of nodes in the bucket
    pub bucket_size: usize,

    /// Number of bytes for token secrets for get_peers responses
    pub token_secret_size: usize,

    /// Max number of peers to provide in response to a get_peers.
    /// Shouldn't be much higher than this as the entire response packet needs to be less than 1500
    pub max_peers_response: usize,

    /// We'll ping the "routers" at least this often (we may ping more frequently if needed)
    pub router_ping_interval_secs: u64,

    /// We'll ping the "routers" at least this often (we may ping more frequently if not join kad)
    pub router_ping_if_not_join_interval_secs: u64,

    /// We'll ping previously-verified nodes at least this often to re-verify them
    /// 对先前已验证的节点进行 ping 操作以重新验证它们的时间间隔
    pub reverify_interval_secs: u64,

    /// Verified nodes that we don't reverify within this amount of time are dropped
    /// 在这段时间内没有重新验证的已验证节点将被删除
    pub reverify_grace_period_secs: u64,

    /// New nodes have this long to respond to a ping before we drop them
    /// 新节点在被删除之前有这么长的时间来响应 ping
    pub verify_grace_period_secs: u64,

    /// When asked to provide peers, we'll only provide ones that announced within this time
    pub get_peers_freshness_secs: u64,

    /// We'll think about sending a find_nodes request at least this often.
    /// If we have enough nodes already we might not do it.
    pub find_nodes_interval_secs: u64,

    /// We won't send a periodic find_nodes request if we have at least this many unverified nodes
    /// 如果至少有 find_nodes_skip_count 个未经验证的节点，我们将不会发送定期的 find_nodes 请求
    pub find_nodes_skip_count: usize,

    /// Max number of torrents to store peers for
    pub max_resources: usize,

    /// Max number of peers per torrent to store
    pub max_peers_per_resource: usize,

    /// We'll think about pinging and pruning nodes at this interval
    pub ping_check_interval_secs: u64,

    /// Outgoing requests may be pruned after this many seconds
    pub outgoing_request_prune_secs: u64,

    /// If true, we will set the read only flag in outgoing requests to prevent
    /// other nodes from adding us to their routing tables. This is useful if
    /// we're behind a restrictive NAT/firewall and can't accept incoming
    /// packets from IPs that we haven't sent anything to.
    ///
    /// 如果为 true，我们将在传出请求中设置只读标志，以防止其他节点将我们添加到其路由表中。
    /// 如果我们位于限制性 NAT/防火墙后面并且无法接受来自我们尚未发送任何内容的 IP 的传入数据包，则这非常有用。
    pub read_only: bool,

    /// Vector of hostnames/ports that the DHT will use as DHT routers for
    /// bootstrapping purposes.
    ///
    /// E.g., "router.example.org:6881"
    pub routers: Vec<String>,

    /// 清理 transaction 的时间间隔
    pub transaction_cleanup_interval_sec: u64,

    /// 发送 query 超时时长
    pub send_query_timeout_sec: u64,

    /// 发送下一次 query 的时间间隔
    pub send_next_query_interval_sec: u64,

    /// 刷新 token 的时间间隔
    pub token_refresh_interval_sec: u64,

    /// 更新 Ipv4 权重的时间间隔
    pub ip4_maintenance_interval_sec: u64,
}

/// Returns DHTSettings with a default set of options.
impl Default for Settings {
    fn default() -> Self {
        Settings {
            ctx_index: -1,
            block_list_max_size: 65535,

            timeout_block_duration_sec: 10,
            reply_error_block_duration_sec: 60 * 60,
            firewall_block_duration_sec: 60 * 60 * 8,

            bucket_size: 8,
            token_secret_size: 10,
            max_peers_response: 128,
            router_ping_interval_secs: 900,
            router_ping_if_not_join_interval_secs: 30,
            reverify_interval_secs: 14 * 60,
            reverify_grace_period_secs: 15 * 60,
            verify_grace_period_secs: 60,
            get_peers_freshness_secs: 15 * 60,
            find_nodes_interval_secs: 33,
            find_nodes_skip_count: 32,
            max_resources: 50,
            max_peers_per_resource: 100,
            ping_check_interval_secs: 10,
            outgoing_request_prune_secs: 30,
            read_only: false,
            routers: vec![
                // "127.0.0.1:6111".to_string(),
                // "87.98.162.88:6881".to_string(),
                "dht.transmissionbt.com:6881".to_string(),
                "router.bittorrent.com:6881".to_string(),
                "router.utorrent.com:6881".to_string(),
            ],
            transaction_cleanup_interval_sec: 10,
            send_query_timeout_sec: 15,
            send_next_query_interval_sec: 1,
            token_refresh_interval_sec: 300,
            ip4_maintenance_interval_sec: 10,
        }
    }
}

#[derive(Clone, Default)]
/// Builder for DHTSettings
pub struct SettingsBuilder {
    settings: Settings,
}

macro_rules! make_builder_method {
    ($prop:ident, $prop_type:ty) => {
        pub fn $prop(mut self, $prop: $prop_type) -> Self {
            self.settings.$prop = $prop;
            self
        }
    };
}

impl SettingsBuilder {
    pub fn new() -> SettingsBuilder {
        Self::default()
    }
    make_builder_method!(ctx_index, i32);
    make_builder_method!(token_secret_size, usize);
    make_builder_method!(max_peers_response, usize);
    make_builder_method!(router_ping_interval_secs, u64);
    make_builder_method!(reverify_interval_secs, u64);
    make_builder_method!(reverify_grace_period_secs, u64);
    make_builder_method!(verify_grace_period_secs, u64);
    make_builder_method!(get_peers_freshness_secs, u64);
    make_builder_method!(find_nodes_interval_secs, u64);
    make_builder_method!(find_nodes_skip_count, usize);
    make_builder_method!(max_resources, usize);
    make_builder_method!(max_peers_per_resource, usize);
    make_builder_method!(ping_check_interval_secs, u64);
    make_builder_method!(outgoing_request_prune_secs, u64);
    make_builder_method!(read_only, bool);
    make_builder_method!(transaction_cleanup_interval_sec, u64);
    make_builder_method!(send_query_timeout_sec, u64);
    make_builder_method!(send_next_query_interval_sec, u64);
    make_builder_method!(token_refresh_interval_sec, u64);
    make_builder_method!(ip4_maintenance_interval_sec, u64);

    make_builder_method!(timeout_block_duration_sec, u64);
    make_builder_method!(reply_error_block_duration_sec, u64);
    make_builder_method!(firewall_block_duration_sec, u64);
    
    pub fn routers(mut self, router_list: &Option<Vec<String>>) -> Self {
        if let Some(router_list) = router_list {
            self.settings.routers = router_list.clone();
        }

        self
    }

    pub fn build(self) -> Settings {
        self.settings
    }
}
