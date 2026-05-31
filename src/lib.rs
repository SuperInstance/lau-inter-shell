use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// TrustLevel
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    Full,
    Partial,
    Minimal,
    Untrusted,
}

impl TrustLevel {
    pub fn can_receive_state(&self) -> bool {
        matches!(self, TrustLevel::Full | TrustLevel::Partial)
    }

    pub fn can_send_commands(&self) -> bool {
        matches!(self, TrustLevel::Full)
    }
}

impl fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrustLevel::Full => write!(f, "full"),
            TrustLevel::Partial => write!(f, "partial"),
            TrustLevel::Minimal => write!(f, "minimal"),
            TrustLevel::Untrusted => write!(f, "untrusted"),
        }
    }
}

// ---------------------------------------------------------------------------
// MessagePriority
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum MessagePriority {
    Critical,
    High,
    #[default]
    Normal,
    Low,
}

impl fmt::Display for MessagePriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessagePriority::Critical => write!(f, "critical"),
            MessagePriority::High => write!(f, "high"),
            MessagePriority::Normal => write!(f, "normal"),
            MessagePriority::Low => write!(f, "low"),
        }
    }
}

// ---------------------------------------------------------------------------
// ShellMessageType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellMessageType {
    Briefing,
    StateUpdate,
    Command,
    Query,
    Response,
    Alert,
    Heartbeat,
    Handshake,
}

impl fmt::Display for ShellMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShellMessageType::Briefing => write!(f, "briefing"),
            ShellMessageType::StateUpdate => write!(f, "state_update"),
            ShellMessageType::Command => write!(f, "command"),
            ShellMessageType::Query => write!(f, "query"),
            ShellMessageType::Response => write!(f, "response"),
            ShellMessageType::Alert => write!(f, "alert"),
            ShellMessageType::Heartbeat => write!(f, "heartbeat"),
            ShellMessageType::Handshake => write!(f, "handshake"),
        }
    }
}

// ---------------------------------------------------------------------------
// ShellCommError
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellCommError {
    PeerNotFound,
    PeerUnreachable,
    ProtocolError(String),
    MessageExpired,
    TrustViolation,
    RateLimited,
}

impl fmt::Display for ShellCommError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShellCommError::PeerNotFound => write!(f, "peer not found"),
            ShellCommError::PeerUnreachable => write!(f, "peer unreachable"),
            ShellCommError::ProtocolError(msg) => write!(f, "protocol error: {msg}"),
            ShellCommError::MessageExpired => write!(f, "message expired"),
            ShellCommError::TrustViolation => write!(f, "trust violation"),
            ShellCommError::RateLimited => write!(f, "rate limited"),
        }
    }
}

impl std::error::Error for ShellCommError {}

// ---------------------------------------------------------------------------
// ShellMessage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub message_type: ShellMessageType,
    pub payload: String,
    pub timestamp: u64,
    pub priority: MessagePriority,
    pub requires_ack: bool,
    pub acknowledged: bool,
}

