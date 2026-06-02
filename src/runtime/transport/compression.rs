use crate::runtime::transport::common::errors;
/// Message compression utilities for transport layer.
///
/// This module provides optional compression support for envelopes,
/// reducing network bandwidth usage. Compression is applied transparently
/// during send and automatically decompressed on receive.
use flate2::Compression;
use std::io::{Read, Write};

/// Compression settings for the transport layer
#[derive(Debug, Clone, Copy)]
pub struct CompressionConfig {
    /// Enable compression
    pub enabled: bool,
    /// Compression level (0-9, where 0 is no compression and 9 is maximum)
    pub level: u32,
    /// Minimum payload size to compress (bytes). Smaller payloads skipped.
    pub min_size: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        CompressionConfig {
            enabled: true,
            level: 6,      // Default gzip compression level
            min_size: 128, // Only compress payloads > 128 bytes
        }
    }
}

impl CompressionConfig {
    /// Creates a compression config with no compression
    pub fn disabled() -> Self {
        CompressionConfig {
            enabled: false,
            level: 0,
            min_size: 0,
        }
    }

    /// Creates a compression config with maximum compression
    pub fn maximum() -> Self {
        CompressionConfig {
            enabled: true,
            level: 9,
            min_size: 64,
        }
    }

    /// Creates a compression config with fast compression
    pub fn fast() -> Self {
        CompressionConfig {
            enabled: true,
            level: 1,
            min_size: 256,
        }
    }
}

/// Compresses data using gzip
///
/// # Arguments
/// * `data` - The data to compress
/// * `config` - Compression configuration
///
/// # Returns
/// Compressed data, or original data if compression is disabled or ineffective
pub fn compress(data: &[u8], config: CompressionConfig) -> Result<Vec<u8>, String> {
    if !config.enabled || data.len() < config.min_size {
        return Ok(data.to_vec());
    }

    let compression = match config.level {
        0 => Compression::none(),
        1 => Compression::fast(),
        9 => Compression::best(),
        level => Compression::new(level),
    };

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), compression);
    encoder
        .write_all(data)
        .map_err(|e| errors::compression_write_failed(&e))?;

    encoder
        .finish()
        .map_err(|e| errors::compression_finish_failed(&e))
}

/// Decompresses data using gzip
///
/// # Arguments
/// * `data` - The compressed data
///
/// # Returns
/// Decompressed data, or error if decompression fails
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut decompressed = Vec::new();

    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| errors::decompression_failed(&e))?;

    Ok(decompressed)
}

/// Calculates compression ratio
///
/// Returns the ratio of compressed size to original size (0.0 = best, 1.0 = no compression)
pub fn compression_ratio(original_size: usize, compressed_size: usize) -> f64 {
    if original_size == 0 {
        0.0
    } else {
        compressed_size as f64 / original_size as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression_config_default_is_enabled() {
        let config = CompressionConfig::default();
        assert!(config.enabled);
        assert_eq!(config.level, 6);
        assert_eq!(config.min_size, 128);
    }

    #[test]
    fn compression_disabled_returns_original() {
        let data = b"test data";
        let config = CompressionConfig::disabled();
        let result = compress(data, config).expect("should compress");
        assert_eq!(result, data);
    }

    #[test]
    fn compression_respects_min_size() {
        let data = b"small";
        let config = CompressionConfig {
            enabled: true,
            level: 6,
            min_size: 1024, // Minimum 1KB
        };
        let result = compress(data, config).expect("should compress");
        assert_eq!(result, data); // Should return original since too small
    }

    #[test]
    fn compresses_and_decompresses_correctly() {
        let original = b"hello world ".repeat(100); // Create a large, repetitive payload
        let config = CompressionConfig::default();

        let compressed = compress(&original, config).expect("should compress");
        assert!(
            compressed.len() < original.len(),
            "compressed should be smaller"
        );

        let decompressed = decompress(&compressed).expect("should decompress");
        assert_eq!(decompressed, original);
    }

    #[test]
    fn compression_ratio_calculated_correctly() {
        let ratio = compression_ratio(1000, 500);
        assert_eq!(ratio, 0.5);

        let ratio = compression_ratio(1000, 1000);
        assert_eq!(ratio, 1.0);

        let ratio = compression_ratio(0, 100);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn maximum_compression_level_compresses_better() {
        let data = &b"repetitive data ".repeat(100);
        let config_normal = CompressionConfig::default();
        let config_max = CompressionConfig::maximum();

        let compressed_normal = compress(data, config_normal).expect("should compress");
        let compressed_max = compress(data, config_max).expect("should compress");

        // Maximum compression should produce smaller or equal output
        assert!(compressed_max.len() <= compressed_normal.len());
    }
}
