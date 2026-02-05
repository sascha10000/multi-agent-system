import { Link } from 'react-router-dom';
import type { SystemSummary } from '../types/api';

interface SystemCardProps {
  system: SystemSummary;
  onDelete?: (name: string) => void;
}

function SystemCard({ system, onDelete }: SystemCardProps) {
  const createdDate = new Date(system.created_at).toLocaleString();

  const handleDelete = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (onDelete && confirm(`Delete system "${system.name}"?`)) {
      onDelete(system.name);
    }
  };

  return (
    <div className="system-card">
      <div className="system-card-header">
        <h3 className="system-card-title">{system.name}</h3>
        <span className="system-card-badge">{system.agent_count} agents</span>
      </div>
      <div className="system-card-agents">
        {system.agents.map((agent) => (
          <span key={agent} className="agent-tag">
            {agent}
          </span>
        ))}
      </div>
      <div className="system-card-footer">
        <span className="system-card-date">Created: {createdDate}</span>
        <div className="system-card-actions">
          <Link to={`/systems/${encodeURIComponent(system.name)}`} className="btn btn-small">
            View
          </Link>
          <Link to={`/systems/${encodeURIComponent(system.name)}/chat`} className="btn btn-small btn-primary">
            Chat
          </Link>
          {onDelete && (
            <button onClick={handleDelete} className="btn btn-small btn-danger">
              Delete
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

export default SystemCard;