impl ShellMessage {
    pub fn new(from: &str, to: &str, msg_type: ShellMessageType, payload: &str) -> Self {
        Self {
            id: format!("msg-{}", simple_hash(format!("{from}{to}{payload}{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()))),
            from: from.to_string(),
            to: to.to_string(),
            message_type: msg_type,
            payload: payload.to_string(),
            timestamp: current_tick(),
            priority: MessagePriority::Normal,
            requires_ack: false,
            acknowledged: false,
        }
    }

    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }

    pub fn is_expired(&self, current_tick: u64, ttl_ticks: u64) -> bool {
        current_tick > self.timestamp + ttl_ticks
    }
}

// ---------------------------------------------------------------------------
// PeerShell
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerShell {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub address: String,
    pub protocol: String,
    pub connected: bool,
    pub last_contact: u64,
    pub briefing_interval: u64,
    pub last_briefing: Option<u64>,
    pub trust_level: TrustLevel,
}

impl PeerShell {
    pub fn new(name: &str, address: &str, protocol: &str) -> Self {
        Self {
            id: format!("peer-{}", simple_hash(name.to_string())),
            name: name.to_string(),
            kind: "shell".to_string(),
            address: address.to_string(),
            protocol: protocol.to_string(),
            connected: false,
            last_contact: 0,
            briefing_interval: 300,
            last_briefing: None,
            trust_level: TrustLevel::Partial,
        }
    }

    pub fn is_reachable(&self) -> bool {
        self.connected
    }

    pub fn needs_briefing(&self, current_tick: u64) -> bool {
        match self.last_briefing {
            None => true,
            Some(last) => current_tick >= last + self.briefing_interval,
        }
    }

    pub fn describe(&self) -> String {
        let status = if self.connected { "connected" } else { "disconnected" };
        format!(
            "[{}] {} ({}) @ {} via {} — trust:{} {}",
            status, self.name, self.kind, self.address, self.protocol, self.trust_level,
            self.last_briefing.map_or("".to_string(), |t| format!(" last_briefing:{t}"))
        )
    }
}

// ---------------------------------------------------------------------------
// Briefing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingRoom {
    pub id: String,
    pub gravity: f64,
    pub alert: String,
    pub tile_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingEnsign {
    pub id: String,
    pub status: String,
    pub room: Option<String>,
    pub energy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Briefing {
    pub from_shell: String,
    pub timestamp: u64,
    pub rooms: Vec<BriefingRoom>,
    pub ensigns: Vec<BriefingEnsign>,
    pub conservation_remaining: f64,
    pub alerts: Vec<String>,
    pub summary: String,
    pub requires_attention: bool,
}

impl Briefing {
    pub fn new(from: &str) -> Self {
        Self {
            from_shell: from.to_string(),
            timestamp: current_tick(),
            rooms: Vec::new(),
            ensigns: Vec::new(),
            conservation_remaining: 1.0,
            alerts: Vec::new(),
            summary: String::new(),
            requires_attention: false,
        }
    }

    pub fn add_room(&mut self, id: &str, gravity: f64, alert: &str, tile_count: u32) {
        self.rooms.push(BriefingRoom {
            id: id.to_string(),
            gravity,
            alert: alert.to_string(),
            tile_count,
        });
    }

    pub fn add_alert(&mut self, alert: &str) {
        self.alerts.push(alert.to_string());
        self.requires_attention = true;
    }

    pub fn render(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("=== Briefing from {} ===", self.from_shell));
        lines.push(format!("Conservation: {:.1}%", self.conservation_remaining * 100.0));
        if !self.rooms.is_empty() {
            lines.push("Rooms:".to_string());
            for r in &self.rooms {
                lines.push(format!("  {} — g={:.2} alert={} tiles={}", r.id, r.gravity, r.alert, r.tile_count));
            }
        }
        if !self.ensigns.is_empty() {
            lines.push("Ensigns:".to_string());
            for e in &self.ensigns {
                lines.push(format!(
                    "  {} — {} energy={:.1}%{}",
                    e.id,
                    e.status,
                    e.energy * 100.0,
                    e.room.as_ref().map_or(String::new(), |r| format!(" room={r}"))
                ));
            }
        }
        if !self.alerts.is_empty() {
            lines.push("Alerts:".to_string());
            for a in &self.alerts {
                lines.push(format!("  ⚠ {a}"));
            }
        }
        if !self.summary.is_empty() {
            lines.push(format!("Summary: {}", self.summary));
        }
        if self.requires_attention {
            lines.push("⚠ REQUIRES ATTENTION".to_string());
        }
        lines.join("\n")
    }

    pub fn needs_escalation(&self) -> bool {
        self.requires_attention || self.conservation_remaining < 0.2
    }
}

// ---------------------------------------------------------------------------
// ShellRoute
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellRoute {
    pub from_pattern: String,
    pub to_pattern: String,
    pub enabled: bool,
}

impl ShellRoute {
    pub fn matches(&self, from: &str, to: &str) -> bool {
        if !self.enabled {
            return false;
        }
        let from_ok = self.from_pattern == "*" || self.from_pattern == from;
        let to_ok = self.to_pattern == "*" || self.to_pattern == to;
        from_ok && to_ok
    }
}

// ---------------------------------------------------------------------------
// InterShellBus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterShellBus {
    pub local_shell_id: String,
    pub peers: HashMap<String, PeerShell>,
    pub outbox: Vec<ShellMessage>,
    pub inbox: Vec<ShellMessage>,
    pub routes: Vec<ShellRoute>,
}

impl InterShellBus {
    pub fn new(shell_id: &str) -> Self {
        Self {
            local_shell_id: shell_id.to_string(),
            peers: HashMap::new(),
            outbox: Vec::new(),
            inbox: Vec::new(),
            routes: Vec::new(),
        }
    }

