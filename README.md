# OMNI-MESH

**Zero-allocation mesh networking middleware for autonomous robot fleets, edge-AI swarms, and multi-agent systems.**

Written in Rust. Cryptographically signed. Production-ready.

> **Version:** 1.0.1 &nbsp;|&nbsp; **Minimum Rust:** 1.85 &nbsp;|&nbsp; **License:** MIT

---

## What is OMNI-MESH?

OMNI-MESH is a decentralized peer-to-peer messaging layer designed for robotics and edge-AI deployments where latency, reliability, and security matter. Every node has a cryptographic identity (DID), every message is signed, and delivery is exactly-once with ordering guarantees.

**Use cases:**
- Multi-robot fleet coordination (warehouse, logistics, agriculture)
- Edge-AI inference mesh (distributed LLM, sensor fusion)
- Autonomous vehicle V2V communication
- Industrial IoT with real-time constraints
- Federated learning model distribution

## Key Features

| Feature | Description |
|---------|-------------|
| Ed25519 Identity | Every node has a DID derived from its Ed25519 public key |
| Signed Envelopes | All messages are cryptographically signed and verified |
| Exactly-Once Delivery | Deduplication + ordered delivery with gap buffering |
| Pluggable Transport | Mock (testing), TCP (lightweight), QUIC (production) |
| Zero-Allocation Hot Path | Fixed-size buffers, no heap allocation in message pipeline |
| Gossip Routing | Decentralized peer discovery via UDP gossip protocol |
| DTN Support | Delay-Tolerant Networking for intermittent connectivity |
| WCET Enforcement | Worst-Case Execution Time guards with CPU pinning |
| Prometheus Metrics | Full observability with latency histograms |
| Python Bindings | PyO3-based SDK for ML/robotics teams |
| Graceful Shutdown | Signal handling, drain queues, clean exit |
| Health Checks | Built-in liveness probes for orchestration |
| Android Support | Cross-compile for ARM64 Android via `--features android` |

## Quick Start

```rust
use std::time::Duration;
use omnimesh::client::{OmnimeshClient, ClientConfig};
use omnimesh::payload;

// Create two nodes
let robot = OmnimeshClient::builder()
    .with_config(ClientConfig::development())
    .build()
    .expect("Failed to build robot");

let controller = OmnimeshClient::builder()
    .with_config(ClientConfig::development())
    .build()
    .expect("Failed to build controller");

// Send a motion command
let cmd = payload::motion_command(1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 100_000);
controller.send(robot.did, cmd).unwrap();

// Receive and process
if let Some(msg) = robot.receive_timeout(Duration::from_secs(1)) {
    println!("Received: {:?}", msg.payload);
}

// Health monitoring
let health = robot.health();
assert!(health.is_healthy());
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Developer SDK                        │
│  OmnimeshClient (send/receive/health/shutdown)       │
├─────────────────────────────────────────────────────┤
│                  Security Layer                       │
│  Ed25519 signing + verification (mode-dependent)     │
├─────────────────────────────────────────────────────┤
│                  Delivery Layer                       │
│  Exactly-once dedup + ordered delivery + DTN         │
├─────────────────────────────────────────────────────┤
│                  Transport Layer                      │
│  Mock | TCP | QUIC (TLS 1.3) + Compression           │
├─────────────────────────────────────────────────────┤
│                  Routing Layer                        │
│  DID→SocketAddr table + UDP gossip discovery         │
├─────────────────────────────────────────────────────┤
│                  Buffer Layer                         │
│  Zero-alloc: RingBuffer, FixedMap, PayloadStorage    │
└─────────────────────────────────────────────────────┘
```

## Operational Modes

| Mode | Transport | Crypto | Delivery | Use Case |
|------|-----------|--------|----------|----------|
| Development | Mock (in-process) | Optional | Best-effort | Testing, CI |
| Lightweight | TCP | Minimal | Lightweight | Embedded, constrained |
| Production | TCP/QUIC | Required | Reliable + DTN | Fleet deployment |

> **Development mode is zero-network** — all message exchange happens through an in-memory bus (`MockBus`). No sockets, no I/O. Safe for CI and unit tests.

