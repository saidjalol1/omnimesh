//! ROS 2 Bridge for OMNI-MESH
//!
//! Provides bidirectional translation between OMNI-MESH payloads and ROS 2
//! standard message formats. The bridge operates over UDP, making it compatible
//! with any ROS 2 installation without requiring `ros2_rust` or DDS dependencies.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────┐     UDP (JSON)      ┌──────────────────┐
//! │  ROS 2 Node  │ ◄──────────────────► │  OMNI-MESH       │
//! │  (Python/C++) │                      │  Ros2Bridge      │
//! └──────────────┘                      └──────────────────┘
//! ```
//!
//! ## Supported ROS 2 Message Mappings
//!
//! | OMNI-MESH Payload      | ROS 2 Equivalent              |
//! |------------------------|-------------------------------|
//! | `MotionCommand`        | `geometry_msgs/msg/Twist`     |
//! | `Vector3`              | `geometry_msgs/msg/Vector3`   |
//! | `Transform`            | `geometry_msgs/msg/Transform` |
//! | `Heartbeat`            | `diagnostic_msgs/msg/DiagnosticStatus` |
//! | `Detection`            | `vision_msgs/msg/Detection2D` |
//! | `SensorFusionFrame`    | `sensor_msgs/msg/PointCloud2` (partial) |
//!
//! ## Usage
//!
//! ```rust,ignore
//! use omnimesh::runtime::ros2_bridge::{Ros2Bridge, Ros2BridgeConfig};
//!
//! let config = Ros2BridgeConfig {
//!     listen_addr: "0.0.0.0:9090".parse().unwrap(),
//!     ros2_target_addr: "127.0.0.1:9091".parse().unwrap(),
//! };
//!
//! let bridge = Ros2Bridge::new(config);
//! bridge.start(); // Spawns background translation tasks
//! ```

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::payload::*;

/// Configuration for the ROS 2 bridge.
#[derive(Debug, Clone)]
pub struct Ros2BridgeConfig {
    /// Address to listen for incoming ROS 2 messages (JSON over UDP)
    pub listen_addr: SocketAddr,
    /// Address to forward translated messages to ROS 2 nodes
    pub ros2_target_addr: SocketAddr,
    /// Maximum message size in bytes
    pub max_message_size: usize,
}

impl Default for Ros2BridgeConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:9090".parse().unwrap(),
            ros2_target_addr: "127.0.0.1:9091".parse().unwrap(),
            max_message_size: 65536,
        }
    }
}

/// ROS 2 standard message types serialized as JSON for bridge communication.
/// These mirror the official ROS 2 message definitions.
pub mod ros2_msgs {
    use serde::{Serialize, Deserialize};

    /// geometry_msgs/msg/Vector3
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Vector3 {
        pub x: f64,
        pub y: f64,
        pub z: f64,
    }

    /// geometry_msgs/msg/Twist
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Twist {
        pub linear: Vector3,
        pub angular: Vector3,
    }

    /// geometry_msgs/msg/Quaternion
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Quaternion {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub w: f64,
    }

    /// geometry_msgs/msg/Transform
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TransformMsg {
        pub translation: Vector3,
        pub rotation: Quaternion,
    }

    /// std_msgs/msg/Header
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Header {
        pub stamp: TimeStamp,
        pub frame_id: String,
    }

    /// builtin_interfaces/msg/Time
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TimeStamp {
        pub sec: i32,
        pub nanosec: u32,
    }

    /// vision_msgs/msg/BoundingBox2D
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BoundingBox2D {
        pub center_x: f64,
        pub center_y: f64,
        pub size_x: f64,
        pub size_y: f64,
    }

    /// vision_msgs/msg/Detection2D
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Detection2D {
        pub header: Header,
        pub id: String,
        pub bbox: BoundingBox2D,
        pub score: f64,
    }