    pub fn register_peer(&mut self, peer: PeerShell) {
        self.peers.insert(peer.id.clone(), peer);
    }

    pub fn send(&mut self, target: &str, message: ShellMessage) -> Result<(), ShellCommError> {
        // Check routes — if any enabled route blocks this, reject
        let blocked = self.routes.iter().any(|r| !r.matches(&self.local_shell_id, target))
            && self.routes.iter().any(|r| r.matches(&self.local_shell_id, target));

        if blocked {
            // Check if no route explicitly allows it when routes exist
            let has_matching_route = self.routes.iter().any(|r| r.matches(&self.local_shell_id, target));
            if !has_matching_route && !self.routes.is_empty() {
                return Err(ShellCommError::ProtocolError("route denied".to_string()));
            }
        }

        let peer = self.peers.values().find(|p| p.name == target || p.id == target)
            .ok_or(ShellCommError::PeerNotFound)?;

        if !peer.is_reachable() {
            return Err(ShellCommError::PeerUnreachable);
        }

        self.outbox.push(message);
        Ok(())
    }

    pub fn broadcast(&mut self, message: ShellMessage) -> Result<(), ShellCommError> {
        let reachable_count = self.peers.values().filter(|p| p.is_reachable()).count();
        if reachable_count == 0 {
            return Err(ShellCommError::PeerUnreachable);
        }
        for peer in self.peers.values().filter(|p| p.is_reachable()) {
            let mut msg = message.clone();
            msg.to = peer.id.clone();
            self.outbox.push(msg);
        }
        Ok(())
    }

    pub fn receive(&mut self) -> Vec<ShellMessage> {
        std::mem::take(&mut self.inbox)
    }

    pub fn send_briefing(&mut self, target: &str, briefing: Briefing) -> Result<(), ShellCommError> {
        let peer = self.peers.values().find(|p| p.name == target || p.id == target)
            .ok_or(ShellCommError::PeerNotFound)?;

        if !peer.is_reachable() {
            return Err(ShellCommError::PeerUnreachable);
        }

        if !peer.trust_level.can_receive_state() {
            return Err(ShellCommError::TrustViolation);
        }

        let msg = ShellMessage::new(
            &self.local_shell_id,
            &peer.id,
            ShellMessageType::Briefing,
            &serde_json::to_string(&briefing).unwrap_or_default(),
        );
        self.outbox.push(msg);
        Ok(())
    }

    pub fn request_briefing(&mut self, target: &str) -> Result<(), ShellCommError> {
        let peer = self.peers.values().find(|p| p.name == target || p.id == target)
            .ok_or(ShellCommError::PeerNotFound)?;

        if !peer.is_reachable() {
            return Err(ShellCommError::PeerUnreachable);
        }

        let msg = ShellMessage::new(
            &self.local_shell_id,
            &peer.id,
            ShellMessageType::Query,
            "request_briefing",
        );
        self.outbox.push(msg);
        Ok(())
    }

    pub fn peer_status(&self, peer_id: &str) -> Option<&PeerShell> {
        self.peers.get(peer_id)
    }

