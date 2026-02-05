import { Handle, Position, type NodeProps, type Node } from '@xyflow/react';

export interface AgentNodeData extends Record<string, unknown> {
  label: string;
  role: string;
  routing: boolean;
}

export type AgentNode = Node<AgentNodeData, 'agent'>;

function AgentNode({ data }: NodeProps<AgentNode>) {
  return (
    <div className={`agent-node ${data.routing ? 'agent-node-routing' : ''}`}>
      <Handle type="target" position={Position.Top} />
      <div className="agent-node-header">
        <span className="agent-node-name">{data.label}</span>
        {data.routing && <span className="agent-node-routing-badge">LLM</span>}
      </div>
      <div className="agent-node-role">{data.role}</div>
      <Handle type="source" position={Position.Bottom} />
    </div>
  );
}

export default AgentNode;
