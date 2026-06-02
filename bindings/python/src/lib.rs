//! Python bindings for OMNI-MESH
//!
//! Provides a native Python module for sending and receiving
//! cryptographically signed messages over the OMNI-MESH network.
//!
//! Usage:
//!   import omnimesh
//!   client = omnimesh.Client(mode="development")
//!   client.send_agent_command(target_did, "pick", b"robot-1", b"shelf-A12")
//!   msg = client.receive(timeout_ms=5000)

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::exceptions::PyValueError;
use std::time::Duration;

use ::omnimesh::client::{OmnimeshClient, ClientConfig};
use ::omnimesh::envelope::Did;
use ::omnimesh::payload::{self, PayloadKind};

/// Parse a 64-character hex string into a Did
fn parse_did(did_hex: &str) -> PyResult<Did> {
    let bytes = hex::decode(did_hex)
        .map_err(|e| PyValueError::new_err(format!("Invalid DID hex: {}", e)))?;
    if bytes.len() != 32 {
        return Err(PyValueError::new_err("DID must be exactly 32 bytes (64 hex chars)"));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(Did(arr))
}

/// Python-facing OMNI-MESH client.
///
/// Each Client instance is a mesh node with its own cryptographic identity (DID).
/// Messages are automatically signed and verified.
#[pyclass]
pub struct Client {
    inner: OmnimeshClient,
}

#[pymethods]
impl Client {
    /// Create a new Client.
    ///
    /// Args:
    ///     mode: "development" (default), "lightweight", or "production"
    ///     listen_port: Custom TCP port to listen on (required for lightweight/production cross-process)
    ///
    /// Returns:
    ///     A new Client instance with a unique cryptographic identity.
    #[new]
    #[pyo3(signature = (mode="development", listen_port=None))]
    fn new(mode: &str, listen_port: Option<u16>) -> PyResult<Self> {
        let config = match (mode, listen_port) {
            ("development", _) => ClientConfig::development(),
            ("lightweight", Some(port)) => ClientConfig::lightweight_on_port(port),
            ("lightweight", None) => ClientConfig::lightweight(),
            ("production", Some(port)) => ClientConfig::production_on_port(port),
            ("production", None) => ClientConfig::production(),
            _ => return Err(PyValueError::new_err(
                "mode must be 'development', 'lightweight', or 'production'"
            )),
        };
        let client = OmnimeshClient::builder()
            .with_config(config)
            .build()
            .map_err(|e| PyValueError::new_err(format!("Failed to build client: {}", e)))?;
        Ok(Client { inner: client })
    }

    /// The node's DID (Decentralized Identifier) as a 64-character hex string.
    #[getter]
    fn did(&self) -> String {
        hex::encode(self.inner.did.0)
    }

    /// Register a peer's DID and network address for routing.
    ///
    /// This tells the client where to send messages for a given DID.
    /// Required for cross-process and cross-machine communication.
    ///
    /// Args:
    ///     did_hex: 64-char hex string of the peer's DID
    ///     addr: IP:port string (e.g. "127.0.0.1:9001")
    fn register_peer(&self, did_hex: &str, addr: &str) -> PyResult<()> {
        let did = parse_did(did_hex)?;
        self.inner.register_peer(did, addr)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Send an AgentCommand payload to another node.
    ///
    /// Args:
    ///     target_did_hex: 64-char hex string of the recipient's DID
    ///     command_type: Command name (e.g. "pick", "move", "stop")
    ///     target_id: Target identifier bytes
    ///     payload: Arbitrary payload bytes
    fn send_agent_command(
        &self,
        target_did_hex: &str,
        command_type: &str,
        target_id: &[u8],
        payload_data: &[u8],
    ) -> PyResult<()> {
        let did = parse_did(target_did_hex)?;
        let msg = payload::agent_command(command_type, target_id, payload_data);
        self.inner.send(did, msg)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Send a MotionCommand (geometry_msgs/Twist equivalent) to another node.
    ///
    /// Args:
    ///     target_did_hex: 64-char hex string of the recipient's DID
    ///     linear_x, linear_y, linear_z: Linear velocity components
    ///     angular_x, angular_y, angular_z: Angular velocity components
    ///     deadline_ns: Deadline in nanoseconds (0 = no deadline)
    #[pyo3(signature = (target_did_hex, linear_x=0.0, linear_y=0.0, linear_z=0.0, angular_x=0.0, angular_y=0.0, angular_z=0.0, deadline_ns=0))]
    fn send_motion_command(
        &self,
        target_did_hex: &str,
        linear_x: f32,
        linear_y: f32,
        linear_z: f32,
        angular_x: f32,
        angular_y: f32,
        angular_z: f32,
        deadline_ns: u64,
    ) -> PyResult<()> {
        let did = parse_did(target_did_hex)?;
        let msg = payload::motion_command(linear_x, linear_y, linear_z, angular_x, angular_y, angular_z, deadline_ns);
        self.inner.send(did, msg)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Send a Heartbeat telemetry payload.
    ///
    /// Args:
    ///     target_did_hex: 64-char hex string of the recipient's DID
    ///     uptime_ms: Node uptime in milliseconds
    ///     cpu: CPU usage percentage (0-100)
    ///     mem_kb: Memory usage in kilobytes
    ///     epoch: Monotonic epoch counter
    fn send_heartbeat(
        &self,
        target_did_hex: &str,
        uptime_ms: u64,
        cpu: u32,
        mem_kb: u32,
        epoch: u64,
    ) -> PyResult<()> {
        let did = parse_did(target_did_hex)?;
        let msg = payload::heartbeat(&self.inner.did.0, uptime_ms, cpu, mem_kb, epoch);
        self.inner.send(did, msg)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Send an LLM query to another node for inference.
    ///
    /// Args:
    ///     target_did_hex: 64-char hex string of the recipient's DID
    ///     prompt: The user prompt
    ///     system_prompt: System prompt for the LLM
    ///     model: Model name (e.g. "llama3", "mistral")
    fn send_llm_query(
        &self,
        target_did_hex: &str,
        prompt: &str,
        system_prompt: &str,
        model: &str,
    ) -> PyResult<()> {
        let did = parse_did(target_did_hex)?;
        let msg = payload::llm_query(prompt, system_prompt, model);
        self.inner.send(did, msg)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Block until a message arrives or timeout_ms elapses.
    ///
    /// The GIL is released while waiting, so other Python threads can run.
    ///
    /// Args:
    ///     timeout_ms: Maximum time to wait in milliseconds
    ///
    /// Returns:
    ///     A dict with message fields, or None if timeout expired.
    ///     Dict always contains: "type", "sender_did"
    ///     Additional fields depend on the message type.
    fn receive(&self, py: Python<'_>, timeout_ms: u64) -> PyResult<Option<PyObject>> {
        let timeout = Duration::from_millis(timeout_ms);
        let msg_opt = py.allow_threads(|| self.inner.receive_timeout(timeout));

        let Some(msg) = msg_opt else { return Ok(None) };

        let dict = PyDict::new_bound(py);
        dict.set_item("sender_did", hex::encode(msg.sender.0))?;
        dict.set_item("received_at_us", msg.received_at_us)?;

        match msg.payload.payload {
            Some(PayloadKind::AgentCommand(cmd)) => {
                dict.set_item("type", "agent_command")?;
                dict.set_item("command_type", cmd.command_type)?;
                dict.set_item("target_did", hex::encode(&cmd.target_did))?;
                dict.set_item("payload", cmd.payload.as_slice())?;
            }
            Some(PayloadKind::Heartbeat(hb)) => {
                dict.set_item("type", "heartbeat")?;
                dict.set_item("uptime_ms", hb.uptime_ms)?;
                dict.set_item("cpu_usage", hb.cpu_usage)?;
                dict.set_item("mem_usage_kb", hb.mem_usage_kb)?;
                dict.set_item("epoch", hb.epoch)?;
            }
            Some(PayloadKind::MotionCommand(mc)) => {
                dict.set_item("type", "motion_command")?;
                if let Some(lin) = mc.linear {
                    dict.set_item("linear_x", lin.x)?;
                    dict.set_item("linear_y", lin.y)?;
                    dict.set_item("linear_z", lin.z)?;
                }
                if let Some(ang) = mc.angular {
                    dict.set_item("angular_x", ang.x)?;
                    dict.set_item("angular_y", ang.y)?;
                    dict.set_item("angular_z", ang.z)?;
                }
                dict.set_item("deadline_ns", mc.deadline_ns)?;
            }
            Some(PayloadKind::LlmQuery(llm)) => {
                dict.set_item("type", "llm_query")?;
                dict.set_item("prompt", llm.prompt)?;
                dict.set_item("system_prompt", llm.system_prompt)?;
                dict.set_item("model", llm.model)?;
            }
            Some(PayloadKind::LlmResponse(resp)) => {
                dict.set_item("type", "llm_response")?;
                dict.set_item("response", resp.response)?;
                dict.set_item("latency_us", resp.latency_us)?;
            }
            Some(PayloadKind::InferenceResult(ir)) => {
                dict.set_item("type", "inference_result")?;
                dict.set_item("model_id", ir.model_id)?;
                dict.set_item("confidence", ir.confidence)?;
                dict.set_item("label", ir.label)?;
                dict.set_item("latency_us", ir.latency_us)?;
            }
            _ => {
                dict.set_item("type", "unknown")?;
            }
        }

        Ok(Some(dict.into()))
    }

    /// Try to receive a message without blocking.
    ///
    /// Returns:
    ///     A dict with message fields, or None if no message available.
    fn try_receive(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        // Use 0ms timeout for non-blocking
        self.receive(py, 0)
    }

    /// Get the number of messages waiting in the inbox.
    fn inbox_len(&self) -> usize {
        self.inner.inbox_len()
    }

    /// Gracefully shut down the client.
    ///
    /// After calling this, send() will raise an error and receive() will return None.
    /// Messages already in the inbox can still be drained with try_receive().
    fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Check if the client has been shut down.
    fn is_shutdown(&self) -> bool {
        self.inner.is_shutdown()
    }
}

/// The `omnimesh` Python module.
#[pymodule]
fn omnimesh(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Client>()?;
    Ok(())
}
