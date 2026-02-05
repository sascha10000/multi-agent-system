import { useState, useMemo } from 'react';
import type { SystemConfigJson, AgentConfigJson } from '../types/api';
import AgentForm from './AgentForm';

interface SystemBuilderProps {
  config: SystemConfigJson;
  onChange: (config: SystemConfigJson) => void;
}

const DEFAULT_AGENT: AgentConfigJson = {
  name: '',
  role: '',
  system_prompt: '',
  handler: {
    provider: 'default',
    routing: false,
    options: { temperature: 0.7, max_tokens: 500 },
  },
  connections: {},
};

function SystemBuilder({ config, onChange }: SystemBuilderProps) {
  const [expandedAgent, setExpandedAgent] = useState<string | null>(
    config.agents.length > 0 ? config.agents[0].name : null
  );

  // Derive available agent names for connection dropdowns
  const agentNames = useMemo(
    () => config.agents.map((a) => a.name).filter(Boolean),
    [config.agents]
  );

  // Provider names from llm_providers
  const providerNames = useMemo(
    () => Object.keys(config.llm_providers),
    [config.llm_providers]
  );

  const updateSystemSettings = (timeout: number) => {
    onChange({
      ...config,
      system: { ...config.system, global_timeout_secs: timeout },
    });
  };

  const updateProvider = (
    field: 'base_url' | 'default_model',
    value: string
  ) => {
    const defaultProvider = config.llm_providers.default;
    onChange({
      ...config,
      llm_providers: {
        ...config.llm_providers,
        default: { ...defaultProvider, [field]: value },
      },
    });
  };

  const updateAgent = (index: number, agent: AgentConfigJson) => {
    const newAgents = [...config.agents];
    newAgents[index] = agent;
    onChange({ ...config, agents: newAgents });
  };

  const addAgent = () => {
    // Generate unique name
    let baseName = 'NewAgent';
    let counter = 1;
    let name = baseName;
    while (config.agents.some((a) => a.name === name)) {
      name = `${baseName}${counter}`;
      counter++;
    }

    const newAgent: AgentConfigJson = {
      ...DEFAULT_AGENT,
      name,
      handler: {
        ...DEFAULT_AGENT.handler,
        provider: providerNames[0] || 'default',
      },
    };

    onChange({
      ...config,
      agents: [...config.agents, newAgent],
    });
    setExpandedAgent(name);
  };

  const deleteAgent = (index: number) => {
    const agentName = config.agents[index].name;

    // Remove agent and clean up any connections pointing to it
    const newAgents = config.agents
      .filter((_, i) => i !== index)
      .map((agent) => {
        const newConnections = { ...agent.connections };
        delete newConnections[agentName];
        return { ...agent, connections: newConnections };
      });

    onChange({ ...config, agents: newAgents });

    if (expandedAgent === agentName) {
      setExpandedAgent(newAgents.length > 0 ? newAgents[0].name : null);
    }
  };

  return (
    <div className="system-builder">
      {/* System Settings */}
      <div className="system-builder-section">
        <div className="system-builder-row">
          <div className="system-builder-field">
            <label>Global Timeout (seconds)</label>
            <input
              type="number"
              value={config.system.global_timeout_secs}
              onChange={(e) => updateSystemSettings(parseInt(e.target.value) || 60)}
              min={1}
              className="system-builder-input"
            />
          </div>
        </div>
      </div>

      {/* LLM Provider */}
      <div className="system-builder-section">
        <h4>LLM Provider</h4>
        <div className="system-builder-row">
          <div className="system-builder-field">
            <label>Provider Name</label>
            <input
              type="text"
              value="default"
              disabled
              className="system-builder-input system-builder-input-disabled"
            />
          </div>
          <div className="system-builder-field">
            <label>Base URL</label>
            <input
              type="text"
              value={config.llm_providers.default?.base_url ?? ''}
              onChange={(e) => updateProvider('base_url', e.target.value)}
              placeholder="http://localhost:11434"
              className="system-builder-input"
            />
          </div>
          <div className="system-builder-field">
            <label>Default Model</label>
            <input
              type="text"
              value={config.llm_providers.default?.default_model ?? ''}
              onChange={(e) => updateProvider('default_model', e.target.value)}
              placeholder="gemma3:4b"
              className="system-builder-input"
            />
          </div>
        </div>
      </div>

      {/* Agents */}
      <div className="system-builder-section">
        <div className="system-builder-section-header">
          <h4>Agents</h4>
          <button type="button" onClick={addAgent} className="btn btn-small">
            + Add Agent
          </button>
        </div>

        {config.agents.length === 0 ? (
          <div className="system-builder-empty">
            No agents defined. Click "Add Agent" to create one.
          </div>
        ) : (
          <div className="system-builder-agents">
            {config.agents.map((agent, index) => (
              <AgentForm
                key={`${agent.name}-${index}`}
                agent={agent}
                availableAgents={agentNames}
                providers={providerNames}
                onChange={(updated) => updateAgent(index, updated)}
                onDelete={() => deleteAgent(index)}
                isExpanded={expandedAgent === agent.name}
                onToggleExpand={() =>
                  setExpandedAgent(
                    expandedAgent === agent.name ? null : agent.name
                  )
                }
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default SystemBuilder;
