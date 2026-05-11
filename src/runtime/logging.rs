//! Structured logging for OMNI-MESH
//!
//! Provides JSON-formatted logs with contextual fields for production debugging.
//!
//! Log levels:
//! - ERROR: Critical failures requiring immediate attention
//! - WARN: Recoverable errors or degraded operation
//! - INFO: Normal operational messages
//! - DEBUG: Detailed diagnostic information
//! - TRACE: Very verbose debugging (disabled in production)
//!
//! Contextual fields:
//! - message_id: Unique message identifier
//! - sender_did: Sender's decentralized identifier
//! - recipient_did: Recipient's decentralized identifier
//! - transport: Transport layer (tcp, quic)
//! - operation: Current operation (send, receive, verify, store)
//! - latency_us: Operation latency in microseconds
//! - error: Error message if applicable

use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: LogLevel,
    pub message: String,
    pub message_id: Option<String>,
    pub sender_did: Option<String>,
    pub recipient_did: Option<String>,
    pub transport: Option<String>,
    pub operation: Option<String>,
    pub latency_us: Option<u64>,
    pub error: Option<String>,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        
        LogEntry {
            timestamp,
            level,
            message: message.into(),
            message_id: None,
            sender_did: None,
            recipient_did: None,
            transport: None,
            operation: None,
            latency_us: None,
            error: None,
        }
    }
    
    pub fn with_message_id(mut self, id: impl Into<String>) -> Self {
        self.message_id = Some(id.into());
        self
    }
    
    pub fn with_sender(mut self, did: impl Into<String>) -> Self {
        self.sender_did = Some(did.into());
        self
    }
    
    pub fn with_recipient(mut self, did: impl Into<String>) -> Self {
        self.recipient_did = Some(did.into());
        self
    }
    
    pub fn with_transport(mut self, transport: impl Into<String>) -> Self {
        self.transport = Some(transport.into());
        self
    }
    
    pub fn with_operation(mut self, op: impl Into<String>) -> Self {
        self.operation = Some(op.into());
        self
    }
    
    pub fn with_latency(mut self, latency_us: u64) -> Self {
        self.latency_us = Some(latency_us);
        self
    }
    
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
    
    /// Format as JSON for structured logging
    pub fn to_json(&self) -> String {
        let mut fields = vec![
            format!("\"timestamp\":{}", self.timestamp),
            format!("\"level\":\"{}\"", self.level.as_str()),
            format!("\"message\":\"{}\"", self.message.replace('"', "\\\"")),
        ];
        
        if let Some(ref id) = self.message_id {
            fields.push(format!("\"message_id\":\"{}\"", id));
        }
        if let Some(ref sender) = self.sender_did {
            fields.push(format!("\"sender_did\":\"{}\"", sender));
        }
        if let Some(ref recipient) = self.recipient_did {
            fields.push(format!("\"recipient_did\":\"{}\"", recipient));
        }
        if let Some(ref transport) = self.transport {
            fields.push(format!("\"transport\":\"{}\"", transport));
        }
        if let Some(ref op) = self.operation {
            fields.push(format!("\"operation\":\"{}\"", op));
        }
        if let Some(latency) = self.latency_us {
            fields.push(format!("\"latency_us\":{}", latency));
        }
        if let Some(ref error) = self.error {
            fields.push(format!("\"error\":\"{}\"", error.replace('"', "\\\"")));
        }
        
        format!("{{{}}}", fields.join(","))
    }
    
    /// Format as human-readable text
    pub fn to_text(&self) -> String {
        let mut parts = vec![
            format!("[{}]", self.level.as_str()),
            self.message.clone(),
        ];
        
        if let Some(ref id) = self.message_id {
            parts.push(format!("msg_id={}", id));
        }
        if let Some(ref sender) = self.sender_did {
            parts.push(format!("from={}", sender));
        }
        if let Some(ref recipient) = self.recipient_did {
            parts.push(format!("to={}", recipient));
        }
        if let Some(ref transport) = self.transport {
            parts.push(format!("transport={}", transport));
        }
        if let Some(ref op) = self.operation {
            parts.push(format!("op={}", op));
        }
        if let Some(latency) = self.latency_us {
            parts.push(format!("latency={}us", latency));
        }
        if let Some(ref error) = self.error {
            parts.push(format!("error={}", error));
        }
        
        parts.join(" ")
    }
}

