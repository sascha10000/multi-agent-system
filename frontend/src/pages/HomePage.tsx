import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api } from '../api/client';
import type { ListSystemsResponse } from '../types/api';

function HomePage() {
  const [data, setData] = useState<ListSystemsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .listSystems()
      .then(setData)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  const totalAgents = data?.systems.reduce((sum, s) => sum + s.agent_count, 0) ?? 0;

  return (
    <div className="page">
      <h1>Dashboard</h1>

      {loading && <div className="loading">Loading...</div>}
      {error && <div className="error">Error: {error}</div>}

      {data && (
        <>
          <div className="stats-grid">
            <div className="stat-card">
              <div className="stat-value">{data.total}</div>
              <div className="stat-label">Active Systems</div>
            </div>
            <div className="stat-card">
              <div className="stat-value">{totalAgents}</div>
              <div className="stat-label">Total Agents</div>
            </div>
          </div>

          <div className="dashboard-section">
            <div className="section-header">
              <h2>Recent Systems</h2>
              <Link to="/systems" className="btn btn-small">
                View All
              </Link>
            </div>

            {data.systems.length === 0 ? (
              <div className="empty-state">
                <p>No systems registered yet.</p>
                <Link to="/systems" className="btn btn-primary">
                  Create Your First System
                </Link>
              </div>
            ) : (
              <div className="systems-list-mini">
                {data.systems.slice(0, 3).map((system) => (
                  <Link
                    key={system.name}
                    to={`/systems/${system.name}`}
                    className="system-list-item"
                  >
                    <span className="system-list-name">{system.name}</span>
                    <span className="system-list-agents">
                      {system.agent_count} agent{system.agent_count !== 1 ? 's' : ''}
                    </span>
                  </Link>
                ))}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

export default HomePage;
