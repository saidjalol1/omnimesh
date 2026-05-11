//! Configuration file loader for OMNI-MESH.
//!
//! Parses `omni-mesh.toml` into runtime configuration.

use serde::Deserialize;
use std::path::Path;

use crate::config::OmnimeshMode;
use crate::config::modes::*;

/// Top-level configuration file structure
#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub core: CoreConfig,
    #[serde(default)]
    pub node: NodeConfig,
    #[serde(default)]
    pub mode: ModeConfigs,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default)]
    pub wcet: WcetConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct NodeConfig {
    #[serde(default)]
    pub node_id: String,
    #[serde(default)]
    pub listen_addresses: Vec<String>,
    #[serde(default)]
    pub known_peers: Vec<String>,
    #[serde(default)]
    pub transport: NodeTransportConfig,
}

/// Transport configuration parsed from [node.transport] section
#[derive(Debug, Deserialize, Clone)]
pub struct NodeTransportConfig {
    /// Transport type: "tcp", "quic", or "both"
    #[serde(default = "default_transport_type", rename = "type")]
    pub transport_type: String,
    /// TCP listen address
    #[serde(default = "default_tcp_listen")]
    pub tcp_listen_addr: String,
    /// TCP default connect address (fallback target)
    #[serde(default = "default_tcp_connect")]
    pub tcp_connect_addr: String,
    /// QUIC listen address
    #[serde(default = "default_quic_listen")]
    pub quic_listen_addr: String,
    /// Max read buffer size
    #[serde(default = "default_max_read_buffer")]
    pub max_read_buffer: usize,
}

impl Default for NodeTransportConfig {
    fn default() -> Self {
        Self {
            transport_type: "tcp".into(),
            tcp_listen_addr: "0.0.0.0:9000".into(),
            tcp_connect_addr: "127.0.0.1:9001".into(),
            quic_listen_addr: "0.0.0.0:9443".into(),
            max_read_buffer: 1048576,
        }
    }
}

impl NodeTransportConfig {
    /// Convert to runtime TransportConfig
    pub fn to_transport_config(&self) -> Result<crate::runtime::transport::config::TransportConfig, String> {
        let tcp_listen = self.tcp_listen_addr.parse()
            .map_err(|e| format!("Invalid tcp_listen_addr '{}': {}", self.tcp_listen_addr, e))?;
        let tcp_connect = self.tcp_connect_addr.parse()
            .map_err(|e| format!("Invalid tcp_connect_addr '{}': {}", self.tcp_connect_addr, e))?;
        let quic_listen = self.quic_listen_addr.parse()
            .map_err(|e| format!("Invalid quic_listen_addr '{}': {}", self.quic_listen_addr, e))?;
        
        let mut config = crate::runtime::transport::config::TransportConfig::new(
            tcp_listen, tcp_connect, quic_listen,
        );
        config.max_read_buffer = self.max_read_buffer;
        Ok(config)
    }
}

