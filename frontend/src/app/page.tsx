'use client';

import { useCallback, useState, useMemo, useRef } from 'react';
import {
  ReactFlow,
  Controls,
  Background,
  MiniMap,
  addEdge,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  type Connection,
  type NodeTypes,
  BackgroundVariant,
  Panel,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import AgentNode from '../components/AgentNode';
import AgentModal from '../components/AgentModal';
import ToolNode from '../components/ToolNode';
import ToolModal from '../components/ToolModal';
import RestApiNode from '../components/RestApiNode';
import RestApiModal from '../components/RestApiModal';
import DatabaseNode from '../components/DatabaseNode';
import DatabaseModal from '../components/DatabaseModal';
import ChatPanel from '../components/ChatPanel';
import SystemsOverview from '../components/SystemsOverview';
import AuthGuard from '../components/AuthGuard';
import OrgSwitcher from '../components/OrgSwitcher';
import OrgManagement from '../components/OrgManagement';
import { authFetch, logout, getActiveOrg, type AuthUser } from '../lib/auth';
import type {
  AgentNodeData,
  ToolNodeData,
  RestApiNodeData,
  DatabaseNodeData,
  SystemConfigJson,
  AgentConfig,
  ToolConfig,
  DatabaseConfig,
  RoutingBehavior,
  EndpointType,
  DatabaseType,
} from '../types/agent';

const API_BASE = process.env.NEXT_PUBLIC_API_BASE || '/api/v1';

/** Convert a SystemConfigJson into ReactFlow nodes and edges, using saved positions when available */
function configToNodesAndEdges(config: SystemConfigJson): {
  nodes: Node<AgentNodeData | ToolNodeData | RestApiNodeData | DatabaseNodeData>[];
  edges: Edge[];
} {
  const savedPositions = config.editor_metadata?.node_positions || {};
  const nameToId: Record<string, string> = {};

  // Calculate grid positions as fallback
  const totalItems = config.agents.length + (config.tools?.length || 0) + (config.databases?.length || 0);
  const cols = Math.max(1, Math.ceil(Math.sqrt(totalItems)));
  const spacingX = 250;
  const spacingY = 200;

  let itemIndex = 0;

  // Convert agents to nodes
  const agentNodes: Node<AgentNodeData>[] = config.agents.map((agent) => {
    const nodeId = `node-${agent.name}`;
    nameToId[agent.name] = nodeId;

    const saved = savedPositions[agent.name];
    const row = Math.floor(itemIndex / cols);
    const col = itemIndex % cols;
    itemIndex++;

    return {
      id: nodeId,
      type: 'agent',
      position: saved
        ? { x: saved.x, y: saved.y }
        : { x: 100 + col * spacingX, y: 50 + row * spacingY },
      data: {
        name: agent.name,
        systemPrompt: agent.system_prompt || 'You are a helpful assistant.',
        provider: agent.handler?.provider || 'default',
        model: agent.handler?.model || 'llama3.2',
        routing: agent.handler?.routing || false,
        routingBehavior: (agent.handler?.routing_behavior as RoutingBehavior) || 'best',
        temperature: agent.handler?.options?.temperature ?? 0.7,
        maxTokens: agent.handler?.options?.max_tokens ?? 1000,
        entryPoint: agent.entry_point || false,
        maxTurns: agent.handler?.max_turns ?? 1,
      },
    };
  });

  // Split tools into MCP tools vs REST API (HTTP) tools
  const allTools = config.tools || [];
  const mcpTools = allTools.filter((t) => {
    const type = t.endpoint.type || (t.endpoint.mcp_tool_name ? 'mcp' : 'http');
    return type === 'mcp';
  });
  const httpTools = allTools.filter((t) => {
    const type = t.endpoint.type || (t.endpoint.mcp_tool_name ? 'mcp' : 'http');
    return type !== 'mcp';
  });

  // Convert MCP tools to Tool nodes
  const toolNodes: Node<ToolNodeData>[] = mcpTools.map((tool) => {
    const nodeId = `node-${tool.name}`;
    nameToId[tool.name] = nodeId;

    const saved = savedPositions[tool.name];
    const row = Math.floor(itemIndex / cols);
    const col = itemIndex % cols;
    itemIndex++;

    return {
      id: nodeId,
      type: 'tool',
      position: saved
        ? { x: saved.x, y: saved.y }
        : { x: 100 + col * spacingX, y: 50 + row * spacingY },
      data: {
        name: tool.name,
        description: tool.description,
        endpointType: 'mcp' as EndpointType,
        endpointUrl: tool.endpoint.url,
        endpointMethod: tool.endpoint.method || 'POST',
        mcpToolName: tool.endpoint.mcp_tool_name || '',
        headers: tool.endpoint.headers || {},
        bodyTemplate: tool.endpoint.body_template
          ? JSON.stringify(tool.endpoint.body_template, null, 2)
          : '',
        parameters: tool.parameters
          ? JSON.stringify(tool.parameters, null, 2)
          : '{\n  "type": "object",\n  "properties": {}\n}',
        extractPath: tool.response_mapping?.extract_path || '',
        responseFormat: tool.response_mapping?.format || 'json',
        timeoutSecs: tool.timeout_secs || 30,
      },
    };
  });

  // Convert HTTP tools to REST API nodes
  const restApiNodes: Node<RestApiNodeData>[] = httpTools.map((tool) => {
    const nodeId = `node-${tool.name}`;
    nameToId[tool.name] = nodeId;

    const saved = savedPositions[tool.name];
    const row = Math.floor(itemIndex / cols);
    const col = itemIndex % cols;
    itemIndex++;

    return {
      id: nodeId,
      type: 'restapi',
      position: saved
        ? { x: saved.x, y: saved.y }
        : { x: 100 + col * spacingX, y: 50 + row * spacingY },
      data: {
        name: tool.name,
        description: tool.description,
        endpointUrl: tool.endpoint.url,
        endpointMethod: tool.endpoint.method || 'GET',
        headers: tool.endpoint.headers || {},
        bodyTemplate: tool.endpoint.body_template
          ? JSON.stringify(tool.endpoint.body_template, null, 2)
          : '',
        parameters: tool.parameters
          ? JSON.stringify(tool.parameters, null, 2)
          : '{\n  "type": "object",\n  "properties": {}\n}',
        extractPath: tool.response_mapping?.extract_path || '',
        responseFormat: tool.response_mapping?.format || 'json',
        timeoutSecs: tool.timeout_secs || 30,
      },
    };
  });

  // Convert databases to nodes
  const databaseNodes: Node<DatabaseNodeData>[] = (config.databases || []).map((db) => {
    const nodeId = `node-${db.name}`;
    nameToId[db.name] = nodeId;

    const saved = savedPositions[db.name];
    const row = Math.floor(itemIndex / cols);
    const col = itemIndex % cols;
    itemIndex++;

    return {
      id: nodeId,
      type: 'database',
      position: saved
        ? { x: saved.x, y: saved.y }
        : { x: 100 + col * spacingX, y: 50 + row * spacingY },
      data: {
        name: db.name,
        description: db.description || '',
        databaseType: (db.database_type || 'sqlite') as DatabaseType,
        connectionString: db.connection_string,
        maxConnections: db.max_connections || 5,
        timeoutSecs: db.timeout_secs || 30,
        readOnly: db.read_only ?? true,
      },
    };
  });

  // Build edges from connections
  const newEdges: Edge[] = [];
  config.agents.forEach((agent) => {
    if (agent.connections) {
      const sourceId = nameToId[agent.name];
      Object.keys(agent.connections).forEach((targetName) => {
        const targetId = nameToId[targetName];
        if (sourceId && targetId) {
          newEdges.push({
            id: `e-${sourceId}-${targetId}`,
            source: sourceId,
            target: targetId,
            animated: true,
          });
        }
      });
    }
  });

  return { nodes: [...agentNodes, ...toolNodes, ...restApiNodes, ...databaseNodes], edges: newEdges };
}

export default function Page() {
  return (
    <AuthGuard>
      {(user) => <EditorPage user={user} />}
    </AuthGuard>
  );
}

function EditorPage({ user }: { user: AuthUser }) {
  // View state: overview (dashboard) vs editor (ReactFlow)
  const [currentView, setCurrentView] = useState<'overview' | 'editor'>('overview');
  const [currentSystemName, setCurrentSystemName] = useState<string | null>(null);
  const [editingName, setEditingName] = useState(false);
  const [saving, setSaving] = useState(false);
  const nameInputRef = useRef<HTMLInputElement>(null);

  // Org state
  const [activeOrgId, setActiveOrgId] = useState<string | null>(getActiveOrg());
  const [managingOrgId, setManagingOrgId] = useState<string | null>(null);

  const [nodes, setNodes, onNodesChange] = useNodesState([] as Node<AgentNodeData | ToolNodeData | RestApiNodeData | DatabaseNodeData>[]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([] as Edge[]);

  // Modal state - separate for agents, tools, REST APIs, and databases
  const [agentModalOpen, setAgentModalOpen] = useState(false);
  const [toolModalOpen, setToolModalOpen] = useState(false);
  const [restApiModalOpen, setRestApiModalOpen] = useState(false);
  const [databaseModalOpen, setDatabaseModalOpen] = useState(false);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  // Chat panel state
  const [chatOpen, setChatOpen] = useState(false);

  // File input ref for JSON import
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Node types configuration - memoized to prevent re-renders
  const nodeTypes: NodeTypes = useMemo(() => ({
    agent: AgentNode,
    tool: ToolNode,
    restapi: RestApiNode,
    database: DatabaseNode,
  }), []);

  // Get selected node
  const selectedNode = nodes.find((n) => n.id === selectedNodeId);
  const selectedAgentData = selectedNode?.type === 'agent' ? selectedNode.data as AgentNodeData : null;
  const selectedToolData = selectedNode?.type === 'tool' ? selectedNode.data as ToolNodeData : null;
  const selectedRestApiData = selectedNode?.type === 'restapi' ? selectedNode.data as RestApiNodeData : null;
  const selectedDatabaseData = selectedNode?.type === 'database' ? selectedNode.data as DatabaseNodeData : null;

  // Handle new connections with validation
  const onConnect = useCallback(
    (connection: Connection) => {
      const sourceNode = nodes.find((n) => n.id === connection.source);
      if (sourceNode?.type === 'tool') {
        console.warn('Tools cannot initiate connections');
        return;
      }
      if (sourceNode?.type === 'restapi') {
        console.warn('REST APIs cannot initiate connections');
        return;
      }
      if (sourceNode?.type === 'database') {
        console.warn('Databases cannot initiate connections');
        return;
      }
      setEdges((eds) => addEdge({ ...connection, animated: true }, eds));
    },
    [nodes, setEdges]
  );

  // Handle node double-click to edit
  const onNodeDoubleClick = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      setSelectedNodeId(node.id);
      if (node.type === 'tool') {
        setToolModalOpen(true);
      } else if (node.type === 'restapi') {
        setRestApiModalOpen(true);
      } else if (node.type === 'database') {
        setDatabaseModalOpen(true);
      } else {
        setAgentModalOpen(true);
      }
    },
    []
  );

  // Add new agent
  const addAgent = useCallback(() => {
    const newId = `agent-${Date.now()}`;
    const newNode: Node<AgentNodeData> = {
      id: newId,
      type: 'agent',
      position: {
        x: Math.random() * 300 + 100,
        y: Math.random() * 200 + 100,
      },
      data: {
        name: 'New Agent',
        systemPrompt: 'You are a helpful assistant.',
        provider: 'default',
        model: 'llama3.2',
        routing: false,
        routingBehavior: 'best',
        temperature: 0.7,
        maxTokens: 1000,
        entryPoint: false,
        maxTurns: 1,
      },
    };
    setNodes((nds) => [...nds, newNode]);
    setSelectedNodeId(newId);
    setAgentModalOpen(true);
  }, [setNodes]);

  // Add new tool (MCP by default as it's simpler to configure)
  const addTool = useCallback(() => {
    const newId = `tool-${Date.now()}`;
    const newNode: Node<ToolNodeData> = {
      id: newId,
      type: 'tool',
      position: {
        x: Math.random() * 300 + 100,
        y: Math.random() * 200 + 100,
      },
      data: {
        name: 'New Tool',
        description: 'An MCP tool',
        endpointType: 'mcp',
        endpointUrl: 'https://example.com/mcp',
        endpointMethod: 'POST',
        mcpToolName: 'tool_name',
        headers: {},
        bodyTemplate: '',
        parameters: '{\n  "type": "object",\n  "properties": {\n    "query": { "type": "string", "description": "The query parameter" }\n  },\n  "required": ["query"]\n}',
        extractPath: '',
        responseFormat: 'json',
        timeoutSecs: 30,
      },
    };
    setNodes((nds) => [...nds, newNode]);
    setSelectedNodeId(newId);
    setToolModalOpen(true);
  }, [setNodes]);

  // Add new REST API
  const addRestApi = useCallback(() => {
    const newId = `restapi-${Date.now()}`;
    const newNode: Node<RestApiNodeData> = {
      id: newId,
      type: 'restapi',
      position: {
        x: Math.random() * 300 + 100,
        y: Math.random() * 200 + 100,
      },
      data: {
        name: 'New REST API',
        description: 'A REST API endpoint',
        endpointUrl: 'https://api.example.com/endpoint',
        endpointMethod: 'GET',
        headers: {},
        bodyTemplate: '',
        parameters: '{\n  "type": "object",\n  "properties": {}\n}',
        extractPath: '',
        responseFormat: 'json',
        timeoutSecs: 30,
      },
    };
    setNodes((nds) => [...nds, newNode]);
    setSelectedNodeId(newId);
    setRestApiModalOpen(true);
  }, [setNodes]);

  // Add new database
  const addDatabase = useCallback(() => {
    const newId = `database-${Date.now()}`;
    const newNode: Node<DatabaseNodeData> = {
      id: newId,
      type: 'database',
      position: {
        x: Math.random() * 300 + 100,
        y: Math.random() * 200 + 100,
      },
      data: {
        name: 'New Database',
        description: 'A database connection',
        databaseType: 'sqlite',
        connectionString: 'sqlite://data.db',
        maxConnections: 5,
        timeoutSecs: 30,
        readOnly: true,
      },
    };
    setNodes((nds) => [...nds, newNode]);
    setSelectedNodeId(newId);
    setDatabaseModalOpen(true);
  }, [setNodes]);

  // Save agent changes
  const handleSaveAgent = useCallback(
    (data: AgentNodeData) => {
      if (!selectedNodeId) return;
      setNodes((nds) =>
        nds.map((node) =>
          node.id === selectedNodeId
            ? { ...node, data: { ...data } }
            : node
        )
      );
      setAgentModalOpen(false);
      setSelectedNodeId(null);
    },
    [selectedNodeId, setNodes]
  );

  // Save tool changes
  const handleSaveTool = useCallback(
    (data: ToolNodeData) => {
      if (!selectedNodeId) return;
      setNodes((nds) =>
        nds.map((node) =>
          node.id === selectedNodeId
            ? { ...node, data: { ...data } }
            : node
        )
      );
      setToolModalOpen(false);
      setSelectedNodeId(null);
    },
    [selectedNodeId, setNodes]
  );

  // Save REST API changes
  const handleSaveRestApi = useCallback(
    (data: RestApiNodeData) => {
      if (!selectedNodeId) return;
      setNodes((nds) =>
        nds.map((node) =>
          node.id === selectedNodeId
            ? { ...node, data: { ...data } }
            : node
        )
      );
      setRestApiModalOpen(false);
      setSelectedNodeId(null);
    },
    [selectedNodeId, setNodes]
  );

  // Save database changes
  const handleSaveDatabase = useCallback(
    (data: DatabaseNodeData) => {
      if (!selectedNodeId) return;
      setNodes((nds) =>
        nds.map((node) =>
          node.id === selectedNodeId
            ? { ...node, data: { ...data } }
            : node
        )
      );
      setDatabaseModalOpen(false);
      setSelectedNodeId(null);
    },
    [selectedNodeId, setNodes]
  );

  // Delete node (agent, tool, or database)
  const handleDeleteNode = useCallback(() => {
    if (!selectedNodeId) return;
    setNodes((nds) => nds.filter((node) => node.id !== selectedNodeId));
    setEdges((eds) =>
      eds.filter(
        (edge) => edge.source !== selectedNodeId && edge.target !== selectedNodeId
      )
    );
    setAgentModalOpen(false);
    setToolModalOpen(false);
    setRestApiModalOpen(false);
    setDatabaseModalOpen(false);
    setSelectedNodeId(null);
  }, [selectedNodeId, setNodes, setEdges]);

  // Export to API format (includes editor_metadata with node positions)
  const exportConfig = useCallback((): SystemConfigJson => {
    const agentNodes = nodes.filter((n) => n.type === 'agent');
    const toolNodes = nodes.filter((n) => n.type === 'tool');
    const restApiNodes = nodes.filter((n) => n.type === 'restapi');
    const databaseNodes = nodes.filter((n) => n.type === 'database');

    // Build node positions map (keyed by agent/tool name)
    const nodePositions: Record<string, { x: number; y: number }> = {};
    nodes.forEach((node) => {
      const name = node.data.name as string;
      nodePositions[name] = { x: node.position.x, y: node.position.y };
    });

    // Convert agent nodes to AgentConfig
    const agents: AgentConfig[] = agentNodes.map((node) => {
      const data = node.data as AgentNodeData;
      const connections: Record<string, { type: 'blocking' | 'notify'; timeout_secs?: number }> = {};

      edges
        .filter((edge) => edge.source === node.id)
        .forEach((edge) => {
          const targetNode = nodes.find((n) => n.id === edge.target);
          if (targetNode) {
            connections[targetNode.data.name as string] = {
              type: 'blocking',
              timeout_secs: 60,
            };
          }
        });

      return {
        name: data.name,
        system_prompt: data.systemPrompt,
        handler: {
          provider: data.provider,
          model: data.model,
          routing: data.routing,
          routing_behavior: data.routingBehavior,
          options: {
            temperature: data.temperature,
            max_tokens: data.maxTokens,
          },
          max_turns: data.maxTurns !== 1 ? data.maxTurns : undefined,
        },
        connections: Object.keys(connections).length > 0 ? connections : undefined,
        entry_point: data.entryPoint || undefined,
      };
    });

    // Convert tool nodes to ToolConfig
    const tools: ToolConfig[] = toolNodes.map((node) => {
      const data = node.data as ToolNodeData;

      let parameters: Record<string, unknown> = {};
      let bodyTemplate: Record<string, unknown> | undefined;

      try {
        if (data.parameters) {
          parameters = JSON.parse(data.parameters);
        }
      } catch {
        console.warn('Failed to parse parameters JSON for tool:', data.name);
      }

      if (data.endpointType !== 'mcp') {
        try {
          if (data.bodyTemplate) {
            bodyTemplate = JSON.parse(data.bodyTemplate);
          }
        } catch {
          console.warn('Failed to parse body template JSON for tool:', data.name);
        }
      }

      return {
        name: data.name,
        description: data.description,
        parameters,
        endpoint: {
          url: data.endpointUrl,
          type: data.endpointType || 'http',
          method: data.endpointMethod,
          headers: Object.keys(data.headers).length > 0 ? data.headers : undefined,
          body_template: data.endpointType !== 'mcp' ? bodyTemplate : undefined,
          mcp_tool_name: data.endpointType === 'mcp' ? data.mcpToolName : undefined,
        },
        response_mapping: {
          extract_path: data.extractPath || undefined,
          format: data.responseFormat,
        },
        timeout_secs: data.timeoutSecs,
      };
    });

    // Convert REST API nodes to ToolConfig (type: 'http')
    const restApiTools: ToolConfig[] = restApiNodes.map((node) => {
      const data = node.data as RestApiNodeData;

      let bodyTemplate: Record<string, unknown> | undefined;
      let parameters: Record<string, unknown> = {};

      try {
        if (data.bodyTemplate) {
          bodyTemplate = JSON.parse(data.bodyTemplate);
        }
      } catch {
        console.warn('Failed to parse body template JSON for REST API:', data.name);
      }

      try {
        if (data.parameters) {
          parameters = JSON.parse(data.parameters);
        }
      } catch {
        console.warn('Failed to parse parameters JSON for REST API:', data.name);
      }

      return {
        name: data.name,
        description: data.description,
        parameters,
        endpoint: {
          url: data.endpointUrl,
          type: 'http' as EndpointType,
          method: data.endpointMethod,
          headers: Object.keys(data.headers).length > 0 ? data.headers : undefined,
          body_template: bodyTemplate,
        },
        response_mapping: {
          extract_path: data.extractPath || undefined,
          format: data.responseFormat,
        },
        timeout_secs: data.timeoutSecs,
      };
    });

    // Merge MCP tools and REST API tools
    const allTools = [...tools, ...restApiTools];

    // Convert database nodes to DatabaseConfig
    const databases: DatabaseConfig[] = databaseNodes.map((node) => {
      const data = node.data as DatabaseNodeData;
      return {
        name: data.name,
        description: data.description,
        connection_string: data.connectionString,
        database_type: data.databaseType,
        max_connections: data.maxConnections,
        timeout_secs: data.timeoutSecs,
        read_only: data.readOnly,
      };
    });

    return {
      system: { global_timeout_secs: 60 },
      llm_providers: {
        default: {
          type: 'ollama',
          base_url: 'http://localhost:11434',
          default_model: 'llama3.2',
        },
      },
      agents,
      tools: allTools.length > 0 ? allTools : undefined,
      databases: databases.length > 0 ? databases : undefined,
      editor_metadata: {
        node_positions: nodePositions,
      },
    };
  }, [nodes, edges]);

  // Handle export button click (copy to clipboard)
  const handleExport = useCallback(() => {
    const config = exportConfig();
    const json = JSON.stringify(config, null, 2);
    navigator.clipboard.writeText(json).then(() => {
      alert('Configuration copied to clipboard!');
    });
  }, [exportConfig]);

  // Load a config into the editor (from overview or import)
  const loadConfigIntoEditor = useCallback((name: string | null, config: SystemConfigJson) => {
    const { nodes: newNodes, edges: newEdges } = configToNodesAndEdges(config);
    setNodes(newNodes);
    setEdges(newEdges);
    setCurrentSystemName(name);
    setCurrentView('editor');
  }, [setNodes, setEdges]);

  // Handle selecting a system from the overview
  const handleSelectSystem = useCallback((name: string, config: SystemConfigJson) => {
    loadConfigIntoEditor(name, config);
  }, [loadConfigIntoEditor]);

  // Handle "New System" from the overview
  const handleNewSystem = useCallback(() => {
    setNodes([]);
    setEdges([]);
    setCurrentSystemName(null);
    setCurrentView('editor');
    setEditingName(true);
    setTimeout(() => nameInputRef.current?.focus(), 100);
  }, [setNodes, setEdges]);

  // Handle import JSON (shared between overview and editor)
  const handleImport = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const content = e.target?.result as string;
        const config: SystemConfigJson = JSON.parse(content);

        if (!config.agents || !Array.isArray(config.agents)) {
          alert('Invalid configuration: missing agents array');
          return;
        }

        const baseName = file.name.replace(/\.json$/i, '');
        loadConfigIntoEditor(null, config);
        // Set a default name from the file name (user can change on save)
        setCurrentSystemName(baseName);
      } catch (error) {
        console.error('Failed to parse JSON:', error);
        alert('Failed to parse JSON file. Please ensure it is valid JSON.');
      }
    };

    reader.readAsText(file);
    event.target.value = '';
  }, [loadConfigIntoEditor]);

  // Trigger file input from overview or editor
  const triggerImport = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  // Save system to backend. Optional overrides for rename-and-save flow.
  const handleSave = useCallback(async (nameOverride?: string, oldName?: string) => {
    const config = exportConfig();

    const name = nameOverride || currentSystemName;
    if (!name) {
      alert('Please set a system name before saving.');
      setEditingName(true);
      setTimeout(() => nameInputRef.current?.focus(), 50);
      return;
    }

    setSaving(true);
    try {
      // If renamed, delete the old system first
      if (oldName && oldName !== name) {
        await authFetch(`${API_BASE}/systems/${encodeURIComponent(oldName)}`, { method: 'DELETE' }).catch(() => {});
      }

      // Try PUT first (update existing), fall back to POST (create new)
      const updateRes = await authFetch(`${API_BASE}/systems/${encodeURIComponent(name)}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ config }),
      });

      if (updateRes.ok) {
        setCurrentSystemName(name);
        return;
      }

      if (updateRes.status === 404) {
        // System doesn't exist yet, create it
        const createBody: Record<string, unknown> = { name, config };
        if (activeOrgId) {
          createBody.org_id = activeOrgId;
        }
        const createRes = await authFetch(`${API_BASE}/systems`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(createBody),
        });

        if (!createRes.ok) {
          const err = await createRes.json().catch(() => ({}));
          throw new Error(err.error || `Failed to create system (${createRes.status})`);
        }

        setCurrentSystemName(name);
        return;
      }

      const err = await updateRes.json().catch(() => ({}));
      throw new Error(err.error || `Failed to save system (${updateRes.status})`);
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to save system');
    } finally {
      setSaving(false);
    }
  }, [exportConfig, currentSystemName, activeOrgId]);

  // Back to overview
  const handleBackToOverview = useCallback(() => {
    setChatOpen(false);
    setCurrentView('overview');
  }, []);

  // Get current config for chat
  const currentConfig = useMemo(() => exportConfig(), [exportConfig]);
  const systemName = currentSystemName || 'untitled-system';

  // Hidden file input (shared across views)
  const fileInput = (
    <input
      type="file"
      ref={fileInputRef}
      onChange={handleImport}
      accept=".json,application/json"
      className="hidden"
    />
  );

  // ========== Overview View ==========
  if (currentView === 'overview') {
    return (
      <>
        {fileInput}
        {/* User bar */}
        <div className="bg-zinc-950 border-b border-zinc-800 px-6 py-2 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <OrgSwitcher onOrgChange={(id) => setActiveOrgId(id)} />
            {activeOrgId && (
              <button
                onClick={() => setManagingOrgId(activeOrgId)}
                className="p-1.5 text-zinc-500 hover:text-zinc-300 rounded-md hover:bg-zinc-800 transition-colors"
                title="Manage organization"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
              </button>
            )}
          </div>
          <div className="flex items-center gap-3">
            <span className="text-sm text-zinc-400">{user.display_name}</span>
            <button
              onClick={logout}
              className="text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
            >
              Sign out
            </button>
          </div>
        </div>
        <SystemsOverview
          onSelectSystem={handleSelectSystem}
          onNewSystem={handleNewSystem}
          onImportJson={triggerImport}
          orgId={activeOrgId}
        />
        {/* Org Management Modal */}
        {managingOrgId && (
          <OrgManagement
            orgId={managingOrgId}
            onClose={() => setManagingOrgId(null)}
          />
        )}
      </>
    );
  }

  // ========== Editor View ==========
  return (
    <div className="w-screen h-screen">
      {fileInput}
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeDoubleClick={onNodeDoubleClick}
        nodeTypes={nodeTypes}
        fitView
        snapToGrid
        snapGrid={[15, 15]}
        deleteKeyCode={['Backspace', 'Delete']}
      >
        <Background variant={BackgroundVariant.Dots} gap={20} size={1} />
        <Controls />
        <MiniMap
          nodeColor={(node) => {
            if (node.type === 'tool') return '#f59e0b';
            if (node.type === 'restapi') return '#f43f5e';
            if (node.type === 'database') return '#06b6d4';
            return '#3b82f6';
          }}
          maskColor="rgba(0, 0, 0, 0.1)"
        />

        {/* Toolbar */}
        <Panel position="top-left" className="flex gap-2 flex-wrap">
          <button
            onClick={handleBackToOverview}
            className="flex items-center gap-2 px-4 py-2 bg-zinc-700 text-white text-sm font-medium rounded-lg shadow-md hover:bg-zinc-600 transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
            Systems
          </button>
          <button
            onClick={() => handleSave()}
            disabled={saving}
            className="flex items-center gap-2 px-4 py-2 bg-emerald-600 text-white text-sm font-medium rounded-lg shadow-md hover:bg-emerald-700 disabled:opacity-50 transition-colors"
          >
            {saving ? (
              <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
              </svg>
            ) : (
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
              </svg>
            )}
            Save
          </button>
          <div className="w-px h-8 bg-zinc-600 self-center" />
          <button
            onClick={addAgent}
            className="flex items-center gap-2 px-4 py-2 bg-blue-500 text-white text-sm font-medium rounded-lg shadow-md hover:bg-blue-600 transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
            </svg>
            Agent
          </button>
          <div className="w-px h-8 bg-zinc-600 self-center" />
          {/* Resources group: Tool, REST API, Database */}
          <div className="flex items-center gap-1 bg-zinc-800/60 rounded-lg p-1">
            <button
              onClick={addTool}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-amber-500 text-white text-xs font-medium rounded-md shadow-sm hover:bg-amber-600 transition-colors"
            >
              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                />
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
              Tool
            </button>
            <button
              onClick={addRestApi}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-rose-500 text-white text-xs font-medium rounded-md shadow-sm hover:bg-rose-600 transition-colors"
            >
              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
                />
              </svg>
              REST
            </button>
            <button
              onClick={addDatabase}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-cyan-600 text-white text-xs font-medium rounded-md shadow-sm hover:bg-cyan-700 transition-colors"
            >
              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <ellipse cx="12" cy="5" rx="9" ry="3" strokeWidth={2} />
                <path strokeWidth={2} d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5" />
                <path strokeWidth={2} d="M3 12c0 1.66 4.03 3 9 3s9-1.34 9-3" />
              </svg>
              DB
            </button>
          </div>
          <button
            onClick={triggerImport}
            className="flex items-center gap-2 px-4 py-2 bg-zinc-600 text-white text-sm font-medium rounded-lg shadow-md hover:bg-zinc-700 transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m4-8l-4 4m0 0l-4-4m4 4V4" />
            </svg>
            Import
          </button>
          <button
            onClick={handleExport}
            className="flex items-center gap-2 px-4 py-2 bg-zinc-700 text-white text-sm font-medium rounded-lg shadow-md hover:bg-zinc-800 transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
            </svg>
            Export
          </button>
          <button
            onClick={() => setChatOpen(true)}
            className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white text-sm font-medium rounded-lg shadow-md hover:bg-green-700 transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
            </svg>
            Chat
          </button>
        </Panel>

        {/* System name indicator (click to edit) */}
        <Panel position="top-right" className="bg-zinc-800/90 px-3 py-1.5 rounded-lg">
          <span className="text-xs text-zinc-400">System: </span>
          {editingName ? (
            <input
              ref={nameInputRef}
              autoFocus
              className="text-xs text-zinc-200 font-medium bg-zinc-700 border border-zinc-500 rounded px-1.5 py-0.5 outline-none focus:border-blue-500 w-40"
              defaultValue={currentSystemName || ''}
              placeholder="Enter system name"
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  const val = e.currentTarget.value.trim();
                  if (val) {
                    const oldName = currentSystemName;
                    setCurrentSystemName(val);
                    setEditingName(false);
                    handleSave(val, oldName && oldName !== val ? oldName : undefined);
                  }
                } else if (e.key === 'Escape') {
                  setEditingName(false);
                }
              }}
              onBlur={(e) => {
                const val = e.currentTarget.value.trim();
                if (val) {
                  const oldName = currentSystemName;
                  setCurrentSystemName(val);
                  setEditingName(false);
                  handleSave(val, oldName && oldName !== val ? oldName : undefined);
                } else {
                  setEditingName(false);
                }
              }}
            />
          ) : (
            <button
              onClick={() => setEditingName(true)}
              className="text-xs text-zinc-200 font-medium hover:text-blue-400 transition-colors cursor-pointer"
              title="Click to rename"
            >
              {systemName}
              <svg className="w-3 h-3 inline-block ml-1 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
              </svg>
            </button>
          )}
        </Panel>

        {/* Help text */}
        <Panel position="bottom-center" className="text-xs text-zinc-400 bg-zinc-800/90 px-3 py-1.5 rounded-lg">
          Double-click to edit &bull; Drag to connect &bull; Delete/Backspace to remove
        </Panel>
      </ReactFlow>

      {/* Agent Edit Modal */}
      <AgentModal
        isOpen={agentModalOpen}
        agent={selectedAgentData}
        onSave={handleSaveAgent}
        onDelete={handleDeleteNode}
        onClose={() => {
          setAgentModalOpen(false);
          setSelectedNodeId(null);
        }}
      />

      {/* Tool Edit Modal */}
      <ToolModal
        isOpen={toolModalOpen}
        tool={selectedToolData}
        onSave={handleSaveTool}
        onDelete={handleDeleteNode}
        onClose={() => {
          setToolModalOpen(false);
          setSelectedNodeId(null);
        }}
      />

      {/* REST API Edit Modal */}
      <RestApiModal
        isOpen={restApiModalOpen}
        restApi={selectedRestApiData}
        onSave={handleSaveRestApi}
        onDelete={handleDeleteNode}
        onClose={() => {
          setRestApiModalOpen(false);
          setSelectedNodeId(null);
        }}
      />

      {/* Database Edit Modal */}
      <DatabaseModal
        isOpen={databaseModalOpen}
        database={selectedDatabaseData}
        onSave={handleSaveDatabase}
        onDelete={handleDeleteNode}
        onClose={() => {
          setDatabaseModalOpen(false);
          setSelectedNodeId(null);
        }}
      />

      {/* Chat Panel */}
      <ChatPanel
        isOpen={chatOpen}
        onClose={() => setChatOpen(false)}
        config={currentConfig}
        systemName={systemName}
      />
    </div>
  );
}
