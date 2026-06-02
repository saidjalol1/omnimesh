//! Unit tests for message compression

#![allow(unused_imports)]

use crate::runtime::transport::compression::{
    CompressionConfig, compress, compression_ratio, decompress,
};

#[test]
fn compression_config_default_enabled() {
    let config = CompressionConfig::default();
    assert!(config.enabled);
    assert_eq!(config.level, 6);
}

#[test]
fn compression_config_disabled_works() {
    let config = CompressionConfig::disabled();
    assert!(!config.enabled);
}

#[test]
fn compression_config_fast_preset() {
    let config = CompressionConfig::fast();
    assert!(config.enabled);
    assert_eq!(config.level, 1);
}

#[test]
fn compression_config_maximum_preset() {
    let config = CompressionConfig::maximum();
    assert!(config.enabled);
    assert_eq!(config.level, 9);
}

#[test]
fn compress_disabled_returns_original() {
    let data = b"test data for compression";
    let config = CompressionConfig::disabled();
    let result = compress(data, config).expect("should compress");
    assert_eq!(result, data.to_vec());
}

#[test]
fn compress_below_minimum_size() {
    let data = b"small";
    let config = CompressionConfig {
        enabled: true,
        level: 6,
        min_size: 1024, // Larger than data
    };
    let result = compress(data, config).expect("should compress");
    assert_eq!(result, data.to_vec());
}

#[test]
fn compress_decompresses_correctly() {
    let original = b"Hello, World! ".repeat(100);
    let config = CompressionConfig::default();

    let compressed = compress(&original, config).expect("should compress");
    assert!(
        compressed.len() < original.len(),
        "compressed should be smaller"
    );

    let decompressed = decompress(&compressed).expect("should decompress");
    assert_eq!(decompressed, original.to_vec());
}

#[test]
fn compression_ratio_valid() {
    let ratio = compression_ratio(1000, 500);
    assert_eq!(ratio, 0.5);

    let ratio = compression_ratio(1000, 1000);
    assert_eq!(ratio, 1.0);

    let ratio = compression_ratio(0, 100);
    assert_eq!(ratio, 0.0);
}

#[test]
fn maximum_compression_reduces_size_most() {
    let data = &b"repetitive data for compression testing ".repeat(50);

    let config_normal = CompressionConfig::default();
    let config_max = CompressionConfig::maximum();
    let config_fast = CompressionConfig::fast();

    let compressed_normal = compress(data, config_normal).expect("should compress");
    let compressed_max = compress(data, config_max).expect("should compress");
    let compressed_fast = compress(data, config_fast).expect("should compress");

    // Maximum should be <= normal, fast might be larger but still compresses
    assert!(compressed_max.len() <= compressed_normal.len());
    assert!(compressed_fast.len() > 0);
}
