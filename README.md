# lau-inter-shell

Oracle ↔ ProArt async bridge. When shells need to talk to each other, they send briefings through the inter-shell protocol. Briefings carry context, trust levels, and response expectations.

## The concept in 60 seconds

Shells run independently, but sometimes they need to communicate. The inter-shell bridge provides:

- **Briefings:** structured messages between shells (request, response, broadcast)
- **Trust levels:** shells don't blindly trust each other — trust is earned through provenance
- **Async:** send a briefing, continue working, pick up the response later
- **Oracle pattern:** one shell (Oracle) can answer questions from many others

## Quick start

```rust
use lau_inter_shell::{Bridge, Briefing, TrustLevel};

let mut bridge = Bridge::new();

// Oracle registers itself
bridge.register_oracle("hermes", TrustLevel::Full);

// Ensign sends a briefing
let briefing = Briefing::new("ensign_7", "hermes")
    .with_question("What's the conservation budget?")
    .with_priority(0.8);

bridge.send(briefing);

// Oracle responds
let response = bridge.await_response("ensign_7");
```

## Contributing

[Open an issue](https://github.com/SuperInstance/lau-inter-shell/issues) or PR.
