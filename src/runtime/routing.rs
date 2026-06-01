use crate::envelope::Did;
use crate::buffer::FixedMap;
use std::net::{SocketAddr, IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

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
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_micros() as u64;
            let info = NeighborInfo {
                addr,
                last_seen_us: now,
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
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_micros() as u64;
            for (did, info) in r.iter() {
                // Ignore extremely stale routes (> 30 seconds)
                if now.saturating_sub(info.last_seen_us) < 30_000_000 {
                    routes.push((*did, info.addr));
                }
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
                    if let Ok((len, _src_addr)) = listen_socket.recv_from(&mut buf).await {
                        // Binary format:
                        // [DID: 32 bytes]
                        // [IP Type: 1 byte (4 or 6)]
                        // [IP Bytes: 4 or 16 bytes]
                        // [Port: 2 bytes]
                        let mut offset = 0;
                        while offset + 33 < len {
                            let mut did_bytes = [0u8; 32];
                            did_bytes.copy_from_slice(&buf[offset..offset + 32]);
                            offset += 32;
                            
                            let ip_type = buf[offset];
                            offset += 1;
                            
                            let addr = if ip_type == 4 && offset + 6 <= len {
                                let mut ip_bytes = [0u8; 4];
                                ip_bytes.copy_from_slice(&buf[offset..offset + 4]);
                                offset += 4;
                                let port = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
                                offset += 2;
                                Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::from(ip_bytes)), port))
                            } else if ip_type == 6 && offset + 18 <= len {
                                let mut ip_bytes = [0u8; 16];
                                ip_bytes.copy_from_slice(&buf[offset..offset + 16]);
                                offset += 16;
                                let port = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
                                offset += 2;
                                Some(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(ip_bytes)), port))
                            } else {
                                None
                            };

                            if let Some(parsed_addr) = addr {
                                // Do not route to self (optional check here if self DID is known)
                                table_clone.update_route(Did(did_bytes), parsed_addr);
                            } else {
                                break; // Invalid format
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
                
                // Serialize routes into multiple MTU-safe packets.
                // Each route entry is: [DID: 32] + [IP type: 1] + [IP: 4 or 16] + [Port: 2]
                // IPv4 entry = 39 bytes, IPv6 entry = 51 bytes.
                // We target max 1200 bytes per packet (safe for all networks including tunnels).
                const MAX_PACKET_SIZE: usize = 1200;
                
                let mut packet = Vec::with_capacity(MAX_PACKET_SIZE);
                
                for (did, addr) in &routes {
                    let entry_size = match addr.ip() {
                        IpAddr::V4(_) => 32 + 1 + 4 + 2, // 39 bytes
                        IpAddr::V6(_) => 32 + 1 + 16 + 2, // 51 bytes
                    };
                    
                    // If adding this entry would exceed the packet limit, send current packet first
                    if !packet.is_empty() && packet.len() + entry_size > MAX_PACKET_SIZE {
                        let _ = socket.send_to(&packet, broadcast_addr).await;
                        packet.clear();
                    }
                    
                    // Serialize the entry
                    packet.extend_from_slice(&did.0);
                    match addr.ip() {
                        IpAddr::V4(ip4) => {
                            packet.push(4);
                            packet.extend_from_slice(&ip4.octets());
                        }
                        IpAddr::V6(ip6) => {
                            packet.push(6);
                            packet.extend_from_slice(&ip6.octets());
                        }
                    }
                    packet.extend_from_slice(&addr.port().to_be_bytes());
                }
                
                // Send the final (possibly partial) packet
                if !packet.is_empty() {
                    let _ = socket.send_to(&packet, broadcast_addr).await;
                }
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