    pub fn list_peers(&self) -> Vec<&PeerShell> {
        self.peers.values().collect()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn current_tick() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn simple_hash(input: String) -> u64 {
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash % 100_000
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_connected_peer(name: &str) -> PeerShell {
        let mut p = PeerShell::new(name, &format!("{name}.local"), "tcp");
        p.connected = true;
        p
    }

    // --- TrustLevel ---

    #[test]
    fn trust_full_receives_state() {
        assert!(TrustLevel::Full.can_receive_state());
    }
    #[test]
    fn trust_partial_receives_state() {
        assert!(TrustLevel::Partial.can_receive_state());
    }
    #[test]
    fn trust_minimal_cannot_receive_state() {
        assert!(!TrustLevel::Minimal.can_receive_state());
    }
    #[test]
    fn trust_untrusted_cannot_receive_state() {
        assert!(!TrustLevel::Untrusted.can_receive_state());
    }

    #[test]
    fn trust_full_can_send_commands() {
        assert!(TrustLevel::Full.can_send_commands());
    }
    #[test]
    fn trust_partial_cannot_send_commands() {
        assert!(!TrustLevel::Partial.can_send_commands());
    }
    #[test]
    fn trust_display() {
        assert_eq!(TrustLevel::Full.to_string(), "full");
        assert_eq!(TrustLevel::Untrusted.to_string(), "untrusted");
    }

    // --- MessagePriority ---

    #[test]
    fn priority_default_is_normal() {
        assert_eq!(MessagePriority::default(), MessagePriority::Normal);
    }
    #[test]
    fn priority_display() {
        assert_eq!(MessagePriority::Critical.to_string(), "critical");
        assert_eq!(MessagePriority::High.to_string(), "high");
        assert_eq!(MessagePriority::Low.to_string(), "low");
    }

    // --- ShellMessageType ---

    #[test]
    fn msg_type_display() {
        assert_eq!(ShellMessageType::Briefing.to_string(), "briefing");
        assert_eq!(ShellMessageType::StateUpdate.to_string(), "state_update");
        assert_eq!(ShellMessageType::Command.to_string(), "command");
        assert_eq!(ShellMessageType::Query.to_string(), "query");
        assert_eq!(ShellMessageType::Response.to_string(), "response");
        assert_eq!(ShellMessageType::Alert.to_string(), "alert");
        assert_eq!(ShellMessageType::Heartbeat.to_string(), "heartbeat");
        assert_eq!(ShellMessageType::Handshake.to_string(), "handshake");
    }

    // --- ShellCommError ---

    #[test]
    fn error_display() {
        assert_eq!(ShellCommError::PeerNotFound.to_string(), "peer not found");
        assert_eq!(ShellCommError::PeerUnreachable.to_string(), "peer unreachable");
        assert_eq!(ShellCommError::ProtocolError("x".into()).to_string(), "protocol error: x");
        assert_eq!(ShellCommError::MessageExpired.to_string(), "message expired");
        assert_eq!(ShellCommError::TrustViolation.to_string(), "trust violation");
        assert_eq!(ShellCommError::RateLimited.to_string(), "rate limited");
    }

    // --- ShellMessage ---

    #[test]
    fn message_new_fields() {
        let msg = ShellMessage::new("oracle", "proart", ShellMessageType::Alert, "fire in sector 7");
        assert_eq!(msg.from, "oracle");
        assert_eq!(msg.to, "proart");
        assert_eq!(msg.message_type, ShellMessageType::Alert);
        assert_eq!(msg.payload, "fire in sector 7");
        assert_eq!(msg.priority, MessagePriority::Normal);
        assert!(!msg.requires_ack);
        assert!(!msg.acknowledged);
        assert!(msg.id.starts_with("msg-"));
    }

    #[test]
    fn message_acknowledge() {
        let mut msg = ShellMessage::new("a", "b", ShellMessageType::Heartbeat, "ping");
        assert!(!msg.acknowledged);
        msg.acknowledge();
        assert!(msg.acknowledged);
    }

    #[test]
    fn message_not_expired_initially() {
        let msg = ShellMessage::new("a", "b", ShellMessageType::Query, "?");
        assert!(!msg.is_expired(current_tick(), 1000));
    }

    #[test]
    fn message_expired_after_ttl() {
        let mut msg = ShellMessage::new("a", "b", ShellMessageType::Query, "?");
        msg.timestamp = 100;
        assert!(msg.is_expired(1200, 1000));
    }

    #[test]
    fn message_serde_roundtrip() {
        let msg = ShellMessage::new("x", "y", ShellMessageType::Command, "run");
        let json = serde_json::to_string(&msg).unwrap();
        let back: ShellMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.from, msg.from);
        assert_eq!(back.to, msg.to);
        assert_eq!(back.payload, msg.payload);
    }

    // --- PeerShell ---

    #[test]
    fn peer_new_defaults() {
        let p = PeerShell::new("Oracle", "10.0.0.1:9000", "tcp");
        assert_eq!(p.name, "Oracle");
        assert_eq!(p.address, "10.0.0.1:9000");
        assert_eq!(p.protocol, "tcp");
        assert!(!p.connected);
        assert_eq!(p.trust_level, TrustLevel::Partial);
        assert!(!p.is_reachable());
    }

    #[test]
    fn peer_reachable_when_connected() {
        let mut p = PeerShell::new("P", "a", "tcp");
        assert!(!p.is_reachable());
        p.connected = true;
        assert!(p.is_reachable());
    }

    #[test]
    fn peer_needs_briefing_initially() {
        let p = PeerShell::new("P", "a", "tcp");
        assert!(p.needs_briefing(current_tick()));
    }

    #[test]
    fn peer_needs_briefing_after_interval() {
        let mut p = PeerShell::new("P", "a", "tcp");
        p.last_briefing = Some(100);
        p.briefing_interval = 50;
        assert!(p.needs_briefing(200));
        assert!(!p.needs_briefing(140));
    }

    #[test]
    fn peer_describe_connected() {
        let mut p = PeerShell::new("Oracle", "10.0.0.1:9000", "tcp");
        p.connected = true;
        let desc = p.describe();
        assert!(desc.contains("connected"));
        assert!(desc.contains("Oracle"));
        assert!(desc.contains("partial"));
    }

    #[test]
    fn peer_describe_disconnected() {
        let p = PeerShell::new("Ghost", "nowhere", "udp");
        assert!(p.describe().contains("disconnected"));
    }

    #[test]
    fn peer_serde_roundtrip() {
        let p = PeerShell::new("Z", "addr", "ws");
        let json = serde_json::to_string(&p).unwrap();
        let back: PeerShell = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, p.name);
        assert_eq!(back.address, p.address);
    }