    /// diagnostic_msgs/msg/DiagnosticStatus (used for heartbeat)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DiagnosticStatus {
        pub level: u8, // 0=OK, 1=WARN, 2=ERROR
        pub name: String,
        pub message: String,
        pub hardware_id: String,
        pub values: Vec<KeyValue>,
    }

    /// diagnostic_msgs/msg/KeyValue
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct KeyValue {
        pub key: String,
        pub value: String,
    }

    /// Wrapper envelope for bridge messages with type discrimination
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BridgeMessage {
        /// ROS 2 message type (e.g. "geometry_msgs/msg/Twist")
        pub msg_type: String,
        /// Topic name (e.g. "/cmd_vel")
        pub topic: String,
        /// JSON-encoded message payload
        pub data: serde_json::Value,
    }
}

/// Converts OMNI-MESH payloads to ROS 2 JSON messages.
pub fn to_ros2(payload: &EnvelopePayload) -> Option<ros2_msgs::BridgeMessage> {
    match payload.payload.as_ref()? {
        PayloadKind::MotionCommand(cmd) => {
            let linear = cmd.linear.as_ref().map(|v| ros2_msgs::Vector3 {
                x: v.x as f64, y: v.y as f64, z: v.z as f64,
            }).unwrap_or(ros2_msgs::Vector3 { x: 0.0, y: 0.0, z: 0.0 });

            let angular = cmd.angular.as_ref().map(|v| ros2_msgs::Vector3 {
                x: v.x as f64, y: v.y as f64, z: v.z as f64,
            }).unwrap_or(ros2_msgs::Vector3 { x: 0.0, y: 0.0, z: 0.0 });

            let twist = ros2_msgs::Twist { linear, angular };
            Some(ros2_msgs::BridgeMessage {
                msg_type: "geometry_msgs/msg/Twist".to_string(),
                topic: "/cmd_vel".to_string(),
                data: serde_json::to_value(&twist).ok()?,
            })
        }

        PayloadKind::Heartbeat(hb) => {
            let status = ros2_msgs::DiagnosticStatus {
                level: 0, // OK
                name: "omnimesh_node".to_string(),
                message: format!("uptime={}ms cpu={}% mem={}KB", hb.uptime_ms, hb.cpu_usage, hb.mem_usage_kb),
                hardware_id: hex::encode(&hb.sender_did),
                values: vec![
                    ros2_msgs::KeyValue { key: "uptime_ms".into(), value: hb.uptime_ms.to_string() },
                    ros2_msgs::KeyValue { key: "cpu_usage".into(), value: hb.cpu_usage.to_string() },
                    ros2_msgs::KeyValue { key: "mem_usage_kb".into(), value: hb.mem_usage_kb.to_string() },
                    ros2_msgs::KeyValue { key: "epoch".into(), value: hb.epoch.to_string() },
                ],
            };
            Some(ros2_msgs::BridgeMessage {
                msg_type: "diagnostic_msgs/msg/DiagnosticStatus".to_string(),
                topic: "/diagnostics".to_string(),
                data: serde_json::to_value(&status).ok()?,
            })
        }

        PayloadKind::SensorFusion(sf) => {
            let detections: Vec<ros2_msgs::Detection2D> = sf.detections.iter().map(|d| {
                let bbox = d.bbox.as_ref().map(|b| ros2_msgs::BoundingBox2D {
                    center_x: ((b.x_min + b.x_max) / 2.0) as f64,
                    center_y: ((b.y_min + b.y_max) / 2.0) as f64,
                    size_x: (b.x_max - b.x_min) as f64,
                    size_y: (b.y_max - b.y_min) as f64,
                }).unwrap_or(ros2_msgs::BoundingBox2D {
                    center_x: 0.0, center_y: 0.0, size_x: 0.0, size_y: 0.0,
                });

                let timestamp_sec = (sf.timestamp_us / 1_000_000) as i32;
                let timestamp_nsec = ((sf.timestamp_us % 1_000_000) * 1000) as u32;

                ros2_msgs::Detection2D {
                    header: ros2_msgs::Header {
                        stamp: ros2_msgs::TimeStamp { sec: timestamp_sec, nanosec: timestamp_nsec },
                        frame_id: sf.frame_id.clone(),
                    },
                    id: d.class.clone(),
                    bbox,
                    score: d.confidence as f64,
                }
            }).collect();

            Some(ros2_msgs::BridgeMessage {
                msg_type: "vision_msgs/msg/Detection2DArray".to_string(),
                topic: "/detections".to_string(),
                data: serde_json::to_value(&detections).ok()?,
            })
        }

        _ => None, // Other payload types don't have direct ROS 2 equivalents
    }
}

