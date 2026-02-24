# Visual Agent Editor Guide

This guide documents all configurable parameters available in the Multi-Agent System visual editor.

## Overview

The visual editor allows you to design multi-agent systems using a drag-and-drop canvas. You can create agents, tools, and connect them to build sophisticated LLM-powered workflows.

---

## Agent Parameters

Agents are the core building blocks of your system. Each agent is backed by an LLM and can process messages, route to other agents, or call tools.

### Basic Info

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `name` | string | **Yes** | "New Agent" | Unique identifier for the agent. Must be unique within the system. Used for routing and connections. |
| `systemPrompt` | string | No | "You are a helpful assistant." | Instructions that define the agent's behavior, personality, and capabilities. This is prepended to every conversation with the agent. |

### LLM Settings

| Parameter | Type | Range | Default | Description |
|-----------|------|-------|---------|-------------|
| `provider` | string | - | "default" | Reference to an LLM provider defined in `llm_providers`. Common values: "default", "openai", "anthropic". |
| `model` | string | - | "llama3.2" | The model identifier to use. Examples: "llama3.2", "gpt-4", "gpt-4o", "claude-3-opus". |
| `temperature` | number | 0 - 2 | 0.7 | Controls randomness/creativity. Lower values (0-0.3) = more deterministic, focused responses. Higher values (1-2) = more creative, varied responses. |
| `maxTokens` | number | 1 - 128,000 | 1000 | Maximum number of tokens in the response. Higher values allow longer responses but increase latency and cost. |

### Routing Settings

Routing enables an agent to dynamically delegate work to connected agents based on the incoming message.

| Parameter | Type | Options | Default | Description |
|-----------|------|---------|---------|-------------|
| `routing` | boolean | true/false | false | Enable dynamic message routing. When enabled, the LLM decides whether to respond directly, forward to other agents, or both. |
| `routingBehavior` | enum | See below | "best" | Controls how the agent delegates to connected agents (only applies when `routing` is true). |

#### Routing Behavior Options

| Value | Description | Use Case |
|-------|-------------|----------|
| `best` | Forward to the single most appropriate connected agent based on the query | Default. Good for hierarchical systems where one expert handles each query. |
| `all` | Forward to **all** connected agents and synthesize their responses | Use for expert panels, voting systems, or when you need multiple perspectives. |
| `direct_first` | Try to answer directly; only forward if the agent lacks expertise | Efficient routing that minimizes unnecessary delegation. |

---

## Tool Parameters

Tools extend agent capabilities by connecting to external services via HTTP APIs or MCP (Model Context Protocol) servers.

### Basic Info

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `name` | string | **Yes** | "New Tool" | Unique identifier for the tool. Used in agent tool calls. |
| `description` | string | No | "An MCP tool" | Human-readable description of what the tool does. This is shown to the LLM to help it decide when to use the tool. |

### Endpoint Configuration

| Parameter | Type | Options | Default | Description |
|-----------|------|---------|---------|-------------|
| `endpointType` | enum | `mcp`, `http` | "mcp" | Protocol type for the tool endpoint. |

#### MCP Endpoint (Model Context Protocol)

MCP is a standardized protocol for LLM tool integration using JSON-RPC 2.0.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `endpointUrl` | string | **Yes** | URL of the MCP server (e.g., `https://example.com/mcp`) |
| `mcpToolName` | string | **Yes** | The name of the tool registered on the MCP server (e.g., `search_jobs`, `get_weather`) |
| `headers` | text | No | HTTP headers, one per line in `Key: Value` format. Supports `${ENV_VAR}` substitution. |

#### HTTP Endpoint

Custom HTTP requests with full control over method, headers, and body.

| Parameter | Type | Options | Default | Description |
|-----------|------|---------|---------|-------------|
| `endpointMethod` | enum | GET, POST, PUT, DELETE, PATCH | "POST" | HTTP method for the request |
| `endpointUrl` | string | - | - | Full URL of the API endpoint |
| `headers` | text | - | - | HTTP headers, one per line in `Key: Value` format. Supports `${ENV_VAR}` substitution. |
| `bodyTemplate` | JSON | - | - | JSON body template. Use `${param}` for parameter substitution from the parameters schema. |

