use crate::envelope::Did;
use crate::buffer::FixedMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Neighbor information tracking for routing
#[derive(Debug, Clone, Copy)]
pub struct NeighborInfo {
    pub addr: SocketAddr,
    pub last_seen_us: u64,
}

impl Default for NeighborInfo {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:0".parse().unwrap(),
            last_seen_us: 0,
        }
    }
}

/// A thread-safe decentralized routing table for mapping DIDs to socket addresses.
///
/// Strictly uses zero dynamic allocation in the hot path.
#[derive(Debug, Clone)]
pub struct RoutingTable {
    routes: Arc<Mutex<FixedMap<Did, NeighborInfo, 1024>>>,
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingTable {
    pub fn new() -> Self {
        RoutingTable {
            routes: Arc::new(Mutex::new(FixedMap::new())),
        }
    }

    /// Update or add a route
    pub fn update_route(&self, did: Did, addr: SocketAddr) {
        if let Ok(mut routes) = self.routes.lock() {
            let info = NeighborInfo {
                addr,
                last_seen_us: 0, // In a real system, get timestamp
            };
            let _ = routes.insert(did, info);
        }
    }

    /// Resolve a DID to a socket address
    pub fn resolve(&self, did: &Did) -> Option<SocketAddr> {
        self.routes.lock().ok().and_then(|r| r.get(did).map(|info| info.addr))
    }

    /// Gossip a list of known routes
    pub fn gossip_routes(&self) -> Vec<(Did, SocketAddr)> {
        let mut routes = Vec::new();
        if let Ok(r) = self.routes.lock() {
            for (did, info) in r.iter() {
                routes.push((*did, info.addr));
            }
        }
        routes
    }

    /// Spawns a background Tokio task to broadcast known routes over UDP multicast.
    pub fn start_gossip_task(self: Arc<Self>, interval_ms: u64, bind_addr: SocketAddr, broadcast_addr: SocketAddr) {
        let task = async move {
            let socket = match tokio::net::UdpSocket::bind(bind_addr).await {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    eprintln!("Failed to bind UDP gossip socket: {}", e);
                    return;
                }
            };
            
            socket.set_broadcast(true).unwrap_or_default();
            
            // Spawn listener task
            let listen_socket = socket.clone();
            let table_clone = self.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    if let Ok((len, _)) = listen_socket.recv_from(&mut buf).await {
                        // Very simple parse logic (just for mock)
                        let mut offset = 0;
                        while offset + 33 <= len {
                            let mut did_bytes = [0u8; 32];
                            did_bytes.copy_from_slice(&buf[offset..offset + 32]);
                            offset += 32;
                            let str_len = buf[offset] as usize;
                            offset += 1;
                            
                            if offset + str_len <= len {
                                if let Ok(addr_str) = std::str::from_utf8(&buf[offset..offset + str_len]) {
                                    if let Ok(addr) = addr_str.parse::<std::net::SocketAddr>() {
                                        table_clone.update_route(Did(did_bytes), addr);
                                    }
                                }
                                offset += str_len;
                            } else {
                                break;
                            }
                        }
                    }
                }
            });

            // Broadcast loop
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(interval_ms));
            loop {
                interval.tick().await;
                let routes = self.gossip_routes();
                if routes.is_empty() { continue; }
                
                let mut buf = Vec::new();
                for (did, addr) in routes {
                    buf.extend_from_slice(&did.0);
                    let addr_str = addr.to_string();
                    buf.push(addr_str.len() as u8);
                    buf.extend_from_slice(addr_str.as_bytes());
                }
                
                let _ = socket.send_to(&buf, broadcast_addr).await;
            }
        };
            
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(task);
        } else {
            // We are in a sync context (e.g. cargo test), spawn a dedicated thread
            std::thread::spawn(move || {
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    rt.block_on(task);
                }
            });
        }
    }
}
