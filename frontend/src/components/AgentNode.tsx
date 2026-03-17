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
        min-w-[200px] max-w-[260px] rounded-xl border shadow-lg transition-all
        bg-zinc-900/95 backdrop-blur-sm
        ${selected
          ? 'border-blue-500/70 shadow-blue-500/10 ring-1 ring-blue-500/20'
          : data.entryPoint
            ? 'border-green-500/50 hover:border-green-400/60'
            : 'border-zinc-700/60 hover:border-zinc-600'}
        hover:shadow-xl
      `}
    >
      {/* Input Handle (top) */}
      <Handle
        type="target"
        position={Position.Top}
        className="!w-2.5 !h-2.5 !bg-zinc-400 !border-[1.5px] !border-zinc-900 !-top-[5px]"
      />

      {/* Header */}
      <div className="px-3 py-2.5 border-b border-zinc-700/50 bg-zinc-800/60 rounded-t-xl">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0">
            <div className="w-5 h-5 rounded-md bg-blue-500/15 flex items-center justify-center flex-shrink-0">
              <svg className="w-3 h-3 text-blue-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
              </svg>
            </div>
            <h3 className="font-semibold text-zinc-100 truncate text-sm leading-tight">
              {data.name}
            </h3>
          </div>
          <div className="flex items-center gap-1">
            {data.entryPoint && (
              <span className="inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] font-semibold bg-green-500/15 text-green-300 rounded-md border border-green-500/20">
                <svg className="w-2.5 h-2.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M13 7l5 5m0 0l-5 5m5-5H6" />
                </svg>
                Entry
              </span>
            )}
          </div>
        </div>
      </div>

      {/* Body */}
      <div className="px-3 py-2">
        <div className="flex items-center gap-1.5 mb-1">
          <span className="text-[10px] text-zinc-300 font-medium bg-zinc-800 px-1.5 py-0.5 rounded">
            {data.routingBehavior}
          </span>
          {data.maxTurns !== 1 && (
            <span className="text-[10px] text-zinc-400 font-medium bg-zinc-800 px-1.5 py-0.5 rounded">
              {data.maxTurns === 0 ? '∞' : data.maxTurns} turns
            </span>
          )}
        </div>
        <p className="text-[10px] text-zinc-500 font-mono truncate">
          {data.model}
        </p>
      </div>

      {/* Output Handle (bottom) */}
      <Handle
        type="source"
        position={Position.Bottom}
        className="!w-2.5 !h-2.5 !bg-blue-400 !border-[1.5px] !border-zinc-900 !-bottom-[5px]"
      />
    </div>
  );
}

export default memo(AgentNode);
