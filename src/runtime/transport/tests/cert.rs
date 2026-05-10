//! Unit tests for certificate utilities

#![allow(unused_imports)]

use crate::runtime::transport::cert::{CertificateConfig, CertificatePair};

#[test]
fn certificate_config_has_defaults() {
    let config = CertificateConfig::default();
    assert_eq!(config.cn, "omnimesh.local");
    assert_eq!(config.org, "OMNI-MESH");
    assert_eq!(config.country, "US");
    assert_eq!(config.validity_days, 365);
}

#[test]
fn certificate_pair_validity_check() {
    let empty_pair = CertificatePair {
        cert_der: vec![],
        key_der: vec![],
    };
    assert!(!empty_pair.is_valid());

    let valid_pair = CertificatePair {
        cert_der: vec![1, 2, 3],
        key_der: vec![4, 5, 6],
    };
    assert!(valid_pair.is_valid());
}

#[test]
fn certificate_generation_creates_valid_cert() {
    let config = CertificateConfig::default();
    let cert = CertificatePair::generate_self_signed(config)
        .expect("should generate certificate");

    assert!(cert.is_valid());
    assert!(!cert.cert_der().is_empty());
    assert!(!cert.key_der().is_empty());
}

#[test]
fn generated_certificate_has_proper_der_format() {
    let config = CertificateConfig::default();
    let cert = CertificatePair::generate_self_signed(config)
        .expect("should generate certificate");

    // DER format starts with SEQUENCE tag (0x30)
    assert_eq!(cert.cert_der()[0], 0x30);
    assert!(cert.cert_der().len() > 100); // Typical X.509 cert is >100 bytes
}

#[test]
fn certificate_pair_accessors() {
    let cert_der = vec![1, 2, 3, 4, 5];
    let key_der = vec![6, 7, 8, 9, 10];

    let pair = CertificatePair {
        cert_der: cert_der.clone(),
        key_der: key_der.clone(),
    };

    assert_eq!(pair.cert_der(), cert_der.as_slice());
    assert_eq!(pair.key_der(), key_der.as_slice());
}