    // --- Briefing ---

    #[test]
    fn briefing_new_defaults() {
        let b = Briefing::new("oracle");
        assert_eq!(b.from_shell, "oracle");
        assert!(b.rooms.is_empty());
        assert!(b.ensigns.is_empty());
        assert!(b.alerts.is_empty());
        assert!(!b.requires_attention);
        assert_eq!(b.conservation_remaining, 1.0);
    }

    #[test]
    fn briefing_add_room() {
        let mut b = Briefing::new("x");
        b.add_room("bridge", 1.0, "none", 42);
        assert_eq!(b.rooms.len(), 1);
        assert_eq!(b.rooms[0].id, "bridge");
        assert_eq!(b.rooms[0].gravity, 1.0);
        assert_eq!(b.rooms[0].tile_count, 42);
    }

    #[test]
    fn briefing_add_alert() {
        let mut b = Briefing::new("x");
        b.add_alert("hull breach");
        assert_eq!(b.alerts.len(), 1);
        assert!(b.requires_attention);
    }

    #[test]
    fn briefing_render_basic() {
        let b = Briefing::new("oracle");
        let rendered = b.render();
        assert!(rendered.contains("oracle"));
        assert!(rendered.contains("100.0%"));
    }

    #[test]
    fn briefing_render_with_rooms() {
        let mut b = Briefing::new("oracle");
        b.add_room("engine", 0.8, "heat", 10);
        let rendered = b.render();
        assert!(rendered.contains("engine"));
        assert!(rendered.contains("0.80"));
    }

