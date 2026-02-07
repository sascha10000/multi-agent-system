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
import ChatPanel from '../components/ChatPanel';
import type {
  AgentNodeData,
  ToolNodeData,
  SystemConfigJson,
  AgentConfig,
  ToolConfig,
  RoutingBehavior,
  EndpointType,
} from '../types/agent';

// Initial nodes for demo - includes an MCP tool example
const initialNodes: Node<AgentNodeData | ToolNodeData>[] = [
  {
    id: '1',
    type: 'agent',
    position: { x: 250, y: 50 },
    data: {
      name: 'Assistant',
      systemPrompt: 'You are a helpful assistant. When users ask about jobs, use the JobSearch tool to find relevant listings. Format the results in a clear, readable way.',
      provider: 'default',
      model: 'llama3.2',
      routing: true,
      routingBehavior: 'best',
      temperature: 0.3,
      maxTokens: 1000,
    },
  },
  {
    id: '2',
    type: 'tool',
    position: { x: 100, y: 250 },
    data: {
      name: 'JobSearch',
      description: 'Search for job listings. Can search by keywords, location, and job type.',
      endpointType: 'mcp',
      endpointUrl: 'https://joblyst.sascha10k.biz/mcp',
      endpointMethod: 'POST',
      mcpToolName: 'search_jobs',
      headers: {},
      bodyTemplate: '',
      parameters: '{\n  "type": "object",\n  "properties": {\n    "query": {\n      "type": "string",\n      "description": "Search keywords (e.g., software engineer, data scientist)"\n    },\n    "location": {\n      "type": "string",\n      "description": "Job location (e.g., Berlin, Remote)"\n    }\n  },\n  "required": ["query"]\n}',
      extractPath: '',
      responseFormat: 'json',
      timeoutSecs: 30,
    },
  },
  {
    id: '3',
    type: 'agent',
    position: { x: 400, y: 250 },
    data: {
      name: 'Analyst',
      systemPrompt: 'You analyze information and provide insights.',
      provider: 'default',
      model: 'llama3.2',
      routing: false,
      routingBehavior: 'best',
      temperature: 0.5,
      maxTokens: 1500,
    },
  },
];

const initialEdges: Edge[] = [
  { id: 'e1-2', source: '1', target: '2', animated: true },
  { id: 'e1-3', source: '1', target: '3', animated: true },
];

