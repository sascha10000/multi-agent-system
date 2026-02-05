import type { AgentConfigJson } from '../types/api';
import ConnectionEditor from './ConnectionEditor';

interface AgentFormProps {
  agent: AgentConfigJson;
  availableAgents: string[];
  providers: string[];
  onChange: (agent: AgentConfigJson) => void;
  onDelete: () => void;
  isExpanded: boolean;
  onToggleExpand: () => void;
}

function AgentForm({
  agent,
  availableAgents,
  providers,
  onChange,
  onDelete,
  isExpanded,
  onToggleExpand,
}: AgentFormProps) {
  // Available connection targets: all agents except this one
  const connectionTargets = availableAgents.filter((name) => name !== agent.name);

  const updateField = <K extends keyof AgentConfigJson>(
    field: K,
    value: AgentConfigJson[K]
  ) => {
    onChange({ ...agent, [field]: value });
  };

  const updateHandler = <K extends keyof AgentConfigJson['handler']>(
    field: K,
    value: AgentConfigJson['handler'][K]
  ) => {
    onChange({
      ...agent,
      handler: { ...agent.handler, [field]: value },
    });
  };

  const updateOptions = (
    field: 'temperature' | 'max_tokens',
    value: number | undefined
  ) => {
    const options = { ...agent.handler.options };
    if (value === undefined) {
      delete options[field];
    } else {
      options[field] = value;
    }
    onChange({
      ...agent,
      handler: { ...agent.handler, options },
    });
  };

  return (
    <div className={`agent-form ${isExpanded ? 'agent-form-expanded' : ''}`}>
      <div className="agent-form-header" onClick={onToggleExpand}>
        <div className="agent-form-header-left">
          <span className="agent-form-expand-icon">{isExpanded ? '▼' : '▶'}</span>
          <span className="agent-form-name">
            {agent.name || 'New Agent'}
          </span>
          {agent.handler.routing && (
            <span className="agent-form-routing-badge">LLM Routing</span>
          )}
        </div>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          className="agent-form-delete"
          title="Delete agent"
        >
          Delete
        </button>
      </div>

      {isExpanded && (
        <div className="agent-form-body">
          <div className="agent-form-row">
            <div className="agent-form-field">
              <label>Name</label>
              <input
                type="text"
                value={agent.name}
                onChange={(e) => updateField('name', e.target.value)}
                placeholder="AgentName"
                className="agent-form-input"
              />
            </div>
            <div className="agent-form-field">
              <label>Role</label>
              <input
                type="text"
                value={agent.role}
                onChange={(e) => updateField('role', e.target.value)}
                placeholder="What this agent does"
                className="agent-form-input"
              />
            </div>
          </div>

          <div className="agent-form-field">
            <label>System Prompt</label>
            <textarea
              value={agent.system_prompt}
              onChange={(e) => updateField('system_prompt', e.target.value)}
              placeholder="Instructions for the LLM..."
              className="agent-form-textarea"
              rows={4}
            />
          </div>

          <div className="agent-form-section">
            <h4>Handler Settings</h4>
            <div className="agent-form-row">
              <div className="agent-form-field">
                <label>Provider</label>
                <select
                  value={agent.handler.provider}
                  onChange={(e) => updateHandler('provider', e.target.value)}
                  className="agent-form-select"
                >
                  {providers.map((p) => (
                    <option key={p} value={p}>
                      {p}
                    </option>
                  ))}
                </select>
              </div>
              <div className="agent-form-field">
                <label>Model</label>
                <input
                  type="text"
                  value={agent.handler.model ?? ''}
                  onChange={(e) =>
                    updateHandler('model', e.target.value || undefined)
                  }
                  placeholder="e.g., gemma3:4b"
                  className="agent-form-input"
                />
              </div>
            </div>

            <div className="agent-form-checkbox-row">
              <label className="agent-form-checkbox-label">
                <input
                  type="checkbox"
                  checked={agent.handler.routing}
                  onChange={(e) => updateHandler('routing', e.target.checked)}
                />
                <span>Enable LLM Routing</span>
              </label>
              <span className="agent-form-checkbox-hint">
                Allow LLM to dynamically route messages to connected agents
              </span>
            </div>
          </div>

          <div className="agent-form-section">
            <h4>Options</h4>
            <div className="agent-form-row">
              <div className="agent-form-field">
                <label>
                  Temperature: {agent.handler.options?.temperature ?? 0.7}
                </label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.1"
                  value={agent.handler.options?.temperature ?? 0.7}
                  onChange={(e) =>
                    updateOptions('temperature', parseFloat(e.target.value))
                  }
                  className="agent-form-slider"
                />
              </div>
              <div className="agent-form-field">
                <label>Max Tokens</label>
                <input
                  type="number"
                  value={agent.handler.options?.max_tokens ?? ''}
                  onChange={(e) =>
                    updateOptions(
                      'max_tokens',
                      e.target.value ? parseInt(e.target.value) : undefined
                    )
                  }
                  placeholder="500"
                  min={1}
                  className="agent-form-input"
                />
              </div>
            </div>
          </div>

          <div className="agent-form-section">
            <h4>Connections</h4>
            <ConnectionEditor
              connections={agent.connections}
              availableTargets={connectionTargets}
              onChange={(connections) => updateField('connections', connections)}
            />
          </div>
        </div>
      )}
    </div>
  );
}

export default AgentForm;