    #[test]
    fn briefing_render_with_alerts() {
        let mut b = Briefing::new("oracle");
        b.add_alert("fire");
        let rendered = b.render();
        assert!(rendered.contains("⚠"));
        assert!(rendered.contains("fire"));
        assert!(rendered.contains("REQUIRES ATTENTION"));
    }

    #[test]
    fn briefing_needs_escalation_when_alerts() {
        let mut b = Briefing::new("x");
        assert!(!b.needs_escalation());
        b.add_alert("bad");
        assert!(b.needs_escalation());
    }

    #[test]
    fn briefing_needs_escalation_when_low_conservation() {
        let mut b = Briefing::new("x");
        b.conservation_remaining = 0.1;
        assert!(b.needs_escalation());
    }

    #[test]
    fn briefing_no_escalation_when_ok() {
        let b = Briefing::new("x");
        assert!(!b.needs_escalation());
    }

    #[test]
    fn briefing_with_ensigns() {
        let mut b = Briefing::new("oracle");
        b.ensigns.push(BriefingEnsign {
            id: "e1".into(),
            status: "active".into(),
            room: Some("bridge".into()),
            energy: 0.75,
        });
        let rendered = b.render();
        assert!(rendered.contains("e1"));
        assert!(rendered.contains("active"));
        assert!(rendered.contains("bridge"));
    }

    #[test]
    fn briefing_serde_roundtrip() {
        let mut b = Briefing::new("shell1");
        b.add_room("r1", 9.8, "none", 5);
        b.add_alert("test");
        let json = serde_json::to_string(&b).unwrap();
        let back: Briefing = serde_json::from_str(&json).unwrap();
        assert_eq!(back.from_shell, "shell1");
        assert_eq!(back.rooms.len(), 1);
        assert!(back.requires_attention);
    }

    // --- ShellRoute ---

    #[test]
    fn route_wildcard_matches_everything() {
        let r = ShellRoute { from_pattern: "*".into(), to_pattern: "*".into(), enabled: true };
        assert!(r.matches("a", "b"));
    }

    #[test]
    fn route_specific_match() {
        let r = ShellRoute { from_pattern: "oracle".into(), to_pattern: "proart".into(), enabled: true };
        assert!(r.matches("oracle", "proart"));
        assert!(!r.matches("oracle", "other"));
        assert!(!r.matches("other", "proart"));
    }

    #[test]
    fn route_disabled_never_matches() {
        let r = ShellRoute { from_pattern: "*".into(), to_pattern: "*".into(), enabled: false };
        assert!(!r.matches("a", "b"));
    }

    #[test]
    fn route_wildcard_from() {
        let r = ShellRoute { from_pattern: "*".into(), to_pattern: "proart".into(), enabled: true };
        assert!(r.matches("anything", "proart"));
        assert!(!r.matches("anything", "other"));
    }

    // --- InterShellBus ---

    #[test]
    fn bus_new() {
        let bus = InterShellBus::new("hermes");
        assert_eq!(bus.local_shell_id, "hermes");
        assert!(bus.peers.is_empty());
        assert!(bus.outbox.is_empty());
        assert!(bus.inbox.is_empty());
    }

    #[test]
    fn bus_register_peer() {
        let mut bus = InterShellBus::new("hermes");
        let p = make_connected_peer("oracle");
        let pid = p.id.clone();
        bus.register_peer(p);
        assert_eq!(bus.peers.len(), 1);
        assert!(bus.peer_status(&pid).is_some());
    }