export default function EditorPage() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Modal state - separate for agents and tools
  const [agentModalOpen, setAgentModalOpen] = useState(false);
  const [toolModalOpen, setToolModalOpen] = useState(false);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  // Chat panel state
  const [chatOpen, setChatOpen] = useState(false);
  const [systemName, setSystemName] = useState('my-agent-system');

  // File input ref for JSON import
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Node types configuration - memoized to prevent re-renders
  const nodeTypes: NodeTypes = useMemo(() => ({
    agent: AgentNode,
    tool: ToolNode,
  }), []);

  // Get selected node
  const selectedNode = nodes.find((n) => n.id === selectedNodeId);
  const selectedAgentData = selectedNode?.type === 'agent' ? selectedNode.data as AgentNodeData : null;
  const selectedToolData = selectedNode?.type === 'tool' ? selectedNode.data as ToolNodeData : null;

  // Handle new connections with validation
  const onConnect = useCallback(
    (connection: Connection) => {
      // Find the source node
      const sourceNode = nodes.find((n) => n.id === connection.source);

      // Tools cannot be sources (they don't initiate connections)
      if (sourceNode?.type === 'tool') {
        console.warn('Tools cannot initiate connections');
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

  // Delete node (agent or tool)
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
    setSelectedNodeId(null);
  }, [selectedNodeId, setNodes, setEdges]);

  // Export to API format
  const exportConfig = useCallback((): SystemConfigJson => {
    // Separate agents and tools
    const agentNodes = nodes.filter((n) => n.type === 'agent');
    const toolNodes = nodes.filter((n) => n.type === 'tool');

    // Create name-to-id mapping for all nodes
    const nameToId: Record<string, string> = {};
    nodes.forEach((node) => {
      nameToId[node.data.name as string] = node.id;
    });

    // Convert agent nodes to AgentConfig
    const agents: AgentConfig[] = agentNodes.map((node) => {
      const data = node.data as AgentNodeData;
      const connections: Record<string, { type: 'blocking' | 'notify'; timeout_secs?: number }> = {};

      // Find all edges where this node is the source
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
        },
        connections: Object.keys(connections).length > 0 ? connections : undefined,
      };
    });

    // Convert tool nodes to ToolConfig
    const tools: ToolConfig[] = toolNodes.map((node) => {
      const data = node.data as ToolNodeData;

      // Parse JSON strings back to objects
      let parameters: Record<string, unknown> = {};
      let bodyTemplate: Record<string, unknown> | undefined;

      try {
        if (data.parameters) {
          parameters = JSON.parse(data.parameters);
        }
      } catch {
        console.warn('Failed to parse parameters JSON for tool:', data.name);
      }

      // Only parse body template for HTTP endpoints
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
      tools: tools.length > 0 ? tools : undefined,
    };
  }, [nodes, edges]);

  // Handle export button click
  const handleExport = useCallback(() => {
    const config = exportConfig();
    const json = JSON.stringify(config, null, 2);

    // Copy to clipboard
    navigator.clipboard.writeText(json).then(() => {
      alert('Configuration copied to clipboard!');
    });

    console.log('Exported configuration:', config);
  }, [exportConfig]);

  // Import JSON configuration file
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

        // Create a map from name to node ID (for both agents and tools)
        const nameToId: Record<string, string> = {};

        // Calculate grid positions
        const totalItems = config.agents.length + (config.tools?.length || 0);
        const cols = Math.ceil(Math.sqrt(totalItems));
        const spacingX = 250;
        const spacingY = 200;

        let itemIndex = 0;

        // Convert agents to nodes
        const agentNodes: Node<AgentNodeData>[] = config.agents.map((agent) => {
          const nodeId = `imported-agent-${Date.now()}-${itemIndex}`;
          nameToId[agent.name] = nodeId;

          const row = Math.floor(itemIndex / cols);
          const col = itemIndex % cols;
          itemIndex++;

          return {
            id: nodeId,
            type: 'agent',
            position: {
              x: 100 + col * spacingX,
              y: 50 + row * spacingY,
            },
            data: {
              name: agent.name,
              systemPrompt: agent.system_prompt || 'You are a helpful assistant.',
              provider: agent.handler?.provider || 'default',
              model: agent.handler?.model || 'llama3.2',
              routing: agent.handler?.routing || false,
              routingBehavior: (agent.handler?.routing_behavior as RoutingBehavior) || 'best',
              temperature: agent.handler?.options?.temperature ?? 0.7,
              maxTokens: agent.handler?.options?.max_tokens ?? 1000,
            },
          };
        });

        // Convert tools to nodes
        const toolNodes: Node<ToolNodeData>[] = (config.tools || []).map((tool) => {
          const nodeId = `imported-tool-${Date.now()}-${itemIndex}`;
          nameToId[tool.name] = nodeId;

          const row = Math.floor(itemIndex / cols);
          const col = itemIndex % cols;
          itemIndex++;

          // Determine endpoint type from config
          const endpointType = tool.endpoint.type || (tool.endpoint.mcp_tool_name ? 'mcp' : 'http');

          return {
            id: nodeId,
            type: 'tool',
            position: {
              x: 100 + col * spacingX,
              y: 50 + row * spacingY,
            },
            data: {
              name: tool.name,
              description: tool.description,
              endpointType: endpointType,
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

        // Combine all nodes
        const newNodes = [...agentNodes, ...toolNodes];

        // Convert connections to edges
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

        // Update the editor with imported data
        setNodes(newNodes);
        setEdges(newEdges);

        // Update system name from file name (remove .json extension)
        const baseName = file.name.replace(/\.json$/i, '');
        setSystemName(baseName);

        console.log('Imported configuration:', config);
        alert(`Imported ${agentNodes.length} agents, ${toolNodes.length} tools, and ${newEdges.length} connections`);
      } catch (error) {
        console.error('Failed to parse JSON:', error);
        alert('Failed to parse JSON file. Please ensure it is valid JSON.');
      }
    };

    reader.readAsText(file);
    // Reset file input so the same file can be imported again
    event.target.value = '';
  }, [setNodes, setEdges]);

  // Get current config for chat
  const currentConfig = useMemo(() => exportConfig(), [exportConfig]);

  return (
    <div className="w-screen h-screen">
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
            if (node.type === 'tool') return '#f59e0b'; // amber for tools
            if ((node.data as AgentNodeData)?.routing) return '#a855f7'; // purple for routing agents
            return '#3b82f6'; // blue for regular agents
          }}
          maskColor="rgba(0, 0, 0, 0.1)"
        />

        {/* Hidden file input for JSON import */}
        <input
          type="file"
          ref={fileInputRef}
          onChange={handleImport}
          accept=".json,application/json"
          className="hidden"
        />

        {/* Toolbar */}
        <Panel position="top-left" className="flex gap-2">
          <button
            onClick={addAgent}
            className="flex items-center gap-2 px-4 py-2 bg-blue-500 text-white text-sm font-medium rounded-lg shadow-md hover:bg-blue-600 transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
            </svg>
            Add Agent
          </button>
          <button
            onClick={addTool}
            className="flex items-center gap-2 px-4 py-2 bg-amber-500 text-white text-sm font-medium rounded-lg shadow-md hover:bg-amber-600 transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
              />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
            Add Tool
          </button>
          <button
            onClick={() => fileInputRef.current?.click()}
            className="flex items-center gap-2 px-4 py-2 bg-zinc-600 text-white text-sm font-medium rounded-lg shadow-md hover:bg-zinc-700 transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m4-8l-4 4m0 0l-4-4m4 4V4" />
            </svg>
            Import JSON
          </button>
          <button
            onClick={handleExport}
            className="flex items-center gap-2 px-4 py-2 bg-zinc-700 text-white text-sm font-medium rounded-lg shadow-md hover:bg-zinc-800 transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
            </svg>
            Export JSON
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

        {/* Help text */}
        <Panel position="bottom-center" className="text-xs text-zinc-400 bg-zinc-800/90 px-3 py-1.5 rounded-lg">
          Double-click to edit • Drag to connect • Delete/Backspace to remove
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
