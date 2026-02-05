// Types matching the Rust API models in crates/mas-api/src/models.rs

export interface SystemSummary {
  name: string;
  agent_count: number;
  agents: string[];
  created_at: string;
}

export interface ListSystemsResponse {
  systems: SystemSummary[];
  total: number;
}

export interface ConnectionInfo {
  target: string;
  connection_type: 'blocking' | 'notify';
  timeout_secs?: number;
}

export interface AgentInfo {
  name: string;
  role: string;
  routing: boolean;
  connections: ConnectionInfo[];
}

export interface SystemDetailResponse {
  name: string;
  agent_count: number;
  agents: AgentInfo[];
  global_timeout_secs: number;
  created_at: string;
}

export interface RegisterSystemRequest {
  name: string;
  config: SystemConfigJson;
}

export interface RegisterSystemResponse {
  name: string;
  message: string;
  agent_count: number;
  created_at: string;
}

export interface DeleteSystemResponse {
  name: string;
  message: string;
}

export interface SendPromptRequest {
  content: string;
  target_agent?: string;
}

export type PromptResult =
  | { type: 'response'; content: string; from: string }
  | { type: 'timeout'; message: string }
  | { type: 'notified' };

export interface SendPromptResponse {
  message_id: string;
  target_agent: string;
  result: PromptResult;
  elapsed_ms: number;
}

// Configuration types (matching mas-core config_loader)

export interface SystemConfigJson {
  system: SystemSettings;
  llm_providers: Record<string, LlmProviderConfig>;
  agents: AgentConfigJson[];
}

export interface SystemSettings {
  global_timeout_secs: number;
}

export interface LlmProviderConfig {
  type: 'ollama';
  base_url: string;
  default_model: string;
}

export interface AgentConfigJson {
  name: string;
  role: string;
  system_prompt: string;
  handler: HandlerConfig;
  connections: Record<string, ConnectionConfig>;
}

export interface HandlerConfig {
  provider: string;
  model?: string;
  routing: boolean;
  options?: CompletionOptions;
}

export interface ConnectionConfig {
  type: 'blocking' | 'notify';
  timeout_secs?: number;
}

export interface CompletionOptions {
  temperature?: number;
  max_tokens?: number;
}

// API Error response
export interface ApiError {
  error: string;
  message: string;
}
