import { useState, useCallback, useMemo, useEffect } from 'react';
import { ReactFlowProvider } from '@xyflow/react';
import type { SystemConfigJson, AgentInfo } from '../types/api';
import TopologyGraph from './TopologyGraph';
import SystemBuilder from './SystemBuilder';

interface ConfigEditorProps {
  onSubmit: (name: string, config: SystemConfigJson) => void;
  isLoading?: boolean;
  initialConfig?: SystemConfigJson;
  initialName?: string;
  submitLabel?: string;
}

type EditorMode = 'form' | 'json' | 'preview';

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

function ConfigEditor({
  onSubmit,
  isLoading,
  initialConfig,
  initialName,
  submitLabel = 'Create System',
}: ConfigEditorProps) {
  const [mode, setMode] = useState<EditorMode>('form');
  const [name, setName] = useState(initialName ?? '');
  const [config, setConfig] = useState<SystemConfigJson>(
    initialConfig ?? EXAMPLE_CONFIG
  );
  const [configText, setConfigText] = useState(
    JSON.stringify(initialConfig ?? EXAMPLE_CONFIG, null, 2)
  );
  const [error, setError] = useState<string | null>(null);
  const [jsonSyncError, setJsonSyncError] = useState<string | null>(null);

  // Sync config to configText when switching to JSON mode
  useEffect(() => {
    if (mode === 'json') {
      setConfigText(JSON.stringify(config, null, 2));
      setJsonSyncError(null);
    }
  }, [mode]);

  // Parse JSON when switching away from JSON mode
  const syncFromJson = useCallback(() => {
    try {
      const parsed = JSON.parse(configText) as SystemConfigJson;
      setConfig(parsed);
      setJsonSyncError(null);
      return true;
    } catch (e) {
      setJsonSyncError(
        `Invalid JSON: ${e instanceof Error ? e.message : 'Parse error'}`
      );
      return false;
    }
  }, [configText]);

  const handleModeChange = (newMode: EditorMode) => {
    if (mode === 'json' && newMode !== 'json') {
      // Sync JSON changes before leaving JSON mode
      if (!syncFromJson()) {
        return; // Stay in JSON mode if sync failed
      }
    }
    setMode(newMode);
  };

  const previewAgents = useMemo((): AgentInfo[] | null => {
    try {
      if (mode === 'json') {
        const parsed = JSON.parse(configText) as SystemConfigJson;
        if (parsed.agents && parsed.agents.length > 0) {
          return configToAgentInfo(parsed);
        }
      } else {
        if (config.agents && config.agents.length > 0) {
          return configToAgentInfo(config);
        }
      }
    } catch {
      // Invalid config, no preview
    }
    return null;
  }, [mode, config, configText]);

  const validateConfig = useCallback(
    (cfg: SystemConfigJson): string | null => {
      if (!cfg.system?.global_timeout_secs) {
        return 'Missing system.global_timeout_secs';
      }
      if (!cfg.llm_providers || Object.keys(cfg.llm_providers).length === 0) {
        return 'At least one LLM provider is required';
      }
      if (!cfg.agents || cfg.agents.length === 0) {
        return 'At least one agent is required';
      }

      // Validate agent names
      const agentNames = new Set<string>();
      for (const agent of cfg.agents) {
        if (!agent.name.trim()) {
          return 'All agents must have a name';
        }
        if (agentNames.has(agent.name)) {
          return `Duplicate agent name: ${agent.name}`;
        }
        agentNames.add(agent.name);

        if (!agent.role.trim()) {
          return `Agent "${agent.name}" must have a role`;
        }
        if (!agent.system_prompt.trim()) {
          return `Agent "${agent.name}" must have a system prompt`;
        }

        // Validate provider exists
        if (!cfg.llm_providers[agent.handler.provider]) {
          return `Agent "${agent.name}" references unknown provider: ${agent.handler.provider}`;
        }

        // Validate connections
        for (const target of Object.keys(agent.connections)) {
          if (target === agent.name) {
            return `Agent "${agent.name}" cannot connect to itself`;
          }
          if (!agentNames.has(target) && !cfg.agents.some((a) => a.name === target)) {
            return `Agent "${agent.name}" connects to unknown agent: ${target}`;
          }
        }
      }

      return null;
    },
    []
  );

  const validateAndSubmit = useCallback(() => {
    setError(null);

    if (!name.trim()) {
      setError('System name is required');
      return;
    }

    let finalConfig: SystemConfigJson;

    if (mode === 'json') {
      try {
        finalConfig = JSON.parse(configText) as SystemConfigJson;
      } catch (e) {
        setError(`Invalid JSON: ${e instanceof Error ? e.message : 'Parse error'}`);
        return;
      }
    } else {
      finalConfig = config;
    }

    const validationError = validateConfig(finalConfig);
    if (validationError) {
      setError(validationError);
      return;
    }

    onSubmit(name.trim(), finalConfig);
  }, [name, mode, config, configText, validateConfig, onSubmit]);

  const loadExample = () => {
    setConfig(EXAMPLE_CONFIG);
    setConfigText(JSON.stringify(EXAMPLE_CONFIG, null, 2));
    setError(null);
    setJsonSyncError(null);
  };

  const formatJson = () => {
    try {
      const parsed = JSON.parse(configText);
      setConfigText(JSON.stringify(parsed, null, 2));
      setJsonSyncError(null);
    } catch (e) {
      setJsonSyncError(
        `Invalid JSON: ${e instanceof Error ? e.message : 'Parse error'}`
      );
    }
  };

  return (
    <div className="config-editor">
      <div className="config-editor-header">
        <h3>{initialConfig ? 'Edit System' : 'Create New System'}</h3>
        <div className="config-editor-tabs">
          <button
            onClick={() => handleModeChange('form')}
            className={`config-editor-tab ${mode === 'form' ? 'config-editor-tab-active' : ''}`}
          >
            Form
          </button>
          <button
            onClick={() => handleModeChange('json')}
            className={`config-editor-tab ${mode === 'json' ? 'config-editor-tab-active' : ''}`}
          >
            JSON
          </button>
          <button
            onClick={() => handleModeChange('preview')}
            className={`config-editor-tab ${mode === 'preview' ? 'config-editor-tab-active' : ''}`}
          >
            Preview
          </button>
        </div>
        <div className="config-editor-actions">
          <button onClick={loadExample} className="btn btn-small">
            Load Example
          </button>
          {mode === 'json' && (
            <button onClick={formatJson} className="btn btn-small">
              Format JSON
            </button>
          )}
        </div>
      </div>

      <div className="config-editor-body config-editor-body-single">
        <div className="config-editor-main">
          {/* System Name - always visible */}
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

          {/* Form Mode */}
          {mode === 'form' && (
            <SystemBuilder config={config} onChange={setConfig} />
          )}

          {/* JSON Mode */}
          {mode === 'json' && (
            <div className="config-editor-field">
              <label htmlFor="system-config">Configuration (JSON)</label>
              <textarea
                id="system-config"
                value={configText}
                onChange={(e) => setConfigText(e.target.value)}
                className="config-editor-textarea config-editor-textarea-tall"
                spellCheck={false}
              />
              {jsonSyncError && (
                <div className="config-editor-warning">{jsonSyncError}</div>
              )}
            </div>
          )}

          {/* Preview Mode */}
          {mode === 'preview' && (
            <div className="config-editor-preview-full">
              <h4>Topology Preview</h4>
              {previewAgents ? (
                <ReactFlowProvider>
                  <TopologyGraph agents={previewAgents} />
                </ReactFlowProvider>
              ) : (
                <div className="config-editor-preview-empty">
                  Configure agents to see topology preview
                </div>
              )}
            </div>
          )}

          {error && <div className="config-editor-error">{error}</div>}

          <button
            onClick={validateAndSubmit}
            disabled={isLoading}
            className="btn btn-primary config-editor-submit"
          >
            {isLoading ? 'Saving...' : submitLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

export default ConfigEditor;