### Parameters Schema

| Parameter | Type | Format | Description |
|-----------|------|--------|-------------|
| `parameters` | JSON | JSON Schema | Defines the parameters the tool accepts. This schema is shown to the LLM so it knows how to call the tool. Must be valid JSON Schema. |

**Example Parameters Schema:**
```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "description": "The search query"
    },
    "limit": {
      "type": "number",
      "description": "Maximum results to return"
    }
  },
  "required": ["query"]
}
```

### Response Mapping

| Parameter | Type | Options | Default | Description |
|-----------|------|---------|---------|-------------|
| `extractPath` | string | JSONPath | "" | JSONPath expression to extract a specific part of the response (e.g., `$.data.results`, `$.items[0].name`) |
| `responseFormat` | enum | json, text, markdown | "json" | How to parse and format the response for the LLM |

### Timeout

| Parameter | Type | Range | Default | Description |
|-----------|------|-------|---------|-------------|
| `timeoutSecs` | number | 1 - 300 | 30 | Request timeout in seconds. Increase for slow APIs. |

---

## Visual Connections

Connections define how agents communicate with each other and with tools.

### Creating Connections

1. **Agent-to-Agent**: Drag from an agent's output handle (right side) to another agent's input handle (left side)
2. **Agent-to-Tool**: Drag from an agent's output handle to a tool's input handle

### Connection Behavior

| Connection Type | Behavior |
|-----------------|----------|
| Agent → Agent | Creates a **blocking** connection with 60s timeout. The source agent can forward messages to the target and wait for a response. |
| Agent → Tool | The agent can call the tool during message processing. Tool responses are incorporated into the agent's response. |
| Tool → Agent | **Not allowed**. Tools cannot initiate connections; they only respond to calls. |

### How Connections Affect Routing

When an agent has `routing: true` enabled:
- The agent sees all connected agents/tools in its system prompt
- The LLM decides which connections to use based on the incoming message
- `routingBehavior` controls whether it picks one (`best`), all (`all`), or tries direct response first (`direct_first`)

---

## Workflow Tips

### Import/Export

The editor supports JSON import/export for sharing and version control:

1. **Export**: Use the JSON tab to copy the system configuration
2. **Import**: Paste valid JSON into the JSON tab and it will update the visual editor
3. **Version Control**: Export JSON and commit to git for history tracking

### Testing via Chat

1. Save your system configuration
2. Navigate to the Chat page
3. Create a new session for your system
4. Send test messages to verify agent behavior
5. Check routing by observing which agents respond

### Best Practices

1. **Name agents descriptively** - Use names like "Researcher", "Analyst", "Coordinator" that reflect their role
2. **Write clear system prompts** - Be specific about the agent's expertise and how it should respond
3. **Use routing for complex systems** - A coordinator agent with `routing: true` can delegate to specialists
4. **Set appropriate temperatures** - Use low (0.1-0.3) for factual tasks, higher (0.7-1.0) for creative tasks
5. **Test incrementally** - Start with simple systems and add complexity gradually

### Common Patterns

#### Expert Panel
```
Coordinator (routing: true, routingBehavior: all)
  ├── Expert A
  ├── Expert B
  └── Expert C
```
Coordinator queries all experts and synthesizes responses.

#### Hierarchical Routing
```
Coordinator (routing: true, routingBehavior: best)
  ├── Research Agent
  │     └── Search Tool
  ├── Analysis Agent
  └── Writing Agent
```
Coordinator routes to the most appropriate specialist.

#### Tool-Augmented Agent
```
Assistant Agent
  ├── Search Tool
  ├── Calculator Tool
  └── Weather Tool
```
Single agent with multiple tool capabilities.

---

## Quick Reference

### Agent Defaults
- Temperature: 0.7
- Max Tokens: 1000
- Provider: "default"
- Model: "llama3.2"
- Routing: disabled

### Tool Defaults
- Endpoint Type: MCP
- Response Format: JSON
- Timeout: 30 seconds

### Validation Rules
1. Agent names must be unique
2. No self-connections allowed
3. Tools cannot be connection sources
4. MCP tools require `mcpToolName`
5. JSON fields must be valid JSON
