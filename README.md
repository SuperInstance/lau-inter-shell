# lau-inter-shell

**Inter-shell communication bus — async messaging between shells with trust, routing, and briefings**

When shells need to talk to each other, they don't shout into the void. They send **briefings** through a structured communication bus that enforces trust levels, checks routes, and respects reachability. This is the nervous system of the LAU shell hierarchy.

---

## What This Does

`lau-inter-shell` provides a Rust library for **inter-process communication between shell instances**. A shell owns an `InterShellBus` that manages:

- **Peers** — other shells it can talk to, each with a trust level and connection state
- **Messages** — typed, timestamped, trackable payloads (briefings, queries, commands, alerts, etc.)
- **Briefings** — rich status reports carrying room states, ensign health, conservation levels, and alerts
- **Routes** — from/to patterns that gate which messages can flow where
- **Trust enforcement** — untrusted peers can't receive state; only fully trusted peers can send commands

Everything is serializable to JSON for persistence or network transport.

---

## Key Idea

Communication is **trust-gated**. Before a briefing reaches a peer, three things are checked:

1. **Reachability** — Is the peer connected?
2. **Trust** — Does the peer's trust level permit this operation?
3. **Routes** — Does an enabled route allow this from→to pair?

| Trust Level | Can Receive State? | Can Send Commands? |
|---|---|---|
| `Full` | ✓ | ✓ |
| `Partial` | ✓ | ✗ |
| `Minimal` | ✗ | ✗ |
| `Untrusted` | ✗ | ✗ |

---

## Install

```toml
[dependencies]
lau-inter-shell = "0.1.0"
```

Requires Rust 2021 edition. Depends on `serde` (with `derive`) and `serde_json`.

---

## Quick Start

### Set up a bus and register peers

```rust
use lau_inter_shell::{InterShellBus, PeerShell, TrustLevel};

let mut bus = InterShellBus::new("hermes");

// Register a fully trusted, connected oracle
let mut oracle = PeerShell::new("oracle", "10.0.0.1:9000", "tcp");
oracle.connected = true;
oracle.trust_level = TrustLevel::Full;
bus.register_peer(oracle);
```

### Send a briefing

```rust
use lau_inter_shell::{Briefing, ShellMessageType};

let mut briefing = Briefing::new("hermes");
briefing.add_room("bridge", 1.0, "none", 20);
briefing.add_room("engine", 0.5, "heat", 15);
briefing.add_alert("coolant leak in engine");
briefing.conservation_remaining = 0.85;
briefing.summary = "Engine room overheating, bridge nominal.".to_string();

bus.send_briefing("oracle", briefing.clone()).unwrap();
```

### Request a briefing from a peer

```rust
bus.request_briefing("oracle").unwrap();
// A Query message with payload "request_briefing" appears in the outbox
```

### Receive messages

```rust
// Simulate an incoming response
bus.inbox.push(ShellMessage::new("oracle", "hermes", ShellMessageType::Response, "all good"));

let messages = bus.receive();
assert_eq!(messages.len(), 1);
assert_eq!(messages[0].payload, "all good");
```

### Broadcast to all reachable peers

```rust
let alert = ShellMessage::new("hermes", "*", ShellMessageType::Alert, "sector 7 fire");
bus.broadcast(alert).unwrap();
// One copy per reachable peer appears in outbox
```

### Render a briefing for display

```rust
let rendered = briefing.render();
// === Briefing from hermes ===
// Conservation: 85.0%
// Rooms:
//   bridge — g=1.00 alert=none tiles=20
//   engine — g=0.50 alert=heat tiles=15
// Alerts:
//   ⚠ coolant leak in engine
// Summary: Engine room overheating, bridge nominal.
// ⚠ REQUIRES ATTENTION
```

### Serialization

```rust
let json = serde_json::to_string(&bus).unwrap();
let restored: InterShellBus = serde_json::from_str(&json).unwrap();
assert_eq!(restored.peers.len(), bus.peers.len());
```

---

## API Reference

### `TrustLevel`

```rust
pub enum TrustLevel {
    Full,       // Full trust: receive state + send commands
    Partial,    // Can receive state, cannot send commands
    Minimal,    // Cannot receive state or send commands
    Untrusted,  // No privileges
}
```

- `can_receive_state() -> bool` — `Full` or `Partial`
- `can_send_commands() -> bool` — `Full` only

### `MessagePriority`

```rust
pub enum MessagePriority {
    Critical,
    High,
    Normal,   // default
    Low,
}
```

