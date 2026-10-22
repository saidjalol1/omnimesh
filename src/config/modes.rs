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

#[cfg(any(feature = "std", feature = "android"))]
#[derive(Debug, Clone)]
pub struct OmniMeshConfig {
    pub node_id: crate::envelope::Did,
    pub listen_addresses: alloc::vec::Vec<core::net::SocketAddr>,
    pub known_peers: alloc::vec::Vec<crate::envelope::Did>,
    pub mode: OmnimeshMode,
    pub dtn_path: Option<alloc::string::String>,
    // Transport addresses (from [node.transport] in TOML) — Bug 2 fix
    pub tcp_listen_addr: core::net::SocketAddr,
    pub tcp_connect_addr: core::net::SocketAddr,
    pub quic_listen_addr: core::net::SocketAddr,
    pub max_read_buffer: usize,
    // Gossip UDP bind address (from [routing] in TOML) — Bug 3 fix
    pub gossip_bind_addr: core::net::SocketAddr,
    /// Gossip discovery interval in milliseconds — from [routing] in TOML
    pub gossip_interval_ms: u64,
    /// Multicast/broadcast address for gossip discovery
    pub multicast_addr: core::net::SocketAddr,
}

#[cfg(any(feature = "std", feature = "android"))]
impl OmniMeshConfig {
    /// Build a minimal `OmniMeshConfig` from a mode, using built-in defaults
    /// for all network addresses.  Useful when no TOML config file is present.
    pub fn from_mode(mode: OmnimeshMode) -> Self {
        use ed25519_dalek::SigningKey;
        use rand_core::OsRng;
        use crate::envelope::Did;

        let signing_key = SigningKey::generate(&mut OsRng);
        let node_id = Did::new(signing_key.verifying_key().to_bytes());

        Self {
            node_id,
            listen_addresses: alloc::vec![],
            known_peers: alloc::vec![],
            mode,
            dtn_path: None, // No persistent DTN path by default; set explicitly if needed
            tcp_listen_addr: "0.0.0.0:9000".parse().unwrap(),
            tcp_connect_addr: "127.0.0.1:9001".parse().unwrap(),
            quic_listen_addr: "0.0.0.0:9443".parse().unwrap(),
            max_read_buffer: 4096,
            gossip_bind_addr: "0.0.0.0:9999".parse().unwrap(),
            gossip_interval_ms: 1000,
            multicast_addr: "224.0.0.1:9999".parse().unwrap(),
        }
    }
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
        #[cfg(feature = "std")]
        let dtn_path = Some(
            std::env::temp_dir()
                .join("omnimesh-dtn")
                .to_string_lossy()
                .into_owned(),
        );
        #[cfg(all(not(feature = "std"), feature = "android"))]
        let dtn_path = Some(alloc::string::String::from("/data/local/tmp/omnimesh-dtn"));
        #[cfg(not(any(feature = "std", feature = "android")))]
        let dtn_path: Option<alloc::string::String> = None;

        OmnimeshMode::Production(ProductionConfig {
            crypto_enabled: true,
            exactly_once_enabled: true,
            ordering_enabled: true,
            dtn_enabled: true,
            dtn_path,
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
