'use client';

import { memo } from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import type { AgentNodeData } from '../types/agent';

interface AgentNodeProps extends NodeProps {
  data: AgentNodeData;
}

function AgentNode({ data, selected }: AgentNodeProps) {
  return (
    <div
      className={`
        min-w-[180px] rounded-lg border-2 shadow-md transition-all
        bg-zinc-800 dark:bg-zinc-800
        ${selected ? 'border-blue-500 shadow-lg' : 'border-zinc-600 dark:border-zinc-600'}
        hover:shadow-lg
      `}
    >
      {/* Input Handle (top) */}
      <Handle
        type="target"
        position={Position.Top}
        className="!w-3 !h-3 !bg-zinc-500 !border-2 !border-zinc-800"
      />

      {/* Header */}
      <div className="px-3 py-2 border-b border-zinc-700 bg-zinc-700 rounded-t-lg">
        <div className="flex items-center justify-between gap-2">
          <h3 className="font-semibold text-zinc-100 truncate text-sm">
            {data.name}
          </h3>
          {data.routing && (
            <span className="px-1.5 py-0.5 text-[10px] font-medium bg-purple-900 text-purple-300 rounded">
              Router
            </span>
          )}
        </div>
      </div>

      {/* Body */}
      <div className="px-3 py-2">
        <p className="text-xs text-zinc-400 truncate">{data.role}</p>
        {data.routing && (
          <p className="text-[10px] text-zinc-500 mt-1">
            Behavior: {data.routingBehavior}
          </p>
        )}
        <p className="text-[10px] text-zinc-500 mt-1">
          {data.model}
        </p>
      </div>

      {/* Output Handle (bottom) */}
      <Handle
        type="source"
        position={Position.Bottom}
        className="!w-3 !h-3 !bg-blue-500 !border-2 !border-zinc-800"
      />
    </div>
  );
}

export default memo(AgentNode);
