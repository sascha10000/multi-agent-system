import { useState, useCallback, useMemo } from 'react';
import { ReactFlowProvider } from '@xyflow/react';
import type { SystemConfigJson, AgentInfo } from '../types/api';
import TopologyGraph from './TopologyGraph';

interface ConfigEditorProps {
  onSubmit: (name: string, config: SystemConfigJson) => void;
  isLoading?: boolean;
}

const EXAMPLE_CONFIG: SystemConfigJson = {
  system: {
    global_timeout_secs: 60,
  },
  llm_providers: {
    default: {
      type: 'ollama',
      base_url: 'http://localhost:11434',
      default_model: 'gemma3:4b',
    },
  },
  agents: [
    {
      name: 'Coordinator',
      role: 'Routes requests to specialized agents',
      system_prompt: 'You are a coordinator. Route research questions to Researcher.',
      handler: {
        provider: 'default',
        model: 'gemma3:4b',
        routing: true,
        options: { temperature: 0.3, max_tokens: 500 },
      },
      connections: {
        Researcher: { type: 'blocking', timeout_secs: 60 },
      },
    },
    {
      name: 'Researcher',
      role: 'Researches and answers questions',
      system_prompt: 'You are a research assistant. Provide detailed, accurate answers.',
      handler: {
        provider: 'default',
        model: 'gemma3:4b',
        routing: false,
        options: { temperature: 0.7, max_tokens: 1000 },
      },
      connections: {},
    },
  ],
};

function configToAgentInfo(config: SystemConfigJson): AgentInfo[] {
  return config.agents.map((agent) => ({
    name: agent.name,
    role: agent.role,
    routing: agent.handler.routing,
    connections: Object.entries(agent.connections).map(([target, conn]) => ({
      target,
      connection_type: conn.type,
      timeout_secs: conn.timeout_secs,
    })),
  }));
}

function ConfigEditor({ onSubmit, isLoading }: ConfigEditorProps) {
  const [name, setName] = useState('');
  const [configText, setConfigText] = useState(JSON.stringify(EXAMPLE_CONFIG, null, 2));
  const [error, setError] = useState<string | null>(null);
  const [showPreview, setShowPreview] = useState(true);

  const previewAgents = useMemo((): AgentInfo[] | null => {
    try {
      const config = JSON.parse(configText) as SystemConfigJson;
      if (config.agents && config.agents.length > 0) {
        return configToAgentInfo(config);
      }
    } catch {
      // Invalid JSON, no preview
    }
    return null;
  }, [configText]);

  const validateAndSubmit = useCallback(() => {
    setError(null);

    if (!name.trim()) {
      setError('System name is required');
      return;
    }

    try {
      const config = JSON.parse(configText) as SystemConfigJson;

      // Basic validation
      if (!config.system?.global_timeout_secs) {
        setError('Missing system.global_timeout_secs');
        return;
      }
      if (!config.llm_providers || Object.keys(config.llm_providers).length === 0) {
        setError('At least one LLM provider is required');
        return;
      }
      if (!config.agents || config.agents.length === 0) {
        setError('At least one agent is required');
        return;
      }

      onSubmit(name.trim(), config);
    } catch (e) {
      setError(`Invalid JSON: ${e instanceof Error ? e.message : 'Parse error'}`);
    }
  }, [name, configText, onSubmit]);

  const loadExample = () => {
    setConfigText(JSON.stringify(EXAMPLE_CONFIG, null, 2));
    setError(null);
  };

  const formatJson = () => {
    try {
      const parsed = JSON.parse(configText);
      setConfigText(JSON.stringify(parsed, null, 2));
      setError(null);
    } catch (e) {
      setError(`Invalid JSON: ${e instanceof Error ? e.message : 'Parse error'}`);
    }
  };

  return (
    <div className="config-editor">
      <div className="config-editor-header">
        <h3>Create New System</h3>
        <div className="config-editor-actions">
          <button onClick={loadExample} className="btn btn-small">
            Load Example
          </button>
          <button onClick={formatJson} className="btn btn-small">
            Format JSON
          </button>
          <button
            onClick={() => setShowPreview(!showPreview)}
            className={`btn btn-small ${showPreview ? 'btn-primary' : ''}`}
          >
            {showPreview ? 'Hide Preview' : 'Show Preview'}
          </button>
        </div>
      </div>

      <div className="config-editor-body">
        <div className="config-editor-form">
          <div className="config-editor-field">
            <label htmlFor="system-name">System Name</label>
            <input
              id="system-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="my-system"
              className="config-editor-input"
            />
          </div>

          <div className="config-editor-field">
            <label htmlFor="system-config">Configuration (JSON)</label>
            <textarea
              id="system-config"
              value={configText}
              onChange={(e) => setConfigText(e.target.value)}
              className="config-editor-textarea"
              spellCheck={false}
            />
          </div>

          {error && <div className="config-editor-error">{error}</div>}

          <button
            onClick={validateAndSubmit}
            disabled={isLoading}
            className="btn btn-primary config-editor-submit"
          >
            {isLoading ? 'Creating...' : 'Create System'}
          </button>
        </div>

        {showPreview && (
          <div className="config-editor-preview">
            <h4>Topology Preview</h4>
            {previewAgents ? (
              <ReactFlowProvider>
                <TopologyGraph agents={previewAgents} />
              </ReactFlowProvider>
            ) : (
              <div className="config-editor-preview-empty">
                Enter valid JSON to see topology preview
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export default ConfigEditor;
