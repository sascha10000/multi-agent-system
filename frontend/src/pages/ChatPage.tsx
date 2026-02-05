import { useState, useEffect, useRef } from 'react';
import { useParams, Link } from 'react-router-dom';
import { api } from '../api/client';
import type { SystemDetailResponse, SendPromptResponse } from '../types/api';
import ChatMessage from '../components/ChatMessage';

interface Message {
  id: string;
  role: 'user' | 'agent' | 'system';
  content: string;
  agentName?: string;
  elapsedMs?: number;
}

function ChatPage() {
  const { name } = useParams<{ name: string }>();
  const [system, setSystem] = useState<SystemDetailResponse | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [targetAgent, setTargetAgent] = useState<string>('');
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!name) return;

    api
      .getSystem(name)
      .then((data) => {
        setSystem(data);
        // Default to Coordinator or first routing agent
        const defaultAgent =
          data.agents.find((a) => a.name === 'Coordinator') ||
          data.agents.find((a) => a.routing) ||
          data.agents[0];
        if (defaultAgent) {
          setTargetAgent(defaultAgent.name);
        }
      })
      .catch((e) => setError(e.message));
  }, [name]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const sendMessage = async () => {
    if (!input.trim() || !name || sending) return;

    const userMessage: Message = {
      id: crypto.randomUUID(),
      role: 'user',
      content: input.trim(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput('');
    setSending(true);
    setError(null);

    try {
      const response: SendPromptResponse = await api.sendPrompt(name, {
        content: userMessage.content,
        target_agent: targetAgent || undefined,
      });

      let agentMessage: Message;

      if (response.result.type === 'response') {
        agentMessage = {
          id: response.message_id,
          role: 'agent',
          content: response.result.content,
          agentName: response.result.from,
          elapsedMs: response.elapsed_ms,
        };
      } else if (response.result.type === 'timeout') {
        agentMessage = {
          id: response.message_id,
          role: 'system',
          content: `Timeout: ${response.result.message}`,
          elapsedMs: response.elapsed_ms,
        };
      } else {
        agentMessage = {
          id: response.message_id,
          role: 'system',
          content: 'Message sent (notify - no response expected)',
          elapsedMs: response.elapsed_ms,
        };
      }

      setMessages((prev) => [...prev, agentMessage]);
    } catch (e) {
      const errorMessage: Message = {
        id: crypto.randomUUID(),
        role: 'system',
        content: `Error: ${e instanceof Error ? e.message : 'Unknown error'}`,
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      setSending(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  if (error && !system) {
    return (
      <div className="page">
        <div className="error">Error: {error}</div>
        <Link to="/systems" className="btn">
          Back to Systems
        </Link>
      </div>
    );
  }

  return (
    <div className="page page-full-height">
      <div className="page-header">
        <div className="page-header-left">
          <Link to={`/systems/${name}`} className="btn btn-small">
            &larr; Back
          </Link>
          <h1>Chat: {name}</h1>
        </div>
      </div>

      <div className="chat-container">
        <div className="chat-messages">
          {messages.length === 0 && (
            <div className="chat-empty">
              <p>No messages yet. Send a prompt to start a conversation.</p>
            </div>
          )}
          {messages.map((msg) => (
            <ChatMessage
              key={msg.id}
              role={msg.role}
              content={msg.content}
              agentName={msg.agentName}
              elapsedMs={msg.elapsedMs}
            />
          ))}
          <div ref={messagesEndRef} />
        </div>

        <div className="chat-input-area">
          {system && (
            <div className="chat-target-select">
              <label htmlFor="target-agent">Target Agent:</label>
              <select
                id="target-agent"
                value={targetAgent}
                onChange={(e) => setTargetAgent(e.target.value)}
              >
                {system.agents.map((agent) => (
                  <option key={agent.name} value={agent.name}>
                    {agent.name} {agent.routing ? '(LLM)' : ''}
                  </option>
                ))}
              </select>
            </div>
          )}
          <div className="chat-input-row">
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Type your message... (Enter to send)"
              className="chat-input"
              disabled={sending}
            />
            <button
              onClick={sendMessage}
              disabled={sending || !input.trim()}
              className="btn btn-primary chat-send-btn"
            >
              {sending ? 'Sending...' : 'Send'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default ChatPage;
