import { useEffect, useState } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import { ReactFlowProvider } from '@xyflow/react';
import { api } from '../api/client';
import type { SystemDetailResponse } from '../types/api';
import TopologyGraph from '../components/TopologyGraph';

function SystemDetailPage() {
  const { name } = useParams<{ name: string }>();
  const navigate = useNavigate();
  const [data, setData] = useState<SystemDetailResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!name) return;

    api
      .getSystem(name)
      .then(setData)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, [name]);

  const handleDelete = async () => {
    if (!name || !confirm(`Delete system "${name}"?`)) return;

    try {
      await api.deleteSystem(name);
      navigate('/systems');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete');
    }
  };

  if (loading) {
    return (
      <div className="page">
        <div className="loading">Loading system...</div>
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="page">
        <div className="error">Error: {error || 'System not found'}</div>
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
          <Link to="/systems" className="btn btn-small">
            &larr; Back
          </Link>
          <h1>{data.name}</h1>
        </div>
        <div className="page-header-right">
          <Link to={`/systems/${name}/chat`} className="btn btn-primary">
            Chat
          </Link>
          <button onClick={handleDelete} className="btn btn-danger">
            Delete
          </button>
        </div>
      </div>

      <div className="system-detail-layout">
        <div className="system-detail-sidebar">
          <div className="sidebar-section">
            <h3>Overview</h3>
            <dl className="info-list">
              <dt>Agents</dt>
              <dd>{data.agent_count}</dd>
              <dt>Timeout</dt>
              <dd>{data.global_timeout_secs}s</dd>
              <dt>Created</dt>
              <dd>{new Date(data.created_at).toLocaleString()}</dd>
            </dl>
          </div>

          <div className="sidebar-section">
            <h3>Agents</h3>
            <ul className="agent-list">
              {data.agents.map((agent) => (
                <li key={agent.name} className="agent-list-item">
                  <div className="agent-list-header">
                    <span className="agent-list-name">{agent.name}</span>
                    {agent.routing && (
                      <span className="badge badge-primary">LLM Routing</span>
                    )}
                  </div>
                  <div className="agent-list-role">{agent.role}</div>
                  {agent.connections.length > 0 && (
                    <div className="agent-list-connections">
                      {agent.connections.map((conn) => (
                        <span
                          key={conn.target}
                          className={`connection-tag connection-${conn.connection_type}`}
                        >
                          {conn.connection_type} &rarr; {conn.target}
                        </span>
                      ))}
                    </div>
                  )}
                </li>
              ))}
            </ul>
          </div>
        </div>

        <div className="system-detail-graph">
          <h3>Topology</h3>
          <ReactFlowProvider>
            <TopologyGraph agents={data.agents} />
          </ReactFlowProvider>
        </div>
      </div>
    </div>
  );
}

export default SystemDetailPage;
