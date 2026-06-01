//! Payload types for OMNI-MESH envelope contents.
//!
//! Strongly-typed payload parsing using prost derive macros.
//! No `protoc` compiler required — types are defined directly in Rust.

use alloc::string::String;
use alloc::vec::Vec;
use prost::Message;

/// Agent-to-agent command messages
#[derive(Clone, PartialEq, Message)]
pub struct AgentCommand {
    #[prost(string, tag = "1")]
    pub command_type: String,
    #[prost(bytes = "vec", tag = "2")]
    pub target_did: Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub timestamp_ns: u64,
    #[prost(bytes = "vec", tag = "4")]
    pub payload: Vec<u8>,
}

/// ROS 2 geometry_msgs/Vector3 equivalent
#[derive(Clone, PartialEq, Message)]
pub struct Vector3 {
    #[prost(float, tag = "1")]
    pub x: f32,
    #[prost(float, tag = "2")]
    pub y: f32,
    #[prost(float, tag = "3")]
    pub z: f32,
}

/// Robot motion commands (geometry_msgs/Twist equivalent)
#[derive(Clone, PartialEq, Message)]
pub struct MotionCommand {
    #[prost(message, optional, tag = "1")]
    pub linear: Option<Vector3>,
    #[prost(message, optional, tag = "2")]
    pub angular: Option<Vector3>,
    #[prost(uint64, tag = "3")]
    pub deadline_ns: u64,
}

/// Inference result from an ML pipeline
#[derive(Clone, PartialEq, Message)]
pub struct InferenceResult {
    #[prost(string, tag = "1")]
    pub model_id: String,
    #[prost(float, tag = "2")]
    pub confidence: f32,
    #[prost(string, tag = "3")]
    pub label: String,
    #[prost(bytes = "vec", tag = "4")]
    pub raw_output: Vec<u8>,
    #[prost(uint64, tag = "5")]
    pub latency_us: u64,
}

/// Request for an LLM inference (e.g. edge AI node asking Ollama)
#[derive(Clone, PartialEq, Message)]
pub struct LlmQuery {
    #[prost(string, tag = "1")]
    pub prompt: String,
    #[prost(string, tag = "2")]
    pub system_prompt: String,
    #[prost(string, tag = "3")]
    pub model: String,
}

/// Response from an LLM inference
#[derive(Clone, PartialEq, Message)]
pub struct LlmResponse {
    #[prost(string, tag = "1")]
    pub response: String,
    #[prost(uint64, tag = "2")]
    pub latency_us: u64,
}

/// Telemetry heartbeat
#[derive(Clone, PartialEq, Message)]
pub struct Heartbeat {
    #[prost(bytes = "vec", tag = "1")]
    pub sender_did: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub uptime_ms: u64,
    #[prost(uint32, tag = "3")]
    pub cpu_usage: u32,
    #[prost(uint32, tag = "4")]
    pub mem_usage_kb: u32,
    #[prost(uint64, tag = "5")]
    pub epoch: u64,
}

/// Envelope payload wrapper with oneof semantics
#[derive(Clone, PartialEq, Message)]
pub struct EnvelopePayload {
    #[prost(oneof = "PayloadKind", tags = "1, 2, 3, 4, 5, 6, 7, 8")]
    pub payload: Option<PayloadKind>,
}

/// Model weights for federated learning / model distribution
#[derive(Clone, PartialEq, Message)]
pub struct ModelWeights {
    #[prost(string, tag = "1")]
    pub model_id: String,
    #[prost(bytes = "vec", tag = "2")]
    pub compressed_weights: Vec<u8>,
    #[prost(string, tag = "3")]
    pub compression: String,
    #[prost(uint64, tag = "4")]
    pub original_size: u64,
}

/// Bounding box for object detection (vision_msgs/BoundingBox2D equivalent)
#[derive(Clone, PartialEq, Message)]
pub struct BoundingBox {
    #[prost(float, tag = "1")]
    pub x_min: f32,
    #[prost(float, tag = "2")]
    pub y_min: f32,
    #[prost(float, tag = "3")]
    pub x_max: f32,
    #[prost(float, tag = "4")]
    pub y_max: f32,
}

/// Single detection from an inference pipeline (vision_msgs/Detection2D equivalent)
#[derive(Clone, PartialEq, Message)]
pub struct Detection {
    #[prost(string, tag = "1")]
    pub class: String,
    #[prost(float, tag = "2")]
    pub confidence: f32,
    #[prost(message, optional, tag = "3")]
    pub bbox: Option<BoundingBox>,
    #[prost(uint64, tag = "4")]
    pub timestamp_us: u64,
}

/// 3D transform (geometry_msgs/Transform equivalent)
#[derive(Clone, PartialEq, Message)]
pub struct Transform {
    #[prost(float, tag = "1")]
    pub x: f32,
    #[prost(float, tag = "2")]
    pub y: f32,
    #[prost(float, tag = "3")]
    pub z: f32,
    #[prost(float, tag = "4")]
    pub qx: f32,
    #[prost(float, tag = "5")]
    pub qy: f32,
    #[prost(float, tag = "6")]
    pub qz: f32,
    #[prost(float, tag = "7")]
    pub qw: f32,
}

