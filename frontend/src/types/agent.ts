// Types matching the Rust API (mas-core/src/config_loader.rs)

export type RoutingBehavior = 'best' | 'all' | 'direct_first';
export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
export type ResponseFormat = 'json' | 'text' | 'markdown';
export type EndpointType = 'http' | 'mcp';

export interface HandlerConfig {
  provider?: string;
  model?: string;
  routing?: boolean;
  routing_behavior?: RoutingBehavior;
  options?: {
    temperature?: number;
    max_tokens?: number;
  };
  max_turns?: number;
}

export interface ConnectionConfig {
  type: 'blocking' | 'notify';
  timeout_secs?: number;
}

export interface AgentConfig {
  name: string;
  system_prompt?: string;
  handler?: HandlerConfig;
  connections?: Record<string, ConnectionConfig>;
  entry_point?: boolean;
}

export interface LlmProviderConfig {
  type: 'ollama' | 'openai';
  base_url?: string;
  default_model?: string;
  api_key?: string;
}

export interface SystemSettings {
  global_timeout_secs?: number;
}

// Tool configuration types (matching Rust tool.rs)
export interface ToolEndpointConfig {
  url: string;
  type?: EndpointType;  // 'http' (default) or 'mcp'
  method?: HttpMethod;
  headers?: Record<string, string>;
  body_template?: Record<string, unknown>;
  mcp_tool_name?: string;  // Required when type is 'mcp'
}

export interface ResponseMappingConfig {
  extract_path?: string;
  format?: ResponseFormat;
}

export interface ToolEndpointConfigFull {
  url: string;
  type?: EndpointType;
  method?: HttpMethod;
  headers?: Record<string, string>;
  body_template?: Record<string, unknown>;
  mcp_tool_name?: string;
}

export interface ToolConfig {
  name: string;
  description: string;
  parameters?: Record<string, unknown>;
  endpoint: ToolEndpointConfigFull;
  response_mapping?: ResponseMappingConfig;
  timeout_secs?: number;
}

// Database configuration types (matching Rust database.rs)
export type DatabaseType = 'sqlite' | 'postgres' | 'mysql';

export interface DatabaseConfig {
  name: string;
  description: string;
  connection_string: string;
  database_type?: DatabaseType;
  max_connections?: number;
  timeout_secs?: number;
  read_only?: boolean;
}

export interface SystemConfigJson {
  system?: SystemSettings;
  llm_providers?: Record<string, LlmProviderConfig>;
  agents: AgentConfig[];
  tools?: ToolConfig[];
  databases?: DatabaseConfig[];
  editor_metadata?: {
    node_positions?: Record<string, { x: number; y: number }>;
  };
}

// React Flow specific types
// Index signature needed for React Flow's Record<string, unknown> constraint
export interface AgentNodeData {
  [key: string]: unknown;
  name: string;
  systemPrompt: string;
  provider: string;
  model: string;
  routing: boolean;
  routingBehavior: RoutingBehavior;
  temperature: number;
  maxTokens: number;
  entryPoint: boolean;
  maxTurns: number;
}

// Tool node data for React Flow
export interface ToolNodeData {
  [key: string]: unknown;
  name: string;
  description: string;
  endpointType: EndpointType;  // 'http' or 'mcp'
  endpointUrl: string;
  endpointMethod: HttpMethod;
  mcpToolName: string;  // MCP tool name (used when endpointType is 'mcp')
  headers: Record<string, string>;
  bodyTemplate: string; // JSON string for the body template (http only)
  parameters: string; // JSON string for the parameters schema
  extractPath: string;
  responseFormat: ResponseFormat;
  timeoutSecs: number;
}

// Default values for new agents
export const defaultAgentData: AgentNodeData = {
  name: 'New Agent',
  systemPrompt: 'You are a helpful assistant.',
  provider: 'default',
  model: 'llama3.2',
  routing: false,
  routingBehavior: 'best',
  temperature: 0.7,
  maxTokens: 1000,
  entryPoint: false,
  maxTurns: 1,
};

// Database node data for React Flow
export interface DatabaseNodeData {
  [key: string]: unknown;
  name: string;
  description: string;
  databaseType: DatabaseType;
  connectionString: string;
  maxConnections: number;
  timeoutSecs: number;
  readOnly: boolean;
}

// Default values for new databases
export const defaultDatabaseData: DatabaseNodeData = {
  name: 'New Database',
  description: 'A database connection',
  databaseType: 'sqlite',
  connectionString: 'sqlite://data.db',
  maxConnections: 5,
  timeoutSecs: 30,
  readOnly: true,
};

// REST API node data for React Flow (simplified HTTP tool)
export interface RestApiNodeData {
  [key: string]: unknown;
  name: string;
  description: string;
  endpointUrl: string;
  endpointMethod: HttpMethod;
  headers: Record<string, string>;
  bodyTemplate: string; // JSON string for the body template
  parameters: string; // JSON string for the parameters schema
  extractPath: string;
  responseFormat: ResponseFormat;
  timeoutSecs: number;
}

// Default values for new REST API nodes
export const defaultRestApiData: RestApiNodeData = {
  name: 'New REST API',
  description: 'A REST API endpoint',
  endpointUrl: 'https://api.example.com/endpoint',
  endpointMethod: 'GET',
  headers: {},
  bodyTemplate: '',
  parameters: '{\n  "type": "object",\n  "properties": {}\n}',
  extractPath: '',
  responseFormat: 'json',
  timeoutSecs: 30,
};

// Default values for new tools (MCP by default)
export const defaultToolData: ToolNodeData = {
  name: 'New Tool',
  description: 'An MCP tool',
  endpointType: 'mcp',
  endpointUrl: 'https://example.com/mcp',
  endpointMethod: 'POST',
  mcpToolName: 'tool_name',
  headers: {},
  bodyTemplate: '',
  parameters: '{\n  "type": "object",\n  "properties": {\n    "query": { "type": "string", "description": "The query parameter" }\n  },\n  "required": ["query"]\n}',
  extractPath: '',
  responseFormat: 'json',
  timeoutSecs: 30,
};
