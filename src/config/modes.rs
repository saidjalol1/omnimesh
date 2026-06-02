#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoMode {
    Optional,
    Disabled,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WcetMode {
    Log,
    HardFail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceMode {
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    Mock,
    Tcp,
    Quic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DidRegistryMode {
    Direct {
        max_entries: usize,
    },
    Hierarchical {
        local_cache_size: usize,
        enable_discovery: bool,
    },
    OffChain {
        resolver_url: alloc::string::String,
        cache_ttl_seconds: u64,
    },
}

#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct OmniMeshConfig {
    pub node_id: crate::envelope::Did,
    pub listen_addresses: alloc::vec::Vec<std::net::SocketAddr>,
    pub known_peers: alloc::vec::Vec<crate::envelope::Did>,
    pub mode: OmnimeshMode,
    pub dtn_path: Option<alloc::string::String>,
}

/// Layer kind names for consistent identification across the runtime
pub mod layer_kinds {
    // Transport layer kinds
    pub const MOCK_TRANSPORT: &str = "mock transport";
    pub const TCP_TRANSPORT: &str = "tcp transport";
    pub const QUIC_TRANSPORT: &str = "quic transport";

    // Security layer kinds
    pub const OPTIONAL_SECURITY: &str = "optional security";
    pub const MINIMAL_SECURITY: &str = "minimal security";
    pub const STANDARD_SECURITY: &str = "standard security";

    // Storage layer kinds
    pub const DEVELOPMENT_STORAGE: &str = "development storage";
    pub const EPHEMERAL_STORAGE: &str = "ephemeral storage";
    pub const PERSISTENT_STORAGE: &str = "persistent storage";

    // Delivery layer kinds
    pub const BEST_EFFORT_DELIVERY: &str = "best-effort delivery";
    pub const LIGHTWEIGHT_DELIVERY: &str = "lightweight delivery";
    pub const RELIABLE_DELIVERY: &str = "reliable delivery";
}

#[derive(Debug, Clone)]
pub struct DevelopmentConfig {
    pub strict_wcet_enforcement: bool,
    pub dynamic_did_registry: bool,
    pub crypto_signatures: CryptoMode,
    pub persistence: PersistenceMode,
    pub buffer_pool_size: usize,
    pub buffer_capacity: usize,
}

#[derive(Debug, Clone)]
pub struct LightweightConfig {
    pub crypto_enabled: bool,
    pub exactly_once_enabled: bool,
    pub ordering_enabled: bool,
    pub buffer_pool_size: usize,
    pub buffer_capacity: usize,
    pub no_std: bool,
}

#[derive(Debug, Clone)]
pub struct ProductionConfig {
    pub crypto_enabled: bool,
    pub exactly_once_enabled: bool,
    pub ordering_enabled: bool,
    pub dtn_enabled: bool,
    pub dtn_path: Option<alloc::string::String>,
}

#[derive(Debug, Clone)]
pub enum OmnimeshMode {
    Development(DevelopmentConfig),
    Lightweight(LightweightConfig),
    Production(ProductionConfig),
}

impl Default for OmnimeshMode {
    fn default() -> Self {
        OmnimeshMode::Development(DevelopmentConfig {
            strict_wcet_enforcement: false,
            dynamic_did_registry: true,
            crypto_signatures: CryptoMode::Optional,
            persistence: PersistenceMode::Enabled,
            buffer_pool_size: 1024,
            buffer_capacity: 8192,
        })
    }
}

impl OmnimeshMode {
    pub fn development() -> Self {
        OmnimeshMode::Development(DevelopmentConfig {
            strict_wcet_enforcement: false,
            dynamic_did_registry: true,
            crypto_signatures: CryptoMode::Optional,
            persistence: PersistenceMode::Enabled,
            buffer_pool_size: 1024,
            buffer_capacity: 8192,
        })
    }

    pub fn lightweight() -> Self {
        OmnimeshMode::Lightweight(LightweightConfig {
            crypto_enabled: false,
            exactly_once_enabled: true,
            ordering_enabled: true,
            buffer_pool_size: 256,
            buffer_capacity: 1500,
            no_std: true,
        })
    }

    pub fn production() -> Self {
        OmnimeshMode::Production(ProductionConfig {
            crypto_enabled: true,
            exactly_once_enabled: true,
            ordering_enabled: true,
            dtn_enabled: true,
            dtn_path: Some("/tmp/omnimesh-dtn".into()),
        })
    }

    pub fn transport_type(&self) -> TransportType {
        match self {
            OmnimeshMode::Development(_) => TransportType::Mock,
            OmnimeshMode::Lightweight(_) => TransportType::Tcp,
            OmnimeshMode::Production(_) => TransportType::Tcp, // Changed to TCP for stability
        }
    }
}
