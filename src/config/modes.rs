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
    pub dtn_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CertifiedConfig {
    pub crypto_enabled: bool,
    pub exactly_once_enabled: bool,
    pub ordering_enabled: bool,
    pub wcet_enforcement: WcetMode,
    pub hsm_signer_required: bool,
    pub certified_build: bool,
}

#[derive(Debug, Clone)]
pub enum OmnimeshMode {
    Development(DevelopmentConfig),
    Lightweight(LightweightConfig),
    Production(ProductionConfig),
    Certified(CertifiedConfig),
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
            dtn_path: None,
        })
    }

    pub fn certified() -> Self {
        OmnimeshMode::Certified(CertifiedConfig {
            crypto_enabled: true,
            exactly_once_enabled: true,
            ordering_enabled: true,
            wcet_enforcement: WcetMode::HardFail,
            hsm_signer_required: true,
            certified_build: true,
        })
    }

    pub fn transport_type(&self) -> TransportType {
        match self {
            OmnimeshMode::Development(_) => TransportType::Mock,
            OmnimeshMode::Lightweight(_) => TransportType::Tcp,
            OmnimeshMode::Production(_) => TransportType::Quic,
            OmnimeshMode::Certified(_) => TransportType::Quic,
        }
    }
}
