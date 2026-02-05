import type { ConnectionConfig } from '../types/api';

interface ConnectionEditorProps {
  connections: Record<string, ConnectionConfig>;
  availableTargets: string[];
  onChange: (connections: Record<string, ConnectionConfig>) => void;
}

function ConnectionEditor({ connections, availableTargets, onChange }: ConnectionEditorProps) {
  const connectionEntries = Object.entries(connections);

  const handleAddConnection = () => {
    // Find first available target not already connected
    const newTarget = availableTargets.find(
      (target) => !connections[target]
    );
    if (newTarget) {
      onChange({
        ...connections,
        [newTarget]: { type: 'blocking', timeout_secs: 60 },
      });
    }
  };

  const handleRemoveConnection = (target: string) => {
    const newConnections = { ...connections };
    delete newConnections[target];
    onChange(newConnections);
  };

  const handleTargetChange = (oldTarget: string, newTarget: string) => {
    if (oldTarget === newTarget) return;
    const config = connections[oldTarget];
    const newConnections = { ...connections };
    delete newConnections[oldTarget];
    newConnections[newTarget] = config;
    onChange(newConnections);
  };

  const handleTypeChange = (target: string, type: 'blocking' | 'notify') => {
    const config = { ...connections[target], type };
    if (type === 'notify') {
      delete config.timeout_secs;
    } else if (!config.timeout_secs) {
      config.timeout_secs = 60;
    }
    onChange({ ...connections, [target]: config });
  };

  const handleTimeoutChange = (target: string, timeout: number) => {
    onChange({
      ...connections,
      [target]: { ...connections[target], timeout_secs: timeout },
    });
  };

  const usedTargets = Object.keys(connections);
  const hasAvailableTargets = availableTargets.some((t) => !usedTargets.includes(t));

  return (
    <div className="connection-editor">
      {connectionEntries.length === 0 ? (
        <div className="connection-editor-empty">No connections defined</div>
      ) : (
        <div className="connection-editor-list">
          {connectionEntries.map(([target, config]) => (
            <div key={target} className="connection-editor-item">
              <span className="connection-editor-arrow">→</span>

              <select
                value={target}
                onChange={(e) => handleTargetChange(target, e.target.value)}
                className="connection-editor-target"
              >
                <option value={target}>{target}</option>
                {availableTargets
                  .filter((t) => t !== target && !connections[t])
                  .map((t) => (
                    <option key={t} value={t}>
                      {t}
                    </option>
                  ))}
              </select>

              <select
                value={config.type}
                onChange={(e) =>
                  handleTypeChange(target, e.target.value as 'blocking' | 'notify')
                }
                className="connection-editor-type"
              >
                <option value="blocking">blocking</option>
                <option value="notify">notify</option>
              </select>

              {config.type === 'blocking' && (
                <div className="connection-editor-timeout">
                  <span>timeout:</span>
                  <input
                    type="number"
                    value={config.timeout_secs ?? 60}
                    onChange={(e) =>
                      handleTimeoutChange(target, parseInt(e.target.value) || 60)
                    }
                    min={1}
                    className="connection-editor-timeout-input"
                  />
                  <span>s</span>
                </div>
              )}

              <button
                type="button"
                onClick={() => handleRemoveConnection(target)}
                className="connection-editor-remove"
                title="Remove connection"
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}

      {hasAvailableTargets && (
        <button
          type="button"
          onClick={handleAddConnection}
          className="btn btn-small connection-editor-add"
        >
          + Add Connection
        </button>
      )}

      {!hasAvailableTargets && connectionEntries.length === 0 && (
        <div className="connection-editor-hint">
          Add more agents to create connections
        </div>
      )}
    </div>
  );
}

export default ConnectionEditor;
