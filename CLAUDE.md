# Multi-Agent System

A Rust-based multi-agent system with configurable connections, parallel messaging, timeout handling, and **dynamic LLM-based routing**.

## Workspace Structure

```
multi-agent-system/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── mas-core/           # Core library (agents, routing, LLM)
│   └── mas-cli/            # CLI binary
├── examples/               # JSON configuration examples
└── CLAUDE.md
```

## Build & Test

```bash
cargo build                 # Build all crates
cargo test                  # Run all tests
cargo run -p mas-cli        # Run CLI (shows usage)
```

## CLI Usage

```bash
# Run demo with mock handlers
cargo run -p mas-cli

# Run LLM routing demo (requires Ollama)
cargo run -p mas-cli -- --llm

# Load and run system from JSON config
cargo run -p mas-cli -- --config examples/basic_routing.json

# Validate config without running (no Ollama needed)
cargo run -p mas-cli -- --config examples/basic_routing.json --dry-run

# Send a prompt (auto-selects Coordinator or first routing agent)
cargo run -p mas-cli -- --config examples/basic_routing.json --prompt "What is AI?"

# Send to a specific agent
cargo run -p mas-cli -- --config examples/expert_panel.json --prompt "Should we use Rust?" --to Moderator

# Quiet mode (suppress logs, only show response)
cargo run -p mas-cli -- --config examples/basic_routing.json --prompt "Hello" -q
```

## Architecture

### Core Components (mas-core)

- **Agent** (`agent.rs`): Entities with name, role, system_prompt, and connections
- **Connection** (`connection.rs`): Blocking (wait for response) or Notify (fire-and-forget)
- **Message** (`message.rs`): Communication unit with UUID, sender, receiver, content
- **AgentSystem** (`agent_system.rs`): Orchestrates message routing and parallel execution
- **Decision** (`decision.rs`): LLM routing decisions (Response, Forward, ResponseAndForward)
- **ConfigLoader** (`config_loader.rs`): JSON configuration parsing and validation

### Connection Types

1. **Blocking**: Sender waits for response with configurable timeout
2. **Notify**: Fire-and-forget, sender continues immediately

### Handler Types

1. **MessageHandler**: Simple handlers that return `Option<String>`
2. **RoutingHandler**: LLM-aware handlers that return `HandlerDecision` for dynamic routing

## JSON Configuration

The system can be configured via JSON files. See `examples/` for full examples.

### Schema

```json
{
  "system": {
    "global_timeout_secs": 60
  },
  "llm_providers": {
    "default": {
      "type": "ollama",
      "base_url": "http://localhost:11434",
      "default_model": "llama3.2"
    }
  },
  "agents": [
    {
      "name": "Coordinator",
      "role": "Routes requests",
      "system_prompt": "You coordinate work.",
      "handler": {
        "provider": "default",
        "model": "llama3.2",
        "routing": true,
        "options": { "temperature": 0.3, "max_tokens": 500 }
      },
      "connections": {
        "Worker": { "type": "blocking", "timeout_secs": 60 },
        "Logger": { "type": "notify" }
      }
    }
  ]
}
```

### Validation Rules

1. Agent names must be unique
2. Connection targets must reference existing agents
3. No self-connections allowed
4. Provider references must exist in `llm_providers`
5. `timeout_secs` is only meaningful for `blocking` connections

## Library Usage

### Simple Handler

```rust
use mas_core::{AgentSystem, AgentBuilder, SystemConfig, MessageHandler};
use std::sync::Arc;

let system = AgentSystem::new(SystemConfig::with_timeout_secs(5));

let coordinator = AgentBuilder::new("Coordinator")
    .system_prompt("You coordinate work.")
    .blocking_connection("Worker")
    .notify_connection("Logger")
    .build();

system.register_agent(coordinator, Arc::new(MyHandler)).await?;
let result = system.send_message("Coordinator", "Worker", "Process this").await?;
```

### LLM Routing Handler

```rust
use mas_core::{AgentSystem, AgentBuilder, SystemConfig, LlmHandler, OllamaProvider};
use std::sync::Arc;

let system = Arc::new(AgentSystem::new(SystemConfig::with_timeout_secs(60)));
let provider = Arc::new(OllamaProvider::detect().await?);

let coordinator = AgentBuilder::new("Coordinator")
    .system_prompt("Route research to Researcher, analysis to Analyst.")
    .blocking_connection("Researcher")
    .blocking_connection("Analyst")
    .build();

let handler = LlmHandler::new(provider)
    .with_routing()
    .with_options(CompletionOptions::new().temperature(0.3));

AgentSystem::register_routing_agent(system.clone(), coordinator, Arc::new(handler)).await?;
```

### Load from JSON

```rust
use mas_core::load_system_from_json;
use std::path::Path;

let system = load_system_from_json(Path::new("config.json")).await?;
```

## Dynamic LLM-Based Routing

The system supports dynamic routing where an LLM decides how to handle messages:

```rust
// LLM returns JSON decisions:
{ "response": "direct answer" }                    // Respond directly
{ "forward_to": [{ "agent": "X", "message": "..." }] }  // Forward to agents
{ "response": "ack", "forward_to": [...] }         // Both respond and forward
```

### Routing Flow

1. Message arrives at routing agent
2. Notify connections auto-fire (Logger, etc.)
3. LLM handler processes message with connection info in prompt
4. LLM returns JSON routing decision
5. System executes decision (forward in parallel, synthesize responses)
6. Final response sent to original sender

## Key Types

### HandlerDecision

```rust
enum HandlerDecision {
    Response { content: String },
    Forward { targets: Vec<ForwardTarget> },
    ResponseAndForward { content: String, targets: Vec<ForwardTarget> },
    None,
}
```

### ForwardTarget

```rust
struct ForwardTarget {
    agent: String,    // Target agent name
    message: String,  // Message to send
}
```
