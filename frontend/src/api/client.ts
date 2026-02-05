import type {
  ListSystemsResponse,
  SystemDetailResponse,
  RegisterSystemRequest,
  RegisterSystemResponse,
  DeleteSystemResponse,
  SendPromptRequest,
  SendPromptResponse,
  ApiError,
} from '../types/api';

const API_BASE = '/api/v1';

class ApiClient {
  private async request<T>(
    endpoint: string,
    options?: RequestInit
  ): Promise<T> {
    const response = await fetch(`${API_BASE}${endpoint}`, {
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
      ...options,
    });

    if (!response.ok) {
      const error: ApiError = await response.json().catch(() => ({
        error: 'Unknown',
        message: response.statusText,
      }));
      throw new Error(error.message || error.error);
    }

    return response.json();
  }

  // System Management

  async listSystems(): Promise<ListSystemsResponse> {
    return this.request<ListSystemsResponse>('/systems');
  }

  async getSystem(name: string): Promise<SystemDetailResponse> {
    return this.request<SystemDetailResponse>(`/systems/${encodeURIComponent(name)}`);
  }

  async createSystem(request: RegisterSystemRequest): Promise<RegisterSystemResponse> {
    return this.request<RegisterSystemResponse>('/systems', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  async deleteSystem(name: string): Promise<DeleteSystemResponse> {
    return this.request<DeleteSystemResponse>(`/systems/${encodeURIComponent(name)}`, {
      method: 'DELETE',
    });
  }

  // Prompt Handling

  async sendPrompt(
    systemName: string,
    request: SendPromptRequest
  ): Promise<SendPromptResponse> {
    return this.request<SendPromptResponse>(
      `/systems/${encodeURIComponent(systemName)}/prompt`,
      {
        method: 'POST',
        body: JSON.stringify(request),
      }
    );
  }
}

export const api = new ApiClient();