/// Converts a ROS 2 JSON bridge message to an OMNI-MESH payload.
pub fn from_ros2(msg: &ros2_msgs::BridgeMessage) -> Option<EnvelopePayload> {
    match msg.msg_type.as_str() {
        "geometry_msgs/msg/Twist" => {
            let twist: ros2_msgs::Twist = serde_json::from_value(msg.data.clone()).ok()?;
            Some(EnvelopePayload {
                payload: Some(PayloadKind::MotionCommand(MotionCommand {
                    linear: Some(Vector3 {
                        x: twist.linear.x as f32,
                        y: twist.linear.y as f32,
                        z: twist.linear.z as f32,
                    }),
                    angular: Some(Vector3 {
                        x: twist.angular.x as f32,
                        y: twist.angular.y as f32,
                        z: twist.angular.z as f32,
                    }),
                    deadline_ns: 0,
                })),
            })
        }

        "diagnostic_msgs/msg/DiagnosticStatus" => {
            let status: ros2_msgs::DiagnosticStatus = serde_json::from_value(msg.data.clone()).ok()?;
            let uptime_ms = status.values.iter()
                .find(|kv| kv.key == "uptime_ms")
                .and_then(|kv| kv.value.parse::<u64>().ok())
                .unwrap_or(0);
            let cpu = status.values.iter()
                .find(|kv| kv.key == "cpu_usage")
                .and_then(|kv| kv.value.parse::<u32>().ok())
                .unwrap_or(0);
            let mem_kb = status.values.iter()
                .find(|kv| kv.key == "mem_usage_kb")
                .and_then(|kv| kv.value.parse::<u32>().ok())
                .unwrap_or(0);
            let epoch = status.values.iter()
                .find(|kv| kv.key == "epoch")
                .and_then(|kv| kv.value.parse::<u64>().ok())
                .unwrap_or(0);

            let sender_did = hex::decode(&status.hardware_id).unwrap_or_default();

            Some(EnvelopePayload {
                payload: Some(PayloadKind::Heartbeat(Heartbeat {
                    sender_did,
                    uptime_ms,
                    cpu_usage: cpu,
                    mem_usage_kb: mem_kb,
                    epoch,
                })),
            })
        }

        _ => None,
    }
}

/// The ROS 2 Bridge runtime.
///
/// Listens for JSON-encoded ROS 2 messages on a UDP port and translates them
/// into OMNI-MESH payloads (and vice versa). This allows any ROS 2 node to
/// communicate with the OMNI-MESH network without native Rust ROS 2 bindings.
pub struct Ros2Bridge {
    config: Ros2BridgeConfig,
    /// Channel for sending OMNI-MESH payloads that came from ROS 2
    inbound_tx: mpsc::Sender<EnvelopePayload>,
    /// Channel for receiving OMNI-MESH payloads to forward to ROS 2
    outbound_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<EnvelopePayload>>>,
    outbound_tx: mpsc::Sender<EnvelopePayload>,
}

impl Ros2Bridge {
    /// Create a new ROS 2 bridge with the given configuration.
    ///
    /// Returns the bridge and a receiver for inbound payloads (from ROS 2 → OMNI-MESH).
    pub fn new(config: Ros2BridgeConfig) -> (Self, mpsc::Receiver<EnvelopePayload>) {
        let (inbound_tx, inbound_rx) = mpsc::channel(256);
        let (outbound_tx, outbound_rx) = mpsc::channel(256);

        let bridge = Self {
            config,
            inbound_tx,
            outbound_rx: Arc::new(tokio::sync::Mutex::new(outbound_rx)),
            outbound_tx,
        };

        (bridge, inbound_rx)
    }

