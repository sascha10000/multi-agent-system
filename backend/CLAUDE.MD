# Multi-Agent Backend System

A Rust-based multi-agent communication system that allows agents to connect and communicate with each other.

## Prerequisites

Before running the project, ensure you have:

1. **Rust and Cargo** - Install from [rustup.rs](https://rustup.rs/)
2. **Ollama** - Required for LLM-powered agent responses
   - Install: `curl -fsSL https://ollama.com/install.sh | sh`
   - Or visit: [https://ollama.com/download](https://ollama.com/download)
3. **Ollama Model** - Pull the model used by agents:
   ```bash
   ollama pull gemma3:4b
   ```

### Starting Ollama

Before running the application, start the Ollama service:

```bash
ollama serve
```

This will start Ollama on the default port `11434`. Keep this running in a separate terminal.

## Overview

This project implements a backend system for managing multiple agents that can communicate with each other. Agents can only send messages to other agents they are connected to, ensuring controlled and secure communication channels.

Agents use Ollama-powered LLMs to process messages and generate intelligent responses based on their assigned roles.

## Architecture

### Core Components

#### `Agent`
Represents an individual agent in the system.

**Fields:**
- `name: String` - Unique identifier for the agent
- `role: String` - The prompt/role that defines the agent's task and behavior
- `connections: Arc<Mutex<HashSet<String>>>` - Thread-safe set of connected agent names

**Methods:**
- `new(name: String, role: String) -> Self` - Creates a new agent
- `connect_to(&self, other_agent_name: &str)` - Connects to another agent
- `disconnect_from(&self, other_agent_name: &str)` - Disconnects from another agent
- `is_connected_to(&self, other_agent_name: &str) -> bool` - Checks connection status
- `get_connections(&self) -> Vec<String>` - Returns list of connected agents

#### `Message`
Represents a message between agents.

**Fields:**
- `from: String` - Sender agent name
- `to: String` - Recipient agent name
- `content: String` - Message content

#### `AgentSystem`
Manages the entire multi-agent system.

**Methods:**
- `new() -> Self` - Creates a new agent system
- `add_agent(&mut self, agent: Agent) -> Result<(), String>` - Adds an agent to the system
- `remove_agent(&mut self, name: &str) -> Result<Agent, String>` - Removes an agent
- `get_agent(&self, name: &str) -> Option<&Agent>` - Retrieves an agent by name
- `connect_agents(&mut self, agent1_name: &str, agent2_name: &str) -> Result<(), String>` - Connects two agents bidirectionally
- `disconnect_agents(&mut self, agent1_name: &str, agent2_name: &str) -> Result<(), String>` - Disconnects two agents
- `send_message(&self, from: &str, to: &str, content: String) -> Result<Message, String>` - Sends a message (only if agents are connected)
- `list_agents(&self) -> Vec<&Agent>` - Lists all agents

## Key Features

1. **Connection-Based Communication**: Agents can only communicate with agents they are explicitly connected to
2. **Thread-Safe**: Uses `Arc<Mutex<>>` for safe concurrent access to agent connections
3. **Bidirectional Connections**: The `connect_agents` method creates bidirectional connections automatically
4. **Error Handling**: Comprehensive error handling for missing agents and unauthorized communications
5. **Type Safety**: Leverages Rust's type system for safe agent management

## Usage Example

```rust
use multi_agent_backend::{Agent, AgentSystem};

fn main() {
    // Create agent system
    let mut system = AgentSystem::new();

    // Create agents with specific roles
    let researcher = Agent::new(
        "Researcher".to_string(),
        "You are a researcher agent. Your task is to gather and analyze information.".to_string(),
    );

    let analyst = Agent::new(
        "Analyst".to_string(),
        "You are an analyst agent. Your task is to process data and provide insights.".to_string(),
    );

    // Add agents to system
    system.add_agent(researcher).unwrap();
    system.add_agent(analyst).unwrap();

    // Connect agents
    system.connect_agents("Researcher", "Analyst").unwrap();

    // Send message (will succeed because agents are connected)
    match system.send_message("Researcher", "Analyst", "Here's my data".to_string()) {
        Ok(msg) => println!("Message sent: {:?}", msg),
        Err(e) => println!("Error: {}", e),
    }
}
```

## Running the Project

**Important:** Make sure Ollama is running before starting the application!

```bash
# Terminal 1: Start Ollama service
ollama serve

# Terminal 2: Build and run the project
cd backend
cargo build

# Run the demo
cargo run

# Run tests
cargo test
```

## Testing

The project includes unit tests for:
- Agent creation
- Agent connections
- Connection-based message validation

Run tests with:
```bash
cargo test
```

## Future Enhancements

Potential improvements for the system:

1. **Async Communication**: Implement async/await for message handling
2. **Message Queue**: Add message queuing system for asynchronous delivery
3. **Agent State Management**: Track agent states (active, idle, busy)
4. **Message History**: Store and retrieve message history
5. **Network Communication**: Extend to support agents across network boundaries
6. **Agent Groups**: Support for agent groups/channels
7. **Message Filtering**: Filter messages based on content or priority
8. **API Layer**: REST or GraphQL API for external interaction
9. **Persistence**: Database integration for agent and message persistence
10. **Authentication**: Secure agent-to-agent authentication

## Design Principles

- **Explicit Connections**: Agents must be explicitly connected before communication
- **Thread Safety**: All shared state is protected with appropriate synchronization
- **Error Handling**: All operations return Results for proper error handling
- **Modularity**: Clean separation between Agent and AgentSystem responsibilities
- **Testability**: Comprehensive test coverage for core functionality