### `ShellMessageType`

```rust
pub enum ShellMessageType {
    Briefing,       // Rich status report
    StateUpdate,    // State change notification
    Command,        // Actionable instruction
    Query,          // Question / request
    Response,       // Answer to a query
    Alert,          // Urgent notification
    Heartbeat,      // Keep-alive ping
    Handshake,      // Connection establishment
}
```

### `ShellMessage`

| Field | Type | Purpose |
|---|---|---|
| `id` | `String` | Unique ID (`msg-{hash}`) |
| `from` | `String` | Sender shell ID |
| `to` | `String` | Recipient shell ID |
| `message_type` | `ShellMessageType` | What kind of message |
| `payload` | `String` | Body content |
| `timestamp` | `u64` | Unix seconds when created |
| `priority` | `MessagePriority` | Urgency level |
| `requires_ack` | `bool` | Whether sender wants acknowledgment |
| `acknowledged` | `bool` | Whether recipient has acknowledged |

**Methods:**
- `new(from, to, msg_type, payload)` — Create a message
- `acknowledge()` — Mark as acknowledged
- `is_expired(current_tick, ttl_ticks) -> bool` — Check if TTL has elapsed

### `PeerShell`

| Field | Type | Purpose |
|---|---|---|
| `id` | `String` | Unique peer ID (`peer-{hash}`) |
| `name` | `String` | Human-readable name |
| `kind` | `String` | Shell type (default: "shell") |
| `address` | `String` | Network address |
| `protocol` | `String` | Transport protocol (tcp, ws, udp) |
| `connected` | `bool` | Whether the peer is reachable |
| `last_contact` | `u64` | Last communication timestamp |
| `briefing_interval` | `u64` | Seconds between briefings (default: 300) |
| `last_briefing` | `Option<u64>` | When the last briefing was sent |
| `trust_level` | `TrustLevel` | Peer's trust level |

**Methods:**
- `is_reachable() -> bool` — `connected` shortcut
- `needs_briefing(current_tick) -> bool` — Whether the briefing interval has elapsed
- `describe() -> String` — Human-readable summary

### `Briefing`

A rich status report carrying the state of a shell's universe.

| Field | Type | Purpose |
|---|---|---|
| `from_shell` | `String` | Who sent this briefing |
| `timestamp` | `u64` | When it was created |
| `rooms` | `Vec<BriefingRoom>` | Room states (id, gravity, alert, tile_count) |
| `ensigns` | `Vec<BriefingEnsign>` | Ensign states (id, status, room, energy) |
| `conservation_remaining` | `f64` | Resource fraction remaining (0.0–1.0) |
| `alerts` | `Vec<String>` | Active alert messages |
| `summary` | `String` | Free-text summary |
| `requires_attention` | `bool` | Whether any alerts are active |

**Methods:**
- `add_room(id, gravity, alert, tile_count)` — Add a room report
- `add_alert(alert)` — Add an alert (sets `requires_attention = true`)
- `render() -> String` — Multi-line formatted briefing
- `needs_escalation() -> bool` — True if alerts exist or conservation < 20%

### `ShellRoute`

```rust
pub struct ShellRoute {
    pub from_pattern: String,   // Shell name or "*"
    pub to_pattern: String,     // Shell name or "*"
    pub enabled: bool,
}
```

- `matches(from, to) -> bool` — Checks if the from/to pair matches the pattern. `*` matches anything. Disabled routes never match.

### `InterShellBus`

The main communication hub.

| Field | Type | Purpose |
|---|---|---|
| `local_shell_id` | `String` | This shell's identity |
| `peers` | `HashMap<String, PeerShell>` | Known peers by ID |
| `outbox` | `Vec<ShellMessage>` | Messages to be delivered |
| `inbox` | `Vec<ShellMessage>` | Received messages |
| `routes` | `Vec<ShellRoute>` | Routing rules |

**Methods:**
- `new(shell_id)` — Create a bus
- `register_peer(peer)` — Add a peer
- `send(target, message) -> Result<(), ShellCommError>` — Send to a specific peer
- `broadcast(message) -> Result<(), ShellCommError>` — Send to all reachable peers
- `receive() -> Vec<ShellMessage>` — Drain the inbox
- `send_briefing(target, briefing) -> Result<(), ShellCommError>` — Send a briefing (trust-checked)
- `request_briefing(target) -> Result<(), ShellCommError>` — Ask a peer for a briefing
- `peer_status(peer_id) -> Option<&PeerShell>` — Look up a peer
- `list_peers() -> Vec<&PeerShell>` — All registered peers