    #[test]
    fn bus_list_peers() {
        let mut bus = InterShellBus::new("hermes");
        bus.register_peer(make_connected_peer("a"));
        bus.register_peer(make_connected_peer("b"));
        assert_eq!(bus.list_peers().len(), 2);
    }

    #[test]
    fn bus_send_success() {
        let mut bus = InterShellBus::new("hermes");
        bus.register_peer(make_connected_peer("oracle"));
        let msg = ShellMessage::new("hermes", "oracle", ShellMessageType::Command, "go");
        assert!(bus.send("oracle", msg).is_ok());
        assert_eq!(bus.outbox.len(), 1);
    }

    #[test]
    fn bus_send_peer_not_found() {
        let mut bus = InterShellBus::new("hermes");
        let msg = ShellMessage::new("hermes", "ghost", ShellMessageType::Command, "go");
        assert_eq!(bus.send("ghost", msg), Err(ShellCommError::PeerNotFound));
    }

    #[test]
    fn bus_send_peer_unreachable() {
        let mut bus = InterShellBus::new("hermes");
        let p = PeerShell::new("oracle", "a", "tcp"); // not connected
        bus.register_peer(p);
        let msg = ShellMessage::new("hermes", "oracle", ShellMessageType::Command, "go");
        assert_eq!(bus.send("oracle", msg), Err(ShellCommError::PeerUnreachable));
    }

    #[test]
    fn bus_broadcast_success() {
        let mut bus = InterShellBus::new("hermes");
        bus.register_peer(make_connected_peer("a"));
        bus.register_peer(make_connected_peer("b"));
        let msg = ShellMessage::new("hermes", "*", ShellMessageType::Alert, "fire");
        assert!(bus.broadcast(msg).is_ok());
        assert_eq!(bus.outbox.len(), 2);
    }

    #[test]
    fn bus_broadcast_no_reachable_peers() {
        let mut bus = InterShellBus::new("hermes");
        bus.register_peer(PeerShell::new("a", "a", "tcp")); // disconnected
        let msg = ShellMessage::new("hermes", "*", ShellMessageType::Alert, "fire");
        assert_eq!(bus.broadcast(msg), Err(ShellCommError::PeerUnreachable));
    }

    #[test]
    fn bus_broadcast_skips_disconnected() {
        let mut bus = InterShellBus::new("hermes");
        bus.register_peer(make_connected_peer("a"));
        bus.register_peer(PeerShell::new("b", "b", "tcp")); // disconnected
        let msg = ShellMessage::new("hermes", "*", ShellMessageType::Heartbeat, "ping");
        assert!(bus.broadcast(msg).is_ok());
        assert_eq!(bus.outbox.len(), 1);
    }

    #[test]
    fn bus_receive_drains_inbox() {
        let mut bus = InterShellBus::new("hermes");
        bus.inbox.push(ShellMessage::new("a", "hermes", ShellMessageType::Heartbeat, "ping"));
        bus.inbox.push(ShellMessage::new("b", "hermes", ShellMessageType::Query, "?"));
        let msgs = bus.receive();
        assert_eq!(msgs.len(), 2);
        assert!(bus.inbox.is_empty());
    }

    #[test]
    fn bus_send_briefing_success() {
        let mut bus = InterShellBus::new("hermes");
        bus.register_peer(make_connected_peer("oracle"));
        let briefing = Briefing::new("hermes");
        assert!(bus.send_briefing("oracle", briefing).is_ok());
        assert_eq!(bus.outbox.len(), 1);
        assert_eq!(bus.outbox[0].message_type, ShellMessageType::Briefing);
    }

    #[test]
    fn bus_send_briefing_trust_violation() {
        let mut bus = InterShellBus::new("hermes");
        let mut p = make_connected_peer("suspicious");
        p.trust_level = TrustLevel::Minimal;
        bus.register_peer(p);
        let briefing = Briefing::new("hermes");
        assert_eq!(bus.send_briefing("suspicious", briefing), Err(ShellCommError::TrustViolation));
    }