/// Sensor fusion frame combining multiple sensor modalities
/// Maps well to a combined PointCloud2 and Image structure
#[derive(Clone, PartialEq, Message)]
pub struct SensorFusionFrame {
    #[prost(string, tag = "1")]
    pub frame_id: String,
    #[prost(uint64, tag = "2")]
    pub timestamp_us: u64,
    #[prost(message, repeated, tag = "3")]
    pub detections: Vec<Detection>,
    #[prost(bytes = "vec", tag = "4")]
    pub point_cloud_data: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub image_data: Vec<u8>,
    #[prost(message, optional, tag = "6")]
    pub pose: Option<Transform>,
}

/// The oneof variants for EnvelopePayload
#[derive(Clone, PartialEq, prost::Oneof)]
pub enum PayloadKind {
    #[prost(message, tag = "1")]
    AgentCommand(AgentCommand),
    #[prost(message, tag = "2")]
    MotionCommand(MotionCommand),
    #[prost(message, tag = "3")]
    InferenceResult(InferenceResult),
    #[prost(message, tag = "4")]
    Heartbeat(Heartbeat),
    #[prost(message, tag = "5")]
    ModelWeights(ModelWeights),
    #[prost(message, tag = "6")]
    SensorFusion(SensorFusionFrame),
    #[prost(message, tag = "7")]
    LlmQuery(LlmQuery),
    #[prost(message, tag = "8")]
    LlmResponse(LlmResponse),
}

/// Error types for payload operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadParseError {
    DecodeFailed,
    EmptyPayload,
}

/// Parse raw bytes into an `EnvelopePayload`
pub fn decode_payload(bytes: &[u8]) -> Result<EnvelopePayload, PayloadParseError> {
    if bytes.is_empty() {
        return Err(PayloadParseError::EmptyPayload);
    }
    EnvelopePayload::decode(bytes).map_err(|_| PayloadParseError::DecodeFailed)
}

/// Encode an `EnvelopePayload` into bytes
pub fn encode_payload(payload: &EnvelopePayload) -> Vec<u8> {
    payload.encode_to_vec()
}

/// Helper: create an AgentCommand payload
pub fn agent_command(command_type: &str, target_did: &[u8], data: &[u8]) -> EnvelopePayload {
    EnvelopePayload {
        payload: Some(PayloadKind::AgentCommand(AgentCommand {
            command_type: command_type.into(),
            target_did: target_did.into(),
            timestamp_ns: 0,
            payload: data.into(),
        })),
    }
}

/// Helper: create a MotionCommand payload
pub fn motion_command(lx: f32, ly: f32, lz: f32, ax: f32, ay: f32, az: f32, deadline_ns: u64) -> EnvelopePayload {
    EnvelopePayload {
        payload: Some(PayloadKind::MotionCommand(MotionCommand {
            linear: Some(Vector3 { x: lx, y: ly, z: lz }),
            angular: Some(Vector3 { x: ax, y: ay, z: az }),
            deadline_ns,
        })),
    }
}

/// Helper: create a Heartbeat payload
pub fn heartbeat(sender_did: &[u8], uptime_ms: u64, cpu: u32, mem_kb: u32, epoch: u64) -> EnvelopePayload {
    EnvelopePayload {
        payload: Some(PayloadKind::Heartbeat(Heartbeat {
            sender_did: sender_did.into(),
            uptime_ms,
            cpu_usage: cpu,
            mem_usage_kb: mem_kb,
            epoch,
        })),
    }
}

/// Helper: create a ModelWeights payload
pub fn model_weights(model_id: &str, weights: &[u8], compression: &str, original_size: u64) -> EnvelopePayload {
    EnvelopePayload {
        payload: Some(PayloadKind::ModelWeights(ModelWeights {
            model_id: model_id.into(),
            compressed_weights: weights.into(),
            compression: compression.into(),
            original_size,
        })),
    }
}

/// Helper: create a SensorFusionFrame payload
pub fn sensor_fusion(frame_id: &str, timestamp_us: u64, detections: Vec<Detection>, pose: Option<Transform>) -> EnvelopePayload {
    EnvelopePayload {
        payload: Some(PayloadKind::SensorFusion(SensorFusionFrame {
            frame_id: frame_id.into(),
            timestamp_us,
            detections,
            point_cloud_data: Vec::new(),
            image_data: Vec::new(),
            pose,
        })),
    }
}

/// Helper: create an LlmQuery payload
pub fn llm_query(prompt: &str, system_prompt: &str, model: &str) -> EnvelopePayload {
    EnvelopePayload {
        payload: Some(PayloadKind::LlmQuery(LlmQuery {
            prompt: prompt.into(),
            system_prompt: system_prompt.into(),
            model: model.into(),
        })),
    }
}

/// Helper: create an LlmResponse payload
pub fn llm_response(response: &str, latency_us: u64) -> EnvelopePayload {
    EnvelopePayload {
        payload: Some(PayloadKind::LlmResponse(LlmResponse {
            response: response.into(),
            latency_us,
        })),
    }
}