### `ShellCommError`

```rust
pub enum ShellCommError {
    PeerNotFound,              // Unknown peer name/ID
    PeerUnreachable,           // Peer exists but disconnected
    ProtocolError(String),     // Route denial or protocol issue
    MessageExpired,            // TTL exceeded
    TrustViolation,            // Insufficient trust level
    RateLimited,               // Too many messages
}
```

Implements `std::error::Error` and `Display`.

---

## How It Works

### Message flow

```
Shell A                          InterShellBus A                Shell B
  |                                  |                            |
  |-- send_briefing(B, briefing) --> |                            |
  |                                  |-- outbox: ShellMessage --> |  (delivery by external transport)
  |                                  |                            |
  |                                  |<-- inbox: ShellMessage <-- |  (response arrives)
  |                                  |
  |<-- receive() --------------------|  (drains inbox)
```

The bus is **not a network transport** — it's a structured message queue. Actual delivery over TCP/WebSocket/etc. is handled externally. The bus provides:

1. **Envelope creation** — wrapping briefings and queries into `ShellMessage` with IDs and timestamps
2. **Trust gating** — checking that the target peer's trust level allows the operation
3. **Route filtering** — checking that an enabled route permits this from→to pair
4. **Reachability checks** — refusing to queue messages for disconnected peers

### ID generation

Message IDs use `msg-{djb2_hash(from + to + payload + timestamp_nanoseconds)}`. Peer IDs use `peer-{djb2_hash(name)}`. The `simple_hash` function is a classic DJB2 hash modulo 100,000.

### Briefing rendering

The `render()` method produces a human-readable multi-line string:

```
=== Briefing from hermes ===
Conservation: 85.0%
Rooms:
  bridge — g=1.00 alert=none tiles=20
  engine — g=0.50 alert=heat tiles=15
Ensigns:
  e1 — active energy=75.0% room=bridge
Alerts:
  ⚠ coolant leak in engine
Summary: Engine room overheating, bridge nominal.
⚠ REQUIRES ATTENTION
```

### Escalation logic

A briefing needs escalation when:
- Any alerts have been added (`requires_attention == true`), OR
- Conservation remaining drops below 20% (`conservation_remaining < 0.2`)

---

## The Math

### Trust as a lattice

Trust levels form a total order:

```
Full > Partial > Minimal > Untrusted
```

The capability predicates are monotone with respect to this order:
- `can_receive_state` is true for `{Full, Partial}` — the upper half of the lattice
- `can_send_commands` is true for `{Full}` — the top element only

This means: if a peer can send commands, it can also receive state (since `Full` implies both). Trust never grants partial privileges in unexpected combinations.

### Message TTL

Expiry is computed as:

```
is_expired(current, ttl) ⟺ current > timestamp + ttl
```

This is a **hard deadline**: the message is valid for exactly `ttl` ticks after creation, then permanently expired. No grace period.

### Route matching

Routes use a simple pattern language:
- `*` matches any shell name (wildcard)
- Any other string matches exactly

A route matches iff `from_pattern` matches `from` AND `to_pattern` matches `to`. The route must be `enabled`. Routes are additive — if ANY enabled route matches, the message flows.

### Briefing escalation

The escalation condition is:

```
needs_escalation = requires_attention ∨ (conservation_remaining < 0.2)
```

This is a disjunction of two independent signals. The conservation threshold (20%) acts as an **automatic escalation trigger** — even without explicit alerts, a shell running low on resources flags itself for attention.

---

## Testing

The crate includes **60 tests** covering:

- `TrustLevel` capabilities and display
- `MessagePriority` defaults and display
- `ShellMessageType` display for all 8 variants
- `ShellCommError` display for all 6 variants
- `ShellMessage` creation, acknowledgment, expiry, and serde round-trips
- `PeerShell` defaults, reachability, briefing intervals, descriptions, and serde round-trips
- `Briefing` construction, room/alert additions, rendering, escalation logic, and serde round-trips
- `ShellRoute` wildcard matching, specific matching, and disabled routes
- `InterShellBus` peer registration, send, broadcast, receive, briefing operations, trust violations, unreachable peers, and serde round-trips
- Full integration test: multi-peer flow with briefings, queries, and responses

Run with:

```bash
cargo test
```

---

## License

MIT
