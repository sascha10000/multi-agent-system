'use client';

import { useState, useRef, useEffect, useCallback } from 'react';
import ReactMarkdown from 'react-markdown';
import type { SystemConfigJson } from '../types/agent';
import type {
  SessionPromptResponse,
  CreateSessionResponse,
  PromptResult,
} from '../types/api';

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  agent?: string;
  timestamp: Date;
}

interface ChatPanelProps {
  isOpen: boolean;
  onClose: () => void;
  config: SystemConfigJson;
  systemName: string;
}

const API_BASE = 'http://localhost:8080/api/v1';

export default function ChatPanel({ isOpen, onClose, config, systemName }: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<'idle' | 'registering' | 'creating_session' | 'ready' | 'error'>('idle');

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isOpen]);

  // Helper to extract detailed error message from API response
  const extractErrorMessage = (errorData: Record<string, unknown>, fallback: string): string => {
    const parts: string[] = [];

    if (errorData.error && typeof errorData.error === 'string') {
      parts.push(errorData.error);
    }
    if (errorData.details && typeof errorData.details === 'string') {
      parts.push(errorData.details);
    }
    if (errorData.message && typeof errorData.message === 'string' && !parts.includes(errorData.message)) {
      parts.push(errorData.message);
    }

    return parts.length > 0 ? parts.join(': ') : fallback;
  };

  // Initialize system and session when panel opens
  const initializeSession = useCallback(async () => {
    if (sessionId || status === 'registering' || status === 'creating_session') return;

    setError(null);
    setStatus('registering');

    try {
      // First, try to register the system (or update if it exists)
      const registerRes = await fetch(`${API_BASE}/systems`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: systemName, config }),
      });

      if (!registerRes.ok) {
        const errorData = await registerRes.json().catch(() => ({}));
        // If system already exists, try to update it
        if (registerRes.status === 409) {
          const updateRes = await fetch(`${API_BASE}/systems/${encodeURIComponent(systemName)}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ config }),
          });
          if (!updateRes.ok) {
            const updateErrorData = await updateRes.json().catch(() => ({}));
            throw new Error(extractErrorMessage(updateErrorData, 'Failed to update system'));
          }
        } else {
          throw new Error(extractErrorMessage(errorData, `Failed to register system (${registerRes.status})`));
        }
      }

      setStatus('creating_session');

      // Create a new session
      const sessionRes = await fetch(`${API_BASE}/sessions`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ system_name: systemName }),
      });

      if (!sessionRes.ok) {
        const errorData = await sessionRes.json().catch(() => ({}));
        throw new Error(extractErrorMessage(errorData, `Failed to create session (${sessionRes.status})`));
      }

      const sessionData: CreateSessionResponse = await sessionRes.json();
      setSessionId(sessionData.id);
      setStatus('ready');

      // Add welcome message
      setMessages([{
        id: 'welcome',
        role: 'system',
        content: `Connected to "${systemName}". You can now chat with the multi-agent system.`,
        timestamp: new Date(),
      }]);

    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to initialize';
      setError(message);
      setStatus('error');
      console.error('Session initialization error:', err);
    }
  }, [config, systemName, sessionId, status]);

  useEffect(() => {
    if (isOpen && status === 'idle') {
      initializeSession();
    }
  }, [isOpen, status, initializeSession]);

  const sendMessage = async () => {
    if (!input.trim() || !sessionId || isLoading) return;

    const userMessage: ChatMessage = {
      id: `user-${Date.now()}`,
      role: 'user',
      content: input.trim(),
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput('');
    setIsLoading(true);
    setError(null);

    try {
      const res = await fetch(`${API_BASE}/sessions/${sessionId}/prompt`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          content: userMessage.content,
          include_context: true,
          context_limit: 5,
        }),
      });

      if (!res.ok) {
        const errorData = await res.json().catch(() => ({}));
        throw new Error(errorData.error || 'Failed to send message');
      }

      const data: SessionPromptResponse = await res.json();

      let assistantContent: string;
      let agentName: string | undefined;

      if (data.result.type === 'response') {
        assistantContent = data.result.content;
        agentName = data.result.from;
      } else if (data.result.type === 'timeout') {
        assistantContent = `Request timed out: ${data.result.message}`;
      } else {
        assistantContent = 'Message sent (no response expected)';
      }

      const assistantMessage: ChatMessage = {
        id: `assistant-${Date.now()}`,
        role: 'assistant',
        content: assistantContent,
        agent: agentName,
        timestamp: new Date(),
      };

      setMessages((prev) => [...prev, assistantMessage]);

    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to send message');
      // Add error message to chat
      setMessages((prev) => [...prev, {
        id: `error-${Date.now()}`,
        role: 'system',
        content: `Error: ${err instanceof Error ? err.message : 'Failed to send message'}`,
        timestamp: new Date(),
      }]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  const resetSession = () => {
    setSessionId(null);
    setMessages([]);
    setStatus('idle');
    setError(null);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-y-0 right-0 w-[450px] bg-zinc-900 border-l border-zinc-700 shadow-2xl flex flex-col z-50">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-700 bg-zinc-800">
        <div className="flex items-center gap-3">
          <div className={`w-2 h-2 rounded-full ${status === 'ready' ? 'bg-green-500' : status === 'error' ? 'bg-red-500' : 'bg-yellow-500 animate-pulse'}`} />
          <div>
            <h2 className="font-semibold text-zinc-100">Chat</h2>
            <p className="text-xs text-zinc-400">{systemName}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={resetSession}
            className="p-1.5 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-700 rounded transition-colors"
            title="New session"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
          </button>
          <button
            onClick={onClose}
            className="p-1.5 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-700 rounded transition-colors"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      {/* Status bar for errors */}
      {error && (
        <div className="px-4 py-3 bg-red-900/50 border-b border-red-800">
          <div className="flex items-start justify-between gap-2">
            <p className="text-red-300 text-sm flex-1">{error}</p>
            {status === 'error' && (
              <button
                onClick={resetSession}
                className="px-2 py-1 text-xs font-medium text-red-200 bg-red-800/50 hover:bg-red-800 rounded transition-colors whitespace-nowrap"
              >
                Retry
              </button>
            )}
          </div>
        </div>
      )}

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {status === 'error' && (
          <div className="flex flex-col items-center justify-center py-8 px-4">
            <div className="w-12 h-12 rounded-full bg-red-900/30 flex items-center justify-center mb-4">
              <svg className="w-6 h-6 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
            </div>
            <h3 className="text-zinc-200 font-medium mb-2">Connection Failed</h3>
            <p className="text-zinc-400 text-sm text-center mb-4 max-w-xs">{error}</p>
            <button
              onClick={resetSession}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
            >
              Try Again
            </button>
          </div>
        )}

        {status !== 'ready' && status !== 'error' && (
          <div className="flex items-center justify-center py-8">
            <div className="flex items-center gap-2 text-zinc-400">
              <svg className="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              <span>
                {status === 'registering' ? 'Registering system...' : 'Creating session...'}
              </span>
            </div>
          </div>
        )}

        {messages.map((msg) => (
          <div
            key={msg.id}
            className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
          >
            <div
              className={`max-w-[85%] rounded-lg px-3 py-2 ${
                msg.role === 'user'
                  ? 'bg-blue-600 text-white'
                  : msg.role === 'system'
                  ? 'bg-zinc-700 text-zinc-300 text-sm italic'
                  : 'bg-zinc-800 text-zinc-100 border border-zinc-700'
              }`}
            >
              {msg.agent && (
                <div className="text-xs text-zinc-400 mb-1 font-medium">
                  {msg.agent}
                </div>
              )}
              <div className="chat-markdown">
                <ReactMarkdown>{msg.content}</ReactMarkdown>
              </div>
            </div>
          </div>
        ))}

        {isLoading && (
          <div className="flex justify-start">
            <div className="bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2">
              <div className="flex items-center gap-1">
                <div className="w-2 h-2 bg-zinc-500 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                <div className="w-2 h-2 bg-zinc-500 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                <div className="w-2 h-2 bg-zinc-500 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
              </div>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-4 border-t border-zinc-700 bg-zinc-800">
        <div className="flex gap-2">
          <input
            ref={inputRef}
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={status === 'ready' ? 'Type a message...' : 'Connecting...'}
            disabled={status !== 'ready' || isLoading}
            className="flex-1 px-3 py-2 bg-zinc-900 border border-zinc-600 rounded-lg text-zinc-100 placeholder-zinc-500 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none disabled:opacity-50"
          />
          <button
            onClick={sendMessage}
            disabled={!input.trim() || status !== 'ready' || isLoading}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
