import { useEffect, useState, useCallback } from 'react';
import { api } from '../api/client';
import type { ListSystemsResponse, SystemConfigJson } from '../types/api';
import SystemCard from '../components/SystemCard';
import ConfigEditor from '../components/ConfigEditor';

function SystemsPage() {
  const [data, setData] = useState<ListSystemsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [creating, setCreating] = useState(false);

  const loadSystems = useCallback(() => {
    setLoading(true);
    api
      .listSystems()
      .then(setData)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    loadSystems();
  }, [loadSystems]);

  const handleCreate = async (name: string, config: SystemConfigJson) => {
    setCreating(true);
    setError(null);
    try {
      await api.createSystem({ name, config });
      setShowCreate(false);
      loadSystems();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to create system');
    } finally {
      setCreating(false);
    }
  };

  const handleDelete = async (name: string) => {
    try {
      await api.deleteSystem(name);
      loadSystems();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete system');
    }
  };

  return (
    <div className="page">
      <div className="page-header">
        <h1>Systems</h1>
        <button
          onClick={() => setShowCreate(!showCreate)}
          className="btn btn-primary"
        >
          {showCreate ? 'Cancel' : 'New System'}
        </button>
      </div>

      {error && <div className="error">Error: {error}</div>}

      {showCreate && (
        <ConfigEditor onSubmit={handleCreate} isLoading={creating} />
      )}

      {loading && <div className="loading">Loading...</div>}

      {data && !loading && (
        <>
          {data.systems.length === 0 && !showCreate ? (
            <div className="empty-state">
              <p>No systems registered yet.</p>
              <button onClick={() => setShowCreate(true)} className="btn btn-primary">
                Create Your First System
              </button>
            </div>
          ) : (
            <div className="systems-grid">
              {data.systems.map((system) => (
                <SystemCard
                  key={system.name}
                  system={system}
                  onDelete={handleDelete}
                />
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}

export default SystemsPage;
