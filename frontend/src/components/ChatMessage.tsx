interface ChatMessageProps {
  role: 'user' | 'agent' | 'system';
  content: string;
  agentName?: string;
  elapsedMs?: number;
}

function ChatMessage({ role, content, agentName, elapsedMs }: ChatMessageProps) {
  return (
    <div className={`chat-message chat-message-${role}`}>
      <div className="chat-message-header">
        <span className="chat-message-sender">
          {role === 'user' ? 'You' : role === 'agent' ? agentName || 'Agent' : 'System'}
        </span>
        {elapsedMs !== undefined && (
          <span className="chat-message-time">{elapsedMs}ms</span>
        )}
      </div>
      <div className="chat-message-content">{content}</div>
    </div>
  );
}

export default ChatMessage;