/// Global logger configuration
pub struct Logger {
    min_level: LogLevel,
    json_format: bool,
}

impl Logger {
    pub fn new(min_level: LogLevel, json_format: bool) -> Self {
        Logger { min_level, json_format }
    }
    
    pub fn log(&self, entry: LogEntry) {
        // Check if we should log this level
        let should_log = match (self.min_level, entry.level) {
            (LogLevel::Error, LogLevel::Error) => true,
            (LogLevel::Warn, LogLevel::Error | LogLevel::Warn) => true,
            (LogLevel::Info, LogLevel::Error | LogLevel::Warn | LogLevel::Info) => true,
            (LogLevel::Debug, LogLevel::Error | LogLevel::Warn | LogLevel::Info | LogLevel::Debug) => true,
            (LogLevel::Trace, _) => true,
            _ => false,
        };
        
        if should_log {
            let output = if self.json_format {
                entry.to_json()
            } else {
                entry.to_text()
            };
            
            println!("{}", output);
        }
    }
}

// Convenience macros for logging
#[macro_export]
macro_rules! log_error {
    ($logger:expr, $msg:expr) => {
        $logger.log(LogEntry::new(LogLevel::Error, $msg))
    };
    ($logger:expr, $msg:expr, $($field:tt)*) => {
        $logger.log(LogEntry::new(LogLevel::Error, $msg)$($field)*)
    };
}

#[macro_export]
macro_rules! log_warn {
    ($logger:expr, $msg:expr) => {
        $logger.log(LogEntry::new(LogLevel::Warn, $msg))
    };
    ($logger:expr, $msg:expr, $($field:tt)*) => {
        $logger.log(LogEntry::new(LogLevel::Warn, $msg)$($field)*)
    };
}

#[macro_export]
macro_rules! log_info {
    ($logger:expr, $msg:expr) => {
        $logger.log(LogEntry::new(LogLevel::Info, $msg))
    };
    ($logger:expr, $msg:expr, $($field:tt)*) => {
        $logger.log(LogEntry::new(LogLevel::Info, $msg)$($field)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_log_entry_json() {
        let entry = LogEntry::new(LogLevel::Info, "Test message")
            .with_message_id("abc123")
            .with_sender("did:omni:sender")
            .with_recipient("did:omni:recipient")
            .with_transport("tcp")
            .with_operation("send")
            .with_latency(150);
        
        let json = entry.to_json();
        
        assert!(json.contains("\"level\":\"INFO\""));
        assert!(json.contains("\"message\":\"Test message\""));
        assert!(json.contains("\"message_id\":\"abc123\""));
        assert!(json.contains("\"sender_did\":\"did:omni:sender\""));
        assert!(json.contains("\"transport\":\"tcp\""));
        assert!(json.contains("\"latency_us\":150"));
        
        println!("JSON: {}", json);
    }
    
    #[test]
    fn test_log_entry_text() {
        let entry = LogEntry::new(LogLevel::Warn, "Connection timeout")
            .with_transport("quic")
            .with_error("Connection refused");
        
        let text = entry.to_text();
        
        assert!(text.contains("[WARN]"));
        assert!(text.contains("Connection timeout"));
        assert!(text.contains("transport=quic"));
        assert!(text.contains("error=Connection refused"));
        
        println!("Text: {}", text);
    }
    
    #[test]
    fn test_logger_filtering() {
        let logger = Logger::new(LogLevel::Warn, false);
        
        // Should log
        logger.log(LogEntry::new(LogLevel::Error, "Error message"));
        logger.log(LogEntry::new(LogLevel::Warn, "Warning message"));
        
        // Should not log (below min level)
        logger.log(LogEntry::new(LogLevel::Info, "Info message"));
        logger.log(LogEntry::new(LogLevel::Debug, "Debug message"));
    }
}
