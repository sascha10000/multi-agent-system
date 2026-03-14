'use client';

import { useState, useRef, useEffect, useCallback, memo } from 'react';
import ReactMarkdown from 'react-markdown';
import type { SystemConfigJson } from '../types/agent';
import type {
  SessionPromptResponse,
  CreateSessionResponse,
  SessionSummary,
  MessageResponse,
  AgentTraceStep,
} from '../types/api';
import { authFetch } from '../lib/auth';

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  agent?: string;
  timestamp: Date;
  trace?: AgentTraceStep[];
  streaming?: boolean;
}

interface ChatPanelProps {
  isOpen: boolean;
  onClose: () => void;
  config: SystemConfigJson;
  systemName: string;
}

const API_BASE = process.env.NEXT_PUBLIC_API_BASE || '/api/v1';

function getStepTypeColor(stepType: string) {
  switch (stepType) {
    case 'request': return 'text-blue-300 bg-blue-500/15 border-blue-500/25';
    case 'response': return 'text-emerald-300 bg-emerald-500/15 border-emerald-500/25';
    case 'forward': return 'text-amber-300 bg-amber-500/15 border-amber-500/25';
    case 'synthesis': return 'text-purple-300 bg-purple-500/15 border-purple-500/25';
    default: return 'text-zinc-400 bg-zinc-700/50 border-zinc-600/40';
  }
}

function getStepTypeIcon(stepType: string) {
  switch (stepType) {
    case 'request': return '→';
    case 'response': return '←';
    case 'forward': return '↗';
    case 'synthesis': return '⊕';
    default: return '•';
  }
}

/**
 * Extract content from JSON response format if present
 * e.g., {"response": "hello"} → "hello"
 * If not JSON or no "response" field, returns original string
 */
function unwrapJsonResponse(content: string): string {
  try {
    const parsed = JSON.parse(content);
    if (typeof parsed === 'object' && parsed !== null && typeof parsed.response === 'string') {
      return parsed.response;
    }
  } catch {
    // Not valid JSON, return as-is
  }
  return content;
}

// ── Memoized message components ──────────────────────────────────────
// Prevents ReactMarkdown from re-parsing unchanged messages on every render.

const NormalMessage = memo(function NormalMessage({
  msg,
}: {
  msg: ChatMessage;
}) {
  if (msg.streaming) {
    return (
      <div className="flex justify-start">
        <div className="max-w-[85%] rounded-lg px-3 py-2 bg-zinc-800 text-zinc-100 border border-zinc-700">
          <div className="flex items-center gap-2 text-sm text-zinc-400">
            <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
            </svg>
            <span>
              {msg.trace && msg.trace.length > 0
                ? `Processing... (${msg.trace.length} step${msg.trace.length !== 1 ? 's' : ''})`
                : 'Sending to agents...'}
            </span>
          </div>
        </div>
      </div>
    );
  }

  return (
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
  );
});

