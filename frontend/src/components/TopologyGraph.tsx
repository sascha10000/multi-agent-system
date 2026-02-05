import { useCallback, useMemo } from 'react';
import {
  ReactFlow,
  Controls,
  Background,
  useNodesState,
  useEdgesState,
  type Edge,
  BackgroundVariant,
  MarkerType,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import AgentNodeComponent, { type AgentNode } from './AgentNode';
import type { AgentInfo } from '../types/api';

interface TopologyGraphProps {
  agents: AgentInfo[];
}

const nodeTypes = {
  agent: AgentNodeComponent,
};

function agentsToFlow(agents: AgentInfo[]): { nodes: AgentNode[]; edges: Edge[] } {
  // Calculate positions in a grid layout
  const cols = Math.max(3, Math.ceil(Math.sqrt(agents.length)));
  const spacing = { x: 220, y: 180 };

  const nodes: AgentNode[] = agents.map((agent, i) => ({
    id: agent.name,
    type: 'agent' as const,
    position: {
      x: (i % cols) * spacing.x + 50,
      y: Math.floor(i / cols) * spacing.y + 50,
    },
    data: {
      label: agent.name,
      role: agent.role,
      routing: agent.routing,
    },
  }));

  const edges: Edge[] = agents.flatMap((agent) =>
    agent.connections.map((conn) => ({
      id: `${agent.name}-${conn.target}`,
      source: agent.name,
      target: conn.target,
      animated: conn.connection_type === 'notify',
      label: conn.connection_type,
      style: {
        stroke: conn.connection_type === 'notify' ? '#f59e0b' : '#6366f1',
        strokeWidth: 2,
      },
      markerEnd: {
        type: MarkerType.ArrowClosed,
        color: conn.connection_type === 'notify' ? '#f59e0b' : '#6366f1',
      },
    }))
  );

  return { nodes, edges };
}

function TopologyGraph({ agents }: TopologyGraphProps) {
  const { nodes: initialNodes, edges: initialEdges } = useMemo(
    () => agentsToFlow(agents),
    [agents]
  );

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Reset nodes/edges when agents change
  const resetLayout = useCallback(() => {
    const { nodes: newNodes, edges: newEdges } = agentsToFlow(agents);
    setNodes(newNodes);
    setEdges(newEdges);
  }, [agents, setNodes, setEdges]);

  return (
    <div className="topology-graph">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.2}
        maxZoom={2}
      >
        <Background variant={BackgroundVariant.Dots} gap={20} size={1} />
        <Controls />
      </ReactFlow>
      <button onClick={resetLayout} className="topology-reset-btn">
        Reset Layout
      </button>
    </div>
  );
}

export default TopologyGraph;
