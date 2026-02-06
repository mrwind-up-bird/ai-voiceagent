//! mDNS service discovery for local network device pairing.
//!
//! When a session is created, the device announces itself as an
//! `_aurus-sync._tcp.local.` service. Joiners browse for this service
//! to discover the creator's IP address and port.

use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::net::IpAddr;

/// mDNS service type for Aurus sync.
const SERVICE_TYPE: &str = "_aurus-sync._tcp.local.";

/// Information about a discovered sync service on the local network.
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub address: IpAddr,
    pub port: u16,
    pub device_name: String,
    pub session_fingerprint: String,
}

/// Manages mDNS service announcement and discovery.
pub struct SyncDiscovery {
    daemon: ServiceDaemon,
    /// The full service name if we're announcing (for unregistration).
    registered_service: Option<String>,
}

impl SyncDiscovery {
    /// Create a new mDNS discovery instance.
    pub fn new() -> Result<Self, String> {
        let daemon =
            ServiceDaemon::new().map_err(|e| format!("Failed to create mDNS daemon: {}", e))?;
        Ok(Self {
            daemon,
            registered_service: None,
        })
    }

    /// Announce this device as a sync session creator on the local network.
    ///
    /// - `port`: The WebSocket port the creator is listening on.
    /// - `device_name`: Human-readable device name (e.g. "Oliver's MacBook").
    /// - `session_fingerprint`: Short hash of session_id for verification.
    pub fn announce(
        &mut self,
        port: u16,
        device_name: &str,
        session_fingerprint: &str,
    ) -> Result<(), String> {
        let hostname = format!("aurus-{}.local.", &uuid::Uuid::new_v4().to_string()[..8]);

        let properties: HashMap<String, String> = [
            ("device".to_string(), device_name.to_string()),
            ("fingerprint".to_string(), session_fingerprint.to_string()),
            ("version".to_string(), "1".to_string()),
        ]
        .into_iter()
        .collect();

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            device_name,
            &hostname,
            "",       // empty = auto-detect local IP
            port,
            properties,
        )
        .map_err(|e| format!("Failed to create service info: {}", e))?;

        let fullname = service_info.get_fullname().to_string();

        self.daemon
            .register(service_info)
            .map_err(|e| format!("Failed to register mDNS service: {}", e))?;

        self.registered_service = Some(fullname.clone());
        tracing::info!(
            "mDNS: Announced sync service on port {} as {}",
            port,
            fullname
        );

        Ok(())
    }

    /// Stop announcing the service.
    pub fn unannounce(&mut self) -> Result<(), String> {
        if let Some(fullname) = self.registered_service.take() {
            self.daemon
                .unregister(&fullname)
                .map_err(|e| format!("Failed to unregister mDNS service: {}", e))?;
            tracing::info!("mDNS: Unannounced sync service: {}", fullname);
        }
        Ok(())
    }

    /// Browse for sync services on the local network.
    /// Returns a receiver that yields `ServiceEvent` instances.
    pub fn browse(
        &self,
    ) -> Result<mdns_sd::Receiver<ServiceEvent>, String> {
        self.daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| format!("Failed to browse mDNS: {}", e))
    }

    /// Shutdown the mDNS daemon cleanly.
    pub fn shutdown(mut self) -> Result<(), String> {
        self.unannounce()?;
        self.daemon
            .shutdown()
            .map_err(|e| format!("Failed to shutdown mDNS daemon: {}", e))?;
        Ok(())
    }
}

/// Extract a `DiscoveredPeer` from a resolved mDNS service.
pub fn peer_from_resolved_service(info: &ResolvedService) -> Option<DiscoveredPeer> {
    // Get the first IPv4 address (prefer IPv4 for simplicity)
    let address = info
        .addresses
        .iter()
        .find(|a| a.is_ipv4())
        .or_else(|| info.addresses.iter().next())
        .map(|a| a.to_ip_addr())?;

    let port = info.port;

    let device_name = info
        .txt_properties
        .get("device")
        .map(|v| v.val_str().to_string())
        .unwrap_or_else(|| "Unknown Device".to_string());

    let session_fingerprint = info
        .txt_properties
        .get("fingerprint")
        .map(|v| v.val_str().to_string())
        .unwrap_or_default();

    Some(DiscoveredPeer {
        address,
        port,
        device_name,
        session_fingerprint,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_creation() {
        let discovery = SyncDiscovery::new();
        assert!(discovery.is_ok(), "Should be able to create mDNS daemon");
        // Clean up
        discovery.unwrap().shutdown().ok();
    }
}