#[derive(Debug, Deserialize)]
pub struct CoreConfig {
    pub mode: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct ModeConfigs {
    #[serde(default)]
    pub development: DevModeConfig,
    #[serde(default)]
    pub lightweight: LightModeConfig,
    #[serde(default)]
    pub production: ProdModeConfig,
}

#[derive(Debug, Deserialize)]
pub struct DevModeConfig {
    #[serde(default = "default_crypto_optional")]
    pub crypto: String,
    #[serde(default = "default_wcet_log")]
    pub wcet_enforcement: String,
    #[serde(default = "default_true")]
    pub dynamic_did_registry: bool,
    #[serde(default = "default_pool_size")]
    pub buffer_pool_size: usize,
    #[serde(default = "default_buffer_capacity")]
    pub buffer_capacity: usize,
}

impl Default for DevModeConfig {
    fn default() -> Self {
        Self {
            crypto: "optional".into(),
            wcet_enforcement: "log".into(),
            dynamic_did_registry: true,
            buffer_pool_size: 1024,
            buffer_capacity: 8192,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LightModeConfig {
    #[serde(default)]
    pub crypto_enabled: bool,
    #[serde(default = "default_true")]
    pub exactly_once: bool,
    #[serde(default = "default_true")]
    pub ordering: bool,
    #[serde(default = "default_light_pool")]
    pub buffer_pool_size: usize,
    #[serde(default = "default_light_capacity")]
    pub buffer_capacity: usize,
    #[serde(default = "default_true")]
    pub no_std: bool,
}

impl Default for LightModeConfig {
    fn default() -> Self {
        Self {
            crypto_enabled: false,
            exactly_once: true,
            ordering: true,
            buffer_pool_size: 256,
            buffer_capacity: 1500,
            no_std: true,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ProdModeConfig {
    #[serde(default = "default_true")]
    pub crypto_enabled: bool,
    #[serde(default = "default_true")]
    pub exactly_once: bool,
    #[serde(default = "default_true")]
    pub ordering: bool,
    #[serde(default = "default_true")]
    pub dtn_enabled: bool,
    pub dtn_path: Option<String>,
}

impl Default for ProdModeConfig {
    fn default() -> Self {
        Self {
            crypto_enabled: true,
            exactly_once: true,
            ordering: true,
            dtn_enabled: true,
            dtn_path: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RoutingConfig {
    #[serde(default = "default_32")]
    pub max_neighbors: usize,
    #[serde(default = "default_1024")]
    pub max_routing_entries: usize,
    #[serde(default = "default_1000")]
    pub gossip_interval_ms: u64,
    #[serde(default = "default_10000")]
    pub dtn_max_messages: u32,
    #[serde(default = "default_1024_u64")]
    pub dtn_max_bytes_mb: u64,
    /// Gossip UDP bind address
    #[serde(default = "default_gossip_bind")]
    pub gossip_bind_addr: String,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            max_neighbors: 32,
            max_routing_entries: 1024,
            gossip_interval_ms: 1000,
            dtn_max_messages: 10000,
            dtn_max_bytes_mb: 1024,
            gossip_bind_addr: "0.0.0.0:9999".into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WcetConfig {
    #[serde(default = "default_500")]
    pub ed25519_verify_us: u64,
    #[serde(default = "default_100")]
    pub blake3_hash_us_per_kb: u64,
    #[serde(default = "default_1000")]
    pub total_pipeline_us: u64,
}

impl Default for WcetConfig {
    fn default() -> Self {
        Self {
            ed25519_verify_us: 500,
            blake3_hash_us_per_kb: 100,
            total_pipeline_us: 1000,
        }
    }
}

// Serde default helpers
fn default_true() -> bool { true }
fn default_crypto_optional() -> String { "optional".into() }
fn default_wcet_log() -> String { "log".into() }
fn default_pool_size() -> usize { 1024 }
fn default_buffer_capacity() -> usize { 8192 }
fn default_light_pool() -> usize { 256 }
fn default_light_capacity() -> usize { 1500 }
fn default_32() -> usize { 32 }
fn default_1024() -> usize { 1024 }
fn default_1024_u64() -> u64 { 1024 }
fn default_1000() -> u64 { 1000 }
fn default_10000() -> u32 { 10000 }
fn default_500() -> u64 { 500 }
fn default_100() -> u64 { 100 }
fn default_transport_type() -> String { "tcp".into() }
fn default_tcp_listen() -> String { "0.0.0.0:9000".into() }
fn default_tcp_connect() -> String { "127.0.0.1:9001".into() }
fn default_quic_listen() -> String { "0.0.0.0:9443".into() }
fn default_max_read_buffer() -> usize { 1048576 }
fn default_gossip_bind() -> String { "0.0.0.0:9999".into() }

impl ConfigFile {
    /// Load configuration from a TOML file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))
    }

    /// Convert parsed config into an `OmnimeshMode`.
    pub fn to_mode(&self) -> Result<OmnimeshMode, String> {
        match self.core.mode.as_str() {
            "development" => {
                let crypto = match self.mode.development.crypto.as_str() {
                    "optional" => CryptoMode::Optional,
                    "disabled" => CryptoMode::Disabled,
                    "required" => CryptoMode::Required,
                    other => return Err(format!("Unknown crypto mode: {}", other)),
                };
                Ok(OmnimeshMode::Development(DevelopmentConfig {
                    strict_wcet_enforcement: self.mode.development.wcet_enforcement == "hard_fail",
                    dynamic_did_registry: self.mode.development.dynamic_did_registry,
                    crypto_signatures: crypto,
                    persistence: PersistenceMode::Enabled,
                    buffer_pool_size: self.mode.development.buffer_pool_size,
                    buffer_capacity: self.mode.development.buffer_capacity,
                }))
            }
            "lightweight" => {
                Ok(OmnimeshMode::Lightweight(LightweightConfig {
                    crypto_enabled: self.mode.lightweight.crypto_enabled,
                    exactly_once_enabled: self.mode.lightweight.exactly_once,
                    ordering_enabled: self.mode.lightweight.ordering,
                    buffer_pool_size: self.mode.lightweight.buffer_pool_size,
                    buffer_capacity: self.mode.lightweight.buffer_capacity,
                    no_std: self.mode.lightweight.no_std,
                }))
            }
            "production" => {
                Ok(OmnimeshMode::Production(ProductionConfig {
                    crypto_enabled: self.mode.production.crypto_enabled,
                    exactly_once_enabled: self.mode.production.exactly_once,
                    ordering_enabled: self.mode.production.ordering,
                    dtn_enabled: self.mode.production.dtn_enabled,
                    dtn_path: self.mode.production.dtn_path.as_ref().map(|s| s.into()),
                }))
            }
            other => Err(format!("Unknown mode: {}", other)),
        }
    }

    /// Convert parsed config into the full runtime configuration.
    pub fn to_runtime_config(&self) -> Result<crate::config::modes::OmniMeshConfig, String> {
        let mode = self.to_mode()?;
        
        // Parse Listen Addresses
        let mut listen_addresses = Vec::new();
        for addr_str in &self.node.listen_addresses {
            let addr = addr_str.parse::<std::net::SocketAddr>()
                .map_err(|e| format!("Invalid listen address '{}': {}", addr_str, e))?;
            listen_addresses.push(addr);
        }

        // Parse Node ID
        let node_id = if self.node.node_id.is_empty() {
            crate::envelope::Did::default()
        } else {
            // Mock DID parsing from hex for simplicity
            let mut bytes = [0u8; 32];
            let id_bytes = self.node.node_id.as_bytes();
            let len = std::cmp::min(id_bytes.len(), 32);
            bytes[..len].copy_from_slice(&id_bytes[..len]);
            crate::envelope::Did(bytes)
        };

        // Parse Known Peers
        let mut known_peers = Vec::new();
        for peer_str in &self.node.known_peers {
            let mut bytes = [0u8; 32];
            let id_bytes = peer_str.as_bytes();
            let len = std::cmp::min(id_bytes.len(), 32);
            bytes[..len].copy_from_slice(&id_bytes[..len]);
            known_peers.push(crate::envelope::Did(bytes));
        }
        
        let dtn_path = match &mode {
            OmnimeshMode::Production(p) => p.dtn_path.clone(),
            _ => None,
        };

        Ok(crate::config::modes::OmniMeshConfig {
            node_id,
            listen_addresses,
            known_peers,
            mode,
            dtn_path,
        })
    }
}
