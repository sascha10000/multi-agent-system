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
import ChatPanel from '../components/ChatPanel';
import type { AgentNodeData, SystemConfigJson, AgentConfig, RoutingBehavior } from '../types/agent';

// Initial nodes for demo
const initialNodes: Node<AgentNodeData>[] = [
  {
    id: '1',
    type: 'agent',
    position: { x: 250, y: 50 },
    data: {
      name: 'Coordinator',
      systemPrompt: 'You coordinate work between team members.',
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
    type: 'agent',
    position: { x: 100, y: 250 },
    data: {
      name: 'Researcher',
      systemPrompt: 'You are an expert researcher.',
      provider: 'default',
      model: 'llama3.2',
      routing: false,
      routingBehavior: 'best',
      temperature: 0.7,
      maxTokens: 2000,
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

  // Modal state
  const [modalOpen, setModalOpen] = useState(false);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  // Chat panel state
  const [chatOpen, setChatOpen] = useState(false);
  const [systemName, setSystemName] = useState('my-agent-system');

  // File input ref for JSON import
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Node types configuration - memoized to prevent re-renders
  const nodeTypes: NodeTypes = useMemo(() => ({ agent: AgentNode }), []);

  // Get selected node data
  const selectedNode = nodes.find((n) => n.id === selectedNodeId);
  const selectedAgentData = selectedNode?.data || null;

  // Handle new connections
  const onConnect = useCallback(
    (connection: Connection) => {
      setEdges((eds) => addEdge({ ...connection, animated: true }, eds));
    },
    [setEdges]
  );

  // Handle node double-click to edit
  const onNodeDoubleClick = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      setSelectedNodeId(node.id);
      setModalOpen(true);
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
    // Open modal for the new node
    setSelectedNodeId(newId);
    setModalOpen(true);
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
      setModalOpen(false);
      setSelectedNodeId(null);
    },
    [selectedNodeId, setNodes]
  );

  // Delete agent
  const handleDeleteAgent = useCallback(() => {
    if (!selectedNodeId) return;
    setNodes((nds) => nds.filter((node) => node.id !== selectedNodeId));
    setEdges((eds) =>
      eds.filter(
        (edge) => edge.source !== selectedNodeId && edge.target !== selectedNodeId
      )
    );
    setModalOpen(false);
    setSelectedNodeId(null);
  }, [selectedNodeId, setNodes, setEdges]);

  // Export to API format
  const exportConfig = useCallback((): SystemConfigJson => {
    const agents: AgentConfig[] = nodes.map((node) => {
      const data = node.data;
      const connections: Record<string, { type: 'blocking' | 'notify'; timeout_secs?: number }> = {};

      // Find all edges where this node is the source
      edges
        .filter((edge) => edge.source === node.id)
        .forEach((edge) => {
          const targetNode = nodes.find((n) => n.id === edge.target);
          if (targetNode) {
            connections[targetNode.data.name] = {
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

        // Create a map from agent name to node ID
        const agentNameToId: Record<string, string> = {};

        // Calculate grid positions for agents
        const cols = Math.ceil(Math.sqrt(config.agents.length));
        const spacingX = 250;
        const spacingY = 200;

        // Convert agents to nodes
        const newNodes: Node<AgentNodeData>[] = config.agents.map((agent, index) => {
          const nodeId = `imported-${Date.now()}-${index}`;
          agentNameToId[agent.name] = nodeId;

          const row = Math.floor(index / cols);
          const col = index % cols;

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

        // Convert connections to edges
        const newEdges: Edge[] = [];
        config.agents.forEach((agent) => {
          if (agent.connections) {
            const sourceId = agentNameToId[agent.name];
            Object.keys(agent.connections).forEach((targetName) => {
              const targetId = agentNameToId[targetName];
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
        alert(`Imported ${newNodes.length} agents and ${newEdges.length} connections`);
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
            if (node.data?.routing) return '#a855f7';
            return '#3b82f6';
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
            onClick={() => fileInputRef.current?.click()}
            className="flex items-center gap-2 px-4 py-2 bg-amber-600 text-white text-sm font-medium rounded-lg shadow-md hover:bg-amber-700 transition-colors"
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

      {/* Edit Modal */}
      <AgentModal
        isOpen={modalOpen}
        agent={selectedAgentData}
        onSave={handleSaveAgent}
        onDelete={handleDeleteAgent}
        onClose={() => {
          setModalOpen(false);
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
