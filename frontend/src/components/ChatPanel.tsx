'use client';

import { useState, useRef, useEffect, useCallback } from 'react';
import ReactMarkdown from 'react-markdown';
import type { SystemConfigJson } from '../types/agent';
import type {
  SessionPromptResponse,
  CreateSessionResponse,
  PromptResult,
  AgentTraceStep,
} from '../types/api';

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  agent?: string;
  timestamp: Date;
  trace?: AgentTraceStep[];
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
  const [verboseMode, setVerboseMode] = useState(false);
  const [expandedTraces, setExpandedTraces] = useState<Set<string>>(new Set());
  const [selectedTraceStep, setSelectedTraceStep] = useState<AgentTraceStep | null>(null);

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
        trace: data.trace && data.trace.length > 0 ? data.trace : undefined,
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
    setExpandedTraces(new Set());
  };

  const toggleTraceExpanded = (messageId: string) => {
    setExpandedTraces((prev) => {
      const next = new Set(prev);
      if (next.has(messageId)) {
        next.delete(messageId);
      } else {
        next.add(messageId);
      }
      return next;
    });
  };

  const getStepTypeColor = (stepType: string) => {
    switch (stepType) {
      case 'request':
        return 'text-blue-400 bg-blue-900/30';
      case 'response':
        return 'text-green-400 bg-green-900/30';
      case 'forward':
        return 'text-amber-400 bg-amber-900/30';
      case 'synthesis':
        return 'text-purple-400 bg-purple-900/30';
      default:
        return 'text-zinc-400 bg-zinc-700';
    }
  };

  const getStepTypeIcon = (stepType: string) => {
    switch (stepType) {
      case 'request':
        return '→';
      case 'response':
        return '←';
      case 'forward':
        return '↗';
      case 'synthesis':
        return '⊕';
      default:
        return '•';
    }
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
            onClick={() => setVerboseMode(!verboseMode)}
            className={`p-1.5 rounded transition-colors ${
              verboseMode
                ? 'text-amber-400 bg-amber-900/30 hover:bg-amber-900/50'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-700'
            }`}
            title={verboseMode ? 'Hide agent trace' : 'Show agent trace'}
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
            </svg>
          </button>
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
          <div key={msg.id} className="space-y-2">
            <div
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

            {/* Agent Trace Display */}
            {verboseMode && msg.trace && msg.trace.length > 0 && (
              <div className="ml-4">
                <button
                  onClick={() => toggleTraceExpanded(msg.id)}
                  className="flex items-center gap-2 text-xs text-amber-400 hover:text-amber-300 transition-colors"
                >
                  <svg
                    className={`w-3 h-3 transition-transform ${expandedTraces.has(msg.id) ? 'rotate-90' : ''}`}
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                  </svg>
                  <span>{msg.trace.length} agent communication{msg.trace.length !== 1 ? 's' : ''}</span>
                </button>

                {expandedTraces.has(msg.id) && (
                  <div className="mt-2 space-y-1 border-l-2 border-amber-900/50 pl-3">
                    {msg.trace.map((step, idx) => (
                      <button
                        key={idx}
                        onClick={() => setSelectedTraceStep(step)}
                        className="w-full text-left text-xs bg-zinc-800/50 rounded p-2 border border-zinc-700/50 hover:bg-zinc-700/50 hover:border-zinc-600 transition-colors cursor-pointer"
                      >
                        <div className="flex items-center gap-2 mb-1">
                          <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${getStepTypeColor(step.step_type)}`}>
                            {getStepTypeIcon(step.step_type)} {step.step_type}
                          </span>
                          <span className="text-zinc-400">
                            <span className="text-zinc-300 font-medium">{step.from}</span>
                            {' → '}
                            <span className="text-zinc-300 font-medium">{step.to}</span>
                          </span>
                          <span className="ml-auto text-zinc-500 text-[10px]">Click to expand</span>
                        </div>
                        <div className="text-zinc-400 mt-1 whitespace-pre-wrap break-words max-h-20 overflow-hidden">
                          {step.content.length > 150 ? `${step.content.slice(0, 150)}...` : step.content}
                        </div>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            )}
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

      {/* Trace Step Modal */}
      {selectedTraceStep && (
        <div
          className="fixed inset-0 bg-black/60 flex items-center justify-center z-[60] p-4"
          onClick={() => setSelectedTraceStep(null)}
        >
          <div
            className="bg-zinc-900 border border-zinc-700 rounded-lg shadow-2xl max-w-2xl w-full max-h-[80vh] flex flex-col"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Modal Header */}
            <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-700 bg-zinc-800 rounded-t-lg">
              <div className="flex items-center gap-3">
                <span className={`px-2 py-1 rounded text-xs font-medium ${getStepTypeColor(selectedTraceStep.step_type)}`}>
                  {getStepTypeIcon(selectedTraceStep.step_type)} {selectedTraceStep.step_type}
                </span>
                <span className="text-zinc-300">
                  <span className="font-medium">{selectedTraceStep.from}</span>
                  <span className="text-zinc-500 mx-2">→</span>
                  <span className="font-medium">{selectedTraceStep.to}</span>
                </span>
              </div>
              <button
                onClick={() => setSelectedTraceStep(null)}
                className="p-1.5 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-700 rounded transition-colors"
              >
                <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>

            {/* Modal Content */}
            <div className="flex-1 overflow-y-auto p-4">
              <div className="chat-markdown bg-zinc-800/50 rounded-lg p-4 border border-zinc-700">
                <ReactMarkdown>{selectedTraceStep.content}</ReactMarkdown>
              </div>
            </div>

            {/* Modal Footer */}
            <div className="px-4 py-3 border-t border-zinc-700 bg-zinc-800/50 rounded-b-lg flex justify-between items-center">
              <span className="text-xs text-zinc-500">
                {selectedTraceStep.content.length} characters
              </span>
              <button
                onClick={() => {
                  navigator.clipboard.writeText(selectedTraceStep.content);
                }}
                className="px-3 py-1.5 text-xs font-medium text-zinc-300 bg-zinc-700 hover:bg-zinc-600 rounded transition-colors flex items-center gap-1.5"
              >
                <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                </svg>
                Copy
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
