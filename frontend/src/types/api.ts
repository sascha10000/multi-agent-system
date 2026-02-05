// API types matching the Rust backend (mas-api/src/models.rs)

export interface RegisterSystemRequest {
  name: string;
  config: import('./agent').SystemConfigJson;
}

export interface RegisterSystemResponse {
  name: string;
  message: string;
  agent_count: number;
  created_at: string;
}

export interface CreateSessionRequest {
  system_name: string;
}

export interface CreateSessionResponse {
  id: string;
  system_name: string;
  created_at: string;
  message: string;
}

export interface SessionPromptRequest {
  content: string;
  target_agent?: string;
  include_context?: boolean;
  context_limit?: number;
}

export interface PromptResultResponse {
  type: 'response';
  content: string;
  from: string;
}

export interface PromptResultTimeout {
  type: 'timeout';
  message: string;
}

export interface PromptResultNotified {
  type: 'notified';
}

export type PromptResult = PromptResultResponse | PromptResultTimeout | PromptResultNotified;

export interface SessionPromptResponse {
  message_id: string;
  session_id: string;
  target_agent: string;
  result: PromptResult;
  elapsed_ms: number;
  context?: MessageResponse[];
}

export interface MessageResponse {
  id: string;
  from: string;
  to: string;
  content: string;
  timestamp: string;
  metadata?: Record<string, unknown>;
}

export interface SessionHistoryResponse {
  session_id: string;
  messages: MessageResponse[];
  total: number;
}

export interface ApiError {
  error: string;
  details?: string;
}