    #[test]
    fn bus_send_briefing_unreachable() {
        let mut bus = InterShellBus::new("hermes");
        let p = PeerShell::new("oracle", "a", "tcp");
        bus.register_peer(p);
        let briefing = Briefing::new("hermes");
        assert_eq!(bus.send_briefing("oracle", briefing), Err(ShellCommError::PeerUnreachable));
    }

    #[test]
    fn bus_send_briefing_peer_not_found() {
        let mut bus = InterShellBus::new("hermes");
        let briefing = Briefing::new("hermes");
        assert_eq!(bus.send_briefing("ghost", briefing), Err(ShellCommError::PeerNotFound));
    }

    #[test]
    fn bus_request_briefing_success() {
        let mut bus = InterShellBus::new("hermes");
        bus.register_peer(make_connected_peer("oracle"));
        assert!(bus.request_briefing("oracle").is_ok());
        assert_eq!(bus.outbox.len(), 1);
        assert_eq!(bus.outbox[0].message_type, ShellMessageType::Query);
        assert_eq!(bus.outbox[0].payload, "request_briefing");
    }

    #[test]
    fn bus_request_briefing_peer_not_found() {
        let mut bus = InterShellBus::new("hermes");
        assert_eq!(bus.request_briefing("ghost"), Err(ShellCommError::PeerNotFound));
    }

    #[test]
    fn bus_peer_status_found() {
        let mut bus = InterShellBus::new("hermes");
        let p = make_connected_peer("oracle");
        let pid = p.id.clone();
        bus.register_peer(p);
        let status = bus.peer_status(&pid).unwrap();
        assert_eq!(status.name, "oracle");
    }

    #[test]
    fn bus_peer_status_not_found() {
        let bus = InterShellBus::new("hermes");
        assert!(bus.peer_status("ghost").is_none());
    }

    // --- Integration ---

    #[test]
    fn full_flow_oracle_to_proart_via_hermes() {
        let mut hermes = InterShellBus::new("hermes");
        let mut oracle = make_connected_peer("oracle");
        oracle.trust_level = TrustLevel::Full;
        hermes.register_peer(oracle);

        let mut proart = make_connected_peer("proart");
        proart.trust_level = TrustLevel::Partial;
        hermes.register_peer(proart);

        // Hermes sends briefing to oracle
        let mut briefing = Briefing::new("hermes");
        briefing.add_room("bridge", 1.0, "none", 20);
        briefing.add_room("engine", 0.5, "heat", 15);
        briefing.add_alert("coolant leak in engine");
        assert!(hermes.send_briefing("oracle", briefing).is_ok());

        // Hermes requests briefing from proart
        assert!(hermes.request_briefing("proart").is_ok());

        // Simulate response arriving in inbox
        let response = ShellMessage::new("proart", "hermes", ShellMessageType::Response, "all good");
        hermes.inbox.push(response);

        let msgs = hermes.receive();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].from, "proart");
        assert_eq!(msgs[0].payload, "all good");

        assert_eq!(hermes.outbox.len(), 2);
    }

    #[test]
    fn bus_serde_roundtrip() {
        let mut bus = InterShellBus::new("hermes");
        bus.register_peer(make_connected_peer("a"));
        let json = serde_json::to_string(&bus).unwrap();
        let back: InterShellBus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.local_shell_id, "hermes");
        assert_eq!(back.peers.len(), 1);
    }

    #[test]
    fn untrusted_peer_cannot_receive_briefing() {
        let mut bus = InterShellBus::new("hermes");
        let mut p = make_connected_peer("spy");
        p.trust_level = TrustLevel::Untrusted;
        bus.register_peer(p);
        assert_eq!(
            bus.send_briefing("spy", Briefing::new("hermes")),
            Err(ShellCommError::TrustViolation)
        );
    }

    #[test]
    fn briefing_render_summary() {
        let mut b = Briefing::new("shell1");
        b.summary = "All systems nominal.".to_string();
        let rendered = b.render();
        assert!(rendered.contains("All systems nominal."));
    }
}
