use super::common::{certificate, errors};
/// Certificate utilities for QUIC transport.
///
/// This module provides helpers for certificate generation and management
/// for QUIC-based secure transports.
use std::fs;

/// Configuration for certificate generation.
#[derive(Debug, Clone)]
pub struct CertificateConfig {
    /// Subject Common Name
    pub cn: String,
    /// Organization name
    pub org: String,
    /// Country code (e.g., "US")
    pub country: String,
    /// Validity duration in days
    pub validity_days: u32,
}

impl Default for CertificateConfig {
    fn default() -> Self {
        Self {
            cn: certificate::DEFAULT_CN.to_string(),
            org: certificate::DEFAULT_ORG.to_string(),
            country: certificate::DEFAULT_COUNTRY.to_string(),
            validity_days: certificate::DEFAULT_VALIDITY_DAYS,
        }
    }
}

/// Represents a certificate and private key pair
#[derive(Debug, Clone)]
pub struct CertificatePair {
    /// DER-encoded certificate
    pub cert_der: Vec<u8>,
    /// DER-encoded private key
    pub key_der: Vec<u8>,
}

impl CertificatePair {
    /// Generates a new self-signed certificate using rcgen
    ///
    /// Creates an X.509 self-signed certificate suitable for QUIC.
    /// This is appropriate for development and testing; production deployments
    /// should use proper certificate infrastructure (CA-signed certificates).
    pub fn generate_self_signed(_config: CertificateConfig) -> Result<Self, String> {
        // Build subject alternative names (SANs) from constants
        let subject_alt_names = certificate::ALT_NAMES
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        // Generate self-signed certificate
        let cert = rcgen::generate_simple_self_signed(subject_alt_names)
            .map_err(|e| errors::cert_generation_failed(&e))?;

        // Serialize certificate and key to DER format
        let cert_der = cert.cert.der().to_vec();
        let key_der = cert.key_pair.serialize_der();

        Ok(CertificatePair { cert_der, key_der })
    }

    /// Loads a certificate from PEM format strings
    ///
    /// Parses PEM-encoded certificate and private key.
    pub fn from_pem(cert_pem: &str, key_pem: &str) -> Result<Self, String> {
        // Parse certificate from PEM
        let mut cert_reader = cert_pem.as_bytes();
        let certs: Result<Vec<_>, _> = rustls_pemfile::certs(&mut cert_reader).collect();

        let certs = certs.map_err(|e| errors::cert_parse_failed(&e))?;

        let cert_der = certs
            .into_iter()
            .next()
            .ok_or("No certificate found in PEM")?
            .to_vec();

        // Parse private key from PEM
        let mut key_reader = key_pem.as_bytes();
        let private_key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(|e| errors::key_parse_failed(&e))?
            .ok_or("No private key found in PEM")?;

        let key_der = private_key.secret_der().to_vec();

        Ok(CertificatePair { cert_der, key_der })
    }

    /// Loads a certificate from DER files on disk
    ///
    /// Reads certificate and private key from separate DER files.
    pub fn from_der_files(cert_path: &str, key_path: &str) -> Result<Self, String> {
        let cert_der =
            fs::read(cert_path).map_err(|e| errors::cert_file_read_failed(cert_path, &e))?;

        let key_der = fs::read(key_path).map_err(|e| errors::key_file_read_failed(key_path, &e))?;

        Ok(CertificatePair { cert_der, key_der })
    }

    /// Returns the certificate in DER format as bytes
    pub fn cert_der(&self) -> &[u8] {
        &self.cert_der
    }

    /// Returns the private key in DER format as bytes
    pub fn key_der(&self) -> &[u8] {
        &self.key_der
    }

    /// Checks if the certificate is valid
    pub fn is_valid(&self) -> bool {
        !self.cert_der.is_empty() && !self.key_der.is_empty()
    }
}
