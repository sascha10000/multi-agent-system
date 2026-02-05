// Types matching the Rust API (mas-core/src/config_loader.rs)

export type RoutingBehavior = 'best' | 'all' | 'direct_first';

export interface HandlerConfig {
  provider?: string;
  model?: string;
  routing?: boolean;
  routing_behavior?: RoutingBehavior;
  options?: {
    temperature?: number;
    max_tokens?: number;
  };
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

export interface SystemConfigJson {
  system?: SystemSettings;
  llm_providers?: Record<string, LlmProviderConfig>;
  agents: AgentConfig[];
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
};