    /// Get a sender to push OMNI-MESH payloads to ROS 2.
    pub fn outbound_sender(&self) -> mpsc::Sender<EnvelopePayload> {
        self.outbound_tx.clone()
    }

    /// Start the bridge (spawns async tasks for bidirectional translation).
    ///
    /// This must be called from within a Tokio runtime.
    pub async fn start(self) -> Result<(), String> {
        let socket = UdpSocket::bind(self.config.listen_addr).await
            .map_err(|e| format!("Failed to bind ROS 2 bridge socket: {}", e))?;

        let socket = Arc::new(socket);
        let target_addr = self.config.ros2_target_addr;
        let max_size = self.config.max_message_size;

        println!("ROS 2 Bridge started:");
        println!("  Listening on: {}", self.config.listen_addr);
        println!("  Forwarding to: {}", target_addr);

        // Task 1: Listen for ROS 2 → OMNI-MESH messages
        let inbound_tx = self.inbound_tx.clone();
        let recv_socket = socket.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; max_size];
            loop {
                match recv_socket.recv_from(&mut buf).await {
                    Ok((len, _src)) => {
                        // Parse JSON bridge message
                        match serde_json::from_slice::<ros2_msgs::BridgeMessage>(&buf[..len]) {
                            Ok(bridge_msg) => {
                                if let Some(payload) = from_ros2(&bridge_msg) {
                                    let _ = inbound_tx.send(payload).await;
                                }
                            }
                            Err(e) => {
                                eprintln!("ROS 2 Bridge: Invalid JSON from ROS 2: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("ROS 2 Bridge: recv error: {}", e);
                    }
                }
            }
        });

        // Task 2: OMNI-MESH → ROS 2 forwarding
        let send_socket = socket;
        let outbound_rx = self.outbound_rx;
        tokio::spawn(async move {
            let mut rx = outbound_rx.lock().await;
            while let Some(payload) = rx.recv().await {
                if let Some(bridge_msg) = to_ros2(&payload) {
                    match serde_json::to_vec(&bridge_msg) {
                        Ok(json_bytes) => {
                            let _ = send_socket.send_to(&json_bytes, target_addr).await;
                        }
                        Err(e) => {
                            eprintln!("ROS 2 Bridge: Failed to serialize: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motion_command_to_ros2_twist() {
        let payload = motion_command(1.5, 0.0, 0.0, 0.0, 0.0, 0.8, 100_000);
        let bridge_msg = to_ros2(&payload).expect("Should convert to ROS 2");

        assert_eq!(bridge_msg.msg_type, "geometry_msgs/msg/Twist");
        assert_eq!(bridge_msg.topic, "/cmd_vel");

        let twist: ros2_msgs::Twist = serde_json::from_value(bridge_msg.data).unwrap();
        assert!((twist.linear.x - 1.5).abs() < 0.01);
        assert!((twist.angular.z - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_ros2_twist_to_motion_command() {
        let bridge_msg = ros2_msgs::BridgeMessage {
            msg_type: "geometry_msgs/msg/Twist".to_string(),
            topic: "/cmd_vel".to_string(),
            data: serde_json::json!({
                "linear": {"x": 2.0, "y": 0.0, "z": 0.0},
                "angular": {"x": 0.0, "y": 0.0, "z": 1.0}
            }),
        };

        let payload = from_ros2(&bridge_msg).expect("Should convert from ROS 2");
        match payload.payload {
            Some(PayloadKind::MotionCommand(cmd)) => {
                let lin = cmd.linear.unwrap();
                let ang = cmd.angular.unwrap();
                assert!((lin.x - 2.0).abs() < 0.01);
                assert!((ang.z - 1.0).abs() < 0.01);
            }
            _ => panic!("Expected MotionCommand"),
        }
    }

    #[test]
    fn test_heartbeat_to_ros2_diagnostic() {
        let payload = heartbeat(&[0xAA; 32], 5000, 75, 4096, 42);
        let bridge_msg = to_ros2(&payload).expect("Should convert to ROS 2");

        assert_eq!(bridge_msg.msg_type, "diagnostic_msgs/msg/DiagnosticStatus");
        assert_eq!(bridge_msg.topic, "/diagnostics");

        let status: ros2_msgs::DiagnosticStatus = serde_json::from_value(bridge_msg.data).unwrap();
        assert_eq!(status.level, 0);
        assert!(status.values.iter().any(|kv| kv.key == "cpu_usage" && kv.value == "75"));
        assert!(status.values.iter().any(|kv| kv.key == "mem_usage_kb" && kv.value == "4096"));
    }

    #[test]
    fn test_ros2_diagnostic_to_heartbeat() {
        let bridge_msg = ros2_msgs::BridgeMessage {
            msg_type: "diagnostic_msgs/msg/DiagnosticStatus".to_string(),
            topic: "/diagnostics".to_string(),
            data: serde_json::json!({
                "level": 0,
                "name": "robot_1",
                "message": "OK",
                "hardware_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "values": [
                    {"key": "uptime_ms", "value": "12000"},
                    {"key": "cpu_usage", "value": "60"},
                    {"key": "mem_usage_kb", "value": "2048"},
                    {"key": "epoch", "value": "99"}
                ]
            }),
        };

        let payload = from_ros2(&bridge_msg).expect("Should convert from ROS 2");
        match payload.payload {
            Some(PayloadKind::Heartbeat(hb)) => {
                assert_eq!(hb.uptime_ms, 12000);
                assert_eq!(hb.cpu_usage, 60);
                assert_eq!(hb.mem_usage_kb, 2048);
                assert_eq!(hb.epoch, 99);
            }
            _ => panic!("Expected Heartbeat"),
        }
    }

    #[test]
    fn test_sensor_fusion_to_ros2_detections() {
        let detections = vec![
            Detection {
                class: "person".into(),
                confidence: 0.95,
                bbox: Some(BoundingBox { x_min: 10.0, y_min: 20.0, x_max: 100.0, y_max: 200.0 }),
                timestamp_us: 1_700_000_000,
            },
        ];
        let payload = sensor_fusion("camera_front", 1_700_000_000, detections, None);
        let bridge_msg = to_ros2(&payload).expect("Should convert to ROS 2");

        assert_eq!(bridge_msg.msg_type, "vision_msgs/msg/Detection2DArray");
        assert_eq!(bridge_msg.topic, "/detections");
    }

    #[test]
    fn test_unsupported_payload_returns_none() {
        let payload = llm_query("hello", "system", "llama3");
        assert!(to_ros2(&payload).is_none(), "LLM queries have no ROS 2 equivalent");
    }

    #[test]
    fn test_unknown_ros2_type_returns_none() {
        let bridge_msg = ros2_msgs::BridgeMessage {
            msg_type: "nav_msgs/msg/Path".to_string(),
            topic: "/plan".to_string(),
            data: serde_json::json!({}),
        };
        assert!(from_ros2(&bridge_msg).is_none());
    }

    #[test]
    fn test_roundtrip_motion_command() {
        let original = motion_command(3.0, 1.0, 0.0, 0.0, 0.0, -0.5, 50_000);
        let ros2_msg = to_ros2(&original).unwrap();
        let restored = from_ros2(&ros2_msg).unwrap();

        match (original.payload, restored.payload) {
            (Some(PayloadKind::MotionCommand(orig)), Some(PayloadKind::MotionCommand(rest))) => {
                let ol = orig.linear.unwrap();
                let rl = rest.linear.unwrap();
                assert!((ol.x - rl.x).abs() < 0.01);
                assert!((ol.y - rl.y).abs() < 0.01);
                let oa = orig.angular.unwrap();
                let ra = rest.angular.unwrap();
                assert!((oa.z - ra.z).abs() < 0.01);
            }
            _ => panic!("Roundtrip failed"),
        }
    }
}