## Running

```bash
# Build (requires Rust >= 1.85)
cargo build --release --features std

# Run daemon with TOML config
cargo run --release --features std -- --config omni-mesh.toml

# Run examples
cargo run --features std --example ping_pong
cargo run --features std --example warehouse_fleet

# Run tests (138 tests)
cargo test --workspace

# Run benchmarks
cargo bench
```

## Cross-Platform Build

```bash
# Standard desktop/server (Linux, Windows, macOS)
cargo build --release --features std

# Android ARM64 (no sled, no OS signals, no CPU affinity)
cargo ndk --target aarch64-linux-android --platform 30 -- build \
    --no-default-features --features android

# Python wheels (all platforms, Python 3.8+)
cd bindings/python && maturin build --release
```

## Python SDK

```python
import omnimesh

# Create a client
client = omnimesh.Client(mode="development")
print(f"My DID: {client.did}")

# Send a command
client.send_agent_command(
    target_did_hex="<64-char-hex>",
    command_type="pick",
    target_id=b"robot-1",
    payload_data=b"shelf-A12"
)

# Receive messages
msg = client.receive(timeout_ms=5000)
if msg:
    print(f"Got {msg['type']} from {msg['sender_did']}")
```

## Configuration

```toml
# omni-mesh.toml
[core]
mode = "production"

[node]
node_id = "node-1"
# known_peers = ["192.168.1.42:9000", "192.168.1.43:9000"]

[node.transport]
type = "tcp"
tcp_listen_addr = "0.0.0.0:9000"
tcp_connect_addr = "127.0.0.1:9001"
quic_listen_addr = "0.0.0.0:9443"
max_read_buffer = 65536      # bytes, default 4096

[routing]
max_routes = 1024
gossip_interval_ms = 1000    # peer announcement frequency (ms)
gossip_bind_addr = "0.0.0.0:9999"
multicast_addr = "224.0.0.1:9999"   # gossip multicast group
```

## Production Deployment Checklist

- [x] Graceful shutdown via Ctrl+C / SIGTERM
- [x] Health check API (`client.health()`)
- [x] Back-pressure with configurable inbox capacity
- [x] Metrics: messages sent/received/dropped
- [x] Condvar-based efficient waiting (no busy-polling)
- [x] Named threads for debugging
- [x] Structured JSON logging
- [x] Ed25519 signature enforcement in production mode
- [x] Exactly-once delivery with persistent deduplication
- [x] 138 tests including concurrency, crash recovery, and edge cases
- [x] Multi-OS CI (Linux, Windows, macOS, Android cross-compile)
- [x] Little-endian wire protocol (correct cross-platform byte order)
- [x] Config-driven gossip interval and multicast address

## Test Coverage

| Test Suite | Tests | Coverage |
|-----------|-------|----------|
| Unit tests (lib) | 66 | Transport, routing, security, storage, config |
| Buffer tests | 2 | Fixed-size data structures |
| Crash recovery | 7 | Restart resilience, memory pressure |
| Delivery | 1 | Ordered delivery pipeline |
| Envelope | 2 | Serialization roundtrip, endianness |
| Flow control | 6 | Back-pressure, QUIC + TCP rate limiting |
| Integration | 3 | Multi-layer pipeline |
| Live network | 3 | TCP/QUIC transport on localhost |
| Multi-node | 6 | Fleet coordination, broadcast, ring topology |
| Persistent dedup | 3 | DTN store deduplication |
| Production edge cases | 28 | Shutdown, concurrency, crypto, overflow |
| SDK | 7 | Client API surface |
| Security | 2 | Signature verification |
| Storage | 2 | Persistent storage layer |
| **Total** | **138** | |

## Contributing

PRs welcome. Run `cargo test --workspace` and `cargo clippy --workspace` before submitting.

**Requirements:**
- Rust >= 1.85 (declared in `rust-toolchain.toml` and `Cargo.toml`)
- For Android cross-compile: Android NDK + `cargo install cargo-ndk`
- For Python wheels: `pip install maturin`