const VerboseTraceStep = memo(function VerboseTraceStep({
  step,
  onSelect,
}: {
  step: AgentTraceStep;
  onSelect: (step: AgentTraceStep) => void;
}) {
  const getColor = (t: string) => {
    switch (t) {
      case 'request': return 'text-blue-300 bg-blue-500/15 border-blue-500/25';
      case 'response': return 'text-emerald-300 bg-emerald-500/15 border-emerald-500/25';
      case 'forward': return 'text-amber-300 bg-amber-500/15 border-amber-500/25';
      case 'synthesis': return 'text-purple-300 bg-purple-500/15 border-purple-500/25';
      default: return 'text-zinc-400 bg-zinc-700/50 border-zinc-600/40';
    }
  };
  const getIcon = (t: string) => {
    switch (t) {
      case 'request': return '→';
      case 'response': return '←';
      case 'forward': return '↗';
      case 'synthesis': return '⊕';
      default: return '•';
    }
  };

  return (
    <button onClick={() => onSelect(step)} className="w-full text-left">
      <div className={`rounded-lg px-3 py-2 border transition-colors hover:brightness-110 cursor-pointer ${
        step.step_type === 'request'
          ? 'bg-blue-950/40 border-blue-800/40'
          : step.step_type === 'forward'
          ? 'bg-amber-950/40 border-amber-800/40'
          : step.step_type === 'synthesis'
          ? 'bg-purple-950/40 border-purple-800/40'
          : 'bg-emerald-950/40 border-emerald-800/40'
      }`}>
        <div className="flex items-center gap-2 mb-1">
          <span className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md border text-[10px] font-semibold uppercase tracking-wide ${getColor(step.step_type)}`}>
            {getIcon(step.step_type)} {step.step_type}
          </span>
          <span className="text-xs text-zinc-400">
            <span className="text-zinc-300 font-medium">{step.from}</span>
            <span className="text-zinc-500 mx-1">&rarr;</span>
            <span className="text-zinc-300 font-medium">{step.to}</span>
          </span>
        </div>
        <div className="text-xs text-zinc-400 whitespace-pre-wrap break-words max-h-24 overflow-hidden leading-relaxed">
          {step.content.length > 200 ? `${step.content.slice(0, 200)}...` : step.content}
        </div>
      </div>
    </button>
  );
});

const VerboseMessage = memo(function VerboseMessage({
  msg,
  onSelectTrace,
}: {
  msg: ChatMessage;
  onSelectTrace: (step: AgentTraceStep) => void;
}) {
  if (msg.role === 'system') {
    return (
      <div className="flex justify-start">
        <div className="max-w-[85%] rounded-lg px-3 py-2 bg-zinc-700 text-zinc-300 text-sm italic">
          <div className="chat-markdown">
            <ReactMarkdown>{msg.content}</ReactMarkdown>
          </div>
        </div>
      </div>
    );
  }

  if (msg.role === 'user') {
    return (
      <div className="flex justify-end">
        <div className="max-w-[85%] rounded-lg px-3 py-2 bg-blue-600 text-white">
          <div className="chat-markdown">
            <ReactMarkdown>{msg.content}</ReactMarkdown>
          </div>
        </div>
      </div>
    );
  }

  // Assistant message
  return (
    <>
      {msg.trace && msg.trace.length > 0 && (
        <div className="space-y-1.5 ml-1">
          {msg.trace.map((step, idx) => {
            const isFinalResponse = step.step_type === 'response' && step.to === 'User' && idx === msg.trace!.length - 1;
            if (isFinalResponse && !msg.streaming) return null;
            return <VerboseTraceStep key={idx} step={step} onSelect={onSelectTrace} />;
          })}
        </div>
      )}
      {msg.streaming ? (
        <div className="flex justify-start">
          <div className="max-w-[85%] rounded-lg px-3 py-2 bg-zinc-800 text-zinc-100 border border-zinc-700 border-dashed">
            <div className="flex items-center gap-2 text-sm text-zinc-400">
              <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
              </svg>
              <span>Awaiting final response...</span>
            </div>
          </div>
        </div>
      ) : (
        <div className="flex justify-start">
          <div className="max-w-[85%] rounded-lg px-3 py-2 bg-zinc-800 text-zinc-100 border border-zinc-700">
            {msg.agent && (
              <div className="text-xs text-zinc-400 mb-1 font-medium flex items-center gap-1.5">
                <span className="w-1.5 h-1.5 rounded-full bg-green-500" />
                {msg.agent} &mdash; final response
              </div>
            )}
            <div className="chat-markdown">
              <ReactMarkdown>{msg.content}</ReactMarkdown>
            </div>
          </div>
        </div>
      )}
    </>
  );
});

// ── Main component ───────────────────────────────────────────────────

export default function ChatPanel({ isOpen, onClose, config, systemName }: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<'idle' | 'registering' | 'creating_session' | 'ready' | 'error'>('idle');
  const [verboseMode, setVerboseMode] = useState(false);
  const [selectedTraceStep, setSelectedTraceStep] = useState<AgentTraceStep | null>(null);

  // Session list state
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [sessionsLoading, setSessionsLoading] = useState(false);
  const [currentView, setCurrentView] = useState<'sessions' | 'chat'>('sessions');
  const [deletingSessionId, setDeletingSessionId] = useState<string | null>(null);
  const [deletingAll, setDeletingAll] = useState(false);

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

  // Fetch sessions for this system
  const fetchSessions = useCallback(async () => {
    setSessionsLoading(true);
    try {
      const res = await authFetch(`${API_BASE}/sessions?system_name=${encodeURIComponent(systemName)}`);
      if (!res.ok) {
        const errorData = await res.json().catch(() => ({}));
        throw new Error(extractErrorMessage(errorData, 'Failed to fetch sessions'));
      }
      const data = await res.json();
      // Sort by last_activity or created_at descending
      const sorted = (data.sessions || []).sort((a: SessionSummary, b: SessionSummary) => {
        const aDate = a.last_activity || a.created_at;
        const bDate = b.last_activity || b.created_at;
        return new Date(bDate).getTime() - new Date(aDate).getTime();
      });
      setSessions(sorted);
    } catch (err) {
      console.error('Failed to fetch sessions:', err);
      // Don't show error in UI, just show empty list
      setSessions([]);
    } finally {
      setSessionsLoading(false);
    }
  }, [systemName]);

  // Load an existing session's history
  const loadSession = useCallback(async (id: string) => {
    setError(null);
    setStatus('creating_session');
    setCurrentView('chat');

    try {
      const res = await authFetch(`${API_BASE}/sessions/${id}/history`);
      if (!res.ok) {
        const errorData = await res.json().catch(() => ({}));
        throw new Error(extractErrorMessage(errorData, 'Failed to load session'));
      }

      const data = await res.json();
      setSessionId(id);

      // Convert MessageResponse[] to ChatMessage[]
      const chatMessages: ChatMessage[] = (data.messages || []).map((msg: MessageResponse) => ({
        id: msg.id,
        role: msg.from === 'user' ? 'user' : 'assistant' as const,
        content: msg.from === 'user' ? msg.content : unwrapJsonResponse(msg.content),
        agent: msg.from !== 'user' ? msg.from : undefined,
        timestamp: new Date(msg.timestamp),
      }));

      // Add welcome message at the start if we have history
      if (chatMessages.length > 0) {
        setMessages([
          {
            id: 'welcome',
            role: 'system',
            content: `Resumed session with "${systemName}". ${chatMessages.length} messages loaded.`,
            timestamp: new Date(),
          },
          ...chatMessages,
        ]);
      } else {
        setMessages([{
          id: 'welcome',
          role: 'system',
          content: `Connected to "${systemName}". This session has no messages yet.`,
          timestamp: new Date(),
        }]);
      }

      setStatus('ready');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load session';
      setError(message);
      setStatus('error');
      setCurrentView('sessions');
    }
  }, [systemName]);

  // Delete a session
  const deleteSession = useCallback(async (id: string) => {
    setDeletingSessionId(id);
    try {
      const res = await authFetch(`${API_BASE}/sessions/${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) {
        const errorData = await res.json().catch(() => ({}));
        throw new Error(extractErrorMessage(errorData, 'Failed to delete session'));
      }

      // Remove from local state
      setSessions((prev) => prev.filter((s) => s.id !== id));

      // If we deleted the currently active session, reset
      if (sessionId === id) {
        setSessionId(null);
        setMessages([]);
        setStatus('idle');
        setCurrentView('sessions');
      }
    } catch (err) {
      console.error('Failed to delete session:', err);
      setError(err instanceof Error ? err.message : 'Failed to delete session');
    } finally {
      setDeletingSessionId(null);
    }
  }, [sessionId]);

  // Delete all sessions for this system
  const deleteAllSessions = useCallback(async () => {
    if (!confirm(`Delete all ${sessions.length} session(s) for "${systemName}"?`)) return;
    setDeletingAll(true);
    try {
      await Promise.all(
        sessions.map((s) =>
          authFetch(`${API_BASE}/sessions/${s.id}`, { method: 'DELETE' })
        )
      );
      setSessions([]);
      // If we were in an active session, reset
      if (sessionId) {
        setSessionId(null);
        setMessages([]);
        setStatus('idle');
        setCurrentView('sessions');
      }
    } catch (err) {
      console.error('Failed to delete all sessions:', err);
      setError(err instanceof Error ? err.message : 'Failed to delete sessions');
      // Refresh the list to show what's left
      fetchSessions();
    } finally {
      setDeletingAll(false);
    }
  }, [sessions, systemName, sessionId, fetchSessions]);

  // Initialize system and create a NEW session
  const initializeSession = useCallback(async () => {
    if (status === 'registering' || status === 'creating_session') return;

    setError(null);
    setStatus('registering');
    setCurrentView('chat');

    try {
      // First, try to register the system (or update if it exists)
      const registerRes = await authFetch(`${API_BASE}/systems`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: systemName, config }),
      });

      if (!registerRes.ok) {
        const errorData = await registerRes.json().catch(() => ({}));
        // If system already exists, try to update it
        if (registerRes.status === 409) {
          const updateRes = await authFetch(`${API_BASE}/systems/${encodeURIComponent(systemName)}`, {
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
      const sessionRes = await authFetch(`${API_BASE}/sessions`, {
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
  }, [config, systemName, status]);

  // Fetch sessions when panel opens
  useEffect(() => {
    if (isOpen && currentView === 'sessions') {
      fetchSessions();
    }
  }, [isOpen, currentView, fetchSessions]);

  const sendMessage = async (contentOverride?: string) => {
    const content = contentOverride || input.trim();
    if (!content || !sessionId || isLoading) return;

    const userMessage: ChatMessage = {
      id: `user-${Date.now()}`,
      role: 'user',
      content,
      timestamp: new Date(),
    };

    const streamingMsgId = `assistant-${Date.now()}`;

    setMessages((prev) => [...prev, userMessage]);
    setInput('');
    setIsLoading(true);
    setError(null);

    // Add a streaming placeholder message
    setMessages((prev) => [
      ...prev,
      {
        id: streamingMsgId,
        role: 'assistant',
        content: '',
        timestamp: new Date(),
        trace: [],
        streaming: true,
      },
    ]);

    try {
      const res = await authFetch(`${API_BASE}/sessions/${sessionId}/prompt/stream`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          content,
          include_context: true,
          context_limit: 5,
        }),
      });

      if (!res.ok) {
        const errorData = await res.json().catch(() => ({}));
        throw new Error(errorData.error || 'Failed to send message');
      }

      const reader = res.body?.getReader();
      if (!reader) throw new Error('No response body');

      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });

        // Parse SSE lines: each event ends with \n\n
        const parts = buffer.split('\n\n');
        buffer = parts.pop() || ''; // Keep incomplete part in buffer

        for (const part of parts) {
          if (!part.trim()) continue;

          let eventType = 'message';
          let data = '';

          for (const line of part.split('\n')) {
            if (line.startsWith('event:')) {
              eventType = line.slice(6).trim();
            } else if (line.startsWith('data:')) {
              data += line.slice(5).trim();
            } else if (line.startsWith(':')) {
              // SSE comment (keepalive), ignore
            }
          }

          if (!data) continue;

          if (eventType === 'trace') {
            try {
              const traceStep: AgentTraceStep = JSON.parse(data);
              // Append trace step to the streaming message
              setMessages((prev) =>
                prev.map((msg) =>
                  msg.id === streamingMsgId
                    ? { ...msg, trace: [...(msg.trace || []), traceStep] }
                    : msg
                )
              );
            } catch {
              // Skip malformed trace events
            }
          } else if (eventType === 'complete') {
            try {
              const response: SessionPromptResponse = JSON.parse(data);

              let content: string;
              let agentName: string | undefined;

              if (response.result.type === 'response') {
                content = unwrapJsonResponse(response.result.content);
                agentName = response.result.from;
              } else if (response.result.type === 'timeout') {
                content = `Request timed out: ${response.result.message}`;
              } else {
                content = 'Message sent (no response expected)';
              }

              // Finalize the streaming message
              setMessages((prev) =>
                prev.map((msg) =>
                  msg.id === streamingMsgId
                    ? {
                        ...msg,
                        content,
                        agent: agentName,
                        trace: response.trace && response.trace.length > 0 ? response.trace : msg.trace,
                        streaming: false,
                      }
                    : msg
                )
              );
            } catch {
              // Skip malformed complete events
            }
          } else if (eventType === 'error') {
            try {
              const errData = JSON.parse(data);
              throw new Error(errData.error || 'Unknown streaming error');
            } catch (parseErr) {
              if (parseErr instanceof Error && parseErr.message !== 'Unknown streaming error') {
                throw new Error(data);
              }
              throw parseErr;
            }
          }
        }
      }

      // If the message is still streaming (no complete event received), mark it as done
      setMessages((prev) =>
        prev.map((msg) =>
          msg.id === streamingMsgId && msg.streaming
            ? { ...msg, content: msg.content || 'No response received', streaming: false }
            : msg
        )
      );
    } catch (err) {
      // Remove the streaming placeholder and show error
      setMessages((prev) =>
        prev
          .filter((msg) => msg.id !== streamingMsgId || (msg.trace && msg.trace.length > 0))
          .map((msg) =>
            msg.id === streamingMsgId
              ? { ...msg, content: `Error: ${err instanceof Error ? err.message : 'Failed'}`, streaming: false }
              : msg
          )
      );

      setError(err instanceof Error ? err.message : 'Failed to send message');
      setMessages((prev) => [
        ...prev,
        {
          id: `error-${Date.now()}`,
          role: 'system',
          content: `Error: ${err instanceof Error ? err.message : 'Failed to send message'}`,
          timestamp: new Date(),
        },
      ]);
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
    setCurrentView('sessions');
  };

  // Format date for session list
  const formatSessionDate = (dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;

    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-y-0 right-0 w-[450px] bg-zinc-900 border-l border-zinc-700 shadow-2xl flex flex-col z-50">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-700 bg-zinc-800">
        <div className="flex items-center gap-3">
          {currentView === 'chat' && (
            <button
              onClick={() => setCurrentView('sessions')}
              className="p-1.5 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-700 rounded transition-colors"
              title="Back to sessions"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
              </svg>
            </button>
          )}
          {currentView === 'sessions' ? (
            <div>
              <h2 className="font-semibold text-zinc-100">Sessions</h2>
              <p className="text-xs text-zinc-400">{systemName}</p>
            </div>
          ) : (
            <>
              <div className={`w-2 h-2 rounded-full ${status === 'ready' ? 'bg-green-500' : status === 'error' ? 'bg-red-500' : 'bg-yellow-500 animate-pulse'}`} />
              <div>
                <h2 className="font-semibold text-zinc-100">Chat</h2>
                <p className="text-xs text-zinc-400">{systemName}</p>
              </div>
            </>
          )}
        </div>
        <div className="flex items-center gap-2">
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

      {/* Mode switcher - only in chat view */}
      {currentView === 'chat' && (
        <div className="px-4 py-2 border-b border-zinc-700 bg-zinc-850 flex items-center gap-2">
          <div className="flex bg-zinc-800 rounded-lg p-0.5 flex-1">
            <button
              onClick={() => setVerboseMode(false)}
              className={`flex-1 px-3 py-1.5 text-xs font-medium rounded-md transition-colors flex items-center justify-center gap-1.5 ${
                !verboseMode
                  ? 'bg-zinc-600 text-zinc-100 shadow-sm'
                  : 'text-zinc-400 hover:text-zinc-300'
              }`}
            >
              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
              </svg>
              Normal
            </button>
            <button
              onClick={() => setVerboseMode(true)}
              className={`flex-1 px-3 py-1.5 text-xs font-medium rounded-md transition-colors flex items-center justify-center gap-1.5 ${
                verboseMode
                  ? 'bg-amber-900/60 text-amber-300 shadow-sm'
                  : 'text-zinc-400 hover:text-zinc-300'
              }`}
            >
              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
              Verbose
            </button>
          </div>
        </div>
      )}

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

      {/* Sessions List View */}
      {currentView === 'sessions' && (
        <div className="flex-1 overflow-y-auto">
          {/* New Session + Delete All Buttons */}
          <div className="p-4 border-b border-zinc-700 flex gap-2">
            <button
              onClick={initializeSession}
              className="flex-1 px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors flex items-center justify-center gap-2"
            >
              <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
              </svg>
              New Session
            </button>
            {sessions.length > 0 && (
              <button
                onClick={deleteAllSessions}
                disabled={deletingAll}
                className="px-3 py-3 text-red-400 hover:text-red-300 hover:bg-red-900/20 border border-zinc-700 hover:border-red-800/50 rounded-lg transition-colors disabled:opacity-50 flex items-center justify-center"
                title="Delete all sessions"
              >
                {deletingAll ? (
                  <svg className="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                  </svg>
                ) : (
                  <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                )}
              </button>
            )}
          </div>

          {/* Sessions List */}
          <div className="p-4 space-y-2">
            {sessionsLoading ? (
              <div className="flex items-center justify-center py-8">
                <div className="flex items-center gap-2 text-zinc-400">
                  <svg className="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                  </svg>
                  <span>Loading sessions...</span>
                </div>
              </div>
            ) : sessions.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-12 text-center">
                <div className="w-12 h-12 rounded-full bg-zinc-800 flex items-center justify-center mb-4">
                  <svg className="w-6 h-6 text-zinc-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                  </svg>
                </div>
                <h3 className="text-zinc-300 font-medium mb-1">No sessions yet</h3>
                <p className="text-zinc-500 text-sm">Start a new session to chat with your agents</p>
              </div>
            ) : (
              sessions.map((session) => (
                <div
                  key={session.id}
                  className="group bg-zinc-800 hover:bg-zinc-750 border border-zinc-700 rounded-lg p-3 cursor-pointer transition-colors"
                  onClick={() => loadSession(session.id)}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-zinc-100 truncate">
                          Session
                        </span>
                        <span className="text-xs text-zinc-500 truncate">
                          {session.id.slice(0, 8)}...
                        </span>
                      </div>
                      <div className="flex items-center gap-3 mt-1">
                        <span className="text-xs text-zinc-400">
                          {session.message_count} message{session.message_count !== 1 ? 's' : ''}
                        </span>
                        <span className="text-xs text-zinc-500">
                          {formatSessionDate(session.last_activity || session.created_at)}
                        </span>
                      </div>
                    </div>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteSession(session.id);
                      }}
                      disabled={deletingSessionId === session.id}
                      className="p-1.5 text-zinc-500 hover:text-red-400 hover:bg-red-900/20 rounded opacity-0 group-hover:opacity-100 transition-all disabled:opacity-50"
                      title="Delete session"
                    >
                      {deletingSessionId === session.id ? (
                        <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                        </svg>
                      ) : (
                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                        </svg>
                      )}
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      )}

      {/* Chat View - Messages */}
      {currentView === 'chat' && (
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

        {/* ===== Welcome Message (shown when no messages yet) ===== */}
        {status === 'ready' && messages.length === 0 && (
          <div className="flex flex-col items-center justify-center py-8 px-4 max-w-lg mx-auto">
            <div className="w-12 h-12 rounded-xl bg-blue-500/15 flex items-center justify-center mb-4 border border-blue-500/20">
              <svg className="w-6 h-6 text-blue-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
              </svg>
            </div>
            <h3 className="text-zinc-100 font-semibold text-base mb-1">{systemName}</h3>
            <p className="text-zinc-400 text-sm text-center mb-6">
              {config.agents.length > 0
                ? `${config.agents.length} agent${config.agents.length !== 1 ? 's' : ''} ready — ${config.agents.map(a => a.name).join(', ')}`
                : 'Start a conversation below'}
            </p>
            <div className="w-full space-y-2">
              <p className="text-zinc-500 text-xs font-medium uppercase tracking-wide mb-2">Suggested prompts</p>
              {[
                'What can you help me with?',
                'Walk me through how you work',
                'I have a question — can you ask me for details first?',
              ].map((starter) => (
                <button
                  key={starter}
                  onClick={() => sendMessage(starter)}
                  disabled={isLoading}
                  className="w-full text-left px-3 py-2.5 text-sm text-zinc-300 bg-zinc-800/60 hover:bg-zinc-700/60 border border-zinc-700/50 hover:border-zinc-600 rounded-lg transition-colors disabled:opacity-50"
                >
                  {starter}
                </button>
              ))}
            </div>
          </div>
        )}

        {/* ===== Normal Mode ===== */}
        {!verboseMode && messages.map((msg) => (
          <div key={msg.id}>
            <NormalMessage msg={msg} />
          </div>
        ))}

        {/* ===== Verbose Mode ===== */}
        {verboseMode && messages.map((msg) => (
          <div key={msg.id} className="space-y-2">
            <VerboseMessage msg={msg} onSelectTrace={setSelectedTraceStep} />
          </div>
        ))}

        {isLoading && !messages.some((m) => m.streaming) && (
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
      )}

      {/* Input - only show in chat view */}
      {currentView === 'chat' && (
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
              onClick={() => sendMessage()}
              disabled={!input.trim() || status !== 'ready' || isLoading}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
              </svg>
            </button>
          </div>
        </div>
      )}

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
                <span className={`inline-flex items-center gap-1 px-2 py-1 rounded-md border text-xs font-semibold uppercase tracking-wide ${getStepTypeColor(selectedTraceStep.step_type)}`}>
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
