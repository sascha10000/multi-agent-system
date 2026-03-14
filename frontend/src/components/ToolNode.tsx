'use client';

import { memo } from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import type { ToolNodeData } from '../types/agent';

interface ToolNodeProps extends NodeProps {
  data: ToolNodeData;
}

function ToolNode({ data, selected }: ToolNodeProps) {
  return (
    <div
      className={`
        min-w-[200px] max-w-[260px] rounded-xl border shadow-lg transition-all
        bg-zinc-900/95 backdrop-blur-sm
        ${selected
          ? 'border-amber-500/70 shadow-amber-500/10 ring-1 ring-amber-500/20'
          : 'border-zinc-700/60 hover:border-zinc-600'}
        hover:shadow-xl
      `}
    >
      {/* Input Handle (top) - Tools only receive, they don't initiate */}
      <Handle
        type="target"
        position={Position.Top}
        className="!w-2.5 !h-2.5 !bg-amber-400 !border-[1.5px] !border-zinc-900 !-top-[5px]"
      />

      {/* Header */}
      <div className="px-3 py-2.5 border-b border-zinc-700/50 bg-amber-900/20 rounded-t-xl">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0">
            <div className="w-5 h-5 rounded-md bg-amber-500/15 flex items-center justify-center flex-shrink-0">
              <svg
                className="w-3 h-3 text-amber-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                />
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                />
              </svg>
            </div>
            <h3 className="font-semibold text-zinc-100 truncate text-sm leading-tight">
              {data.name}
            </h3>
          </div>
          <span className="inline-flex items-center px-1.5 py-0.5 text-[10px] font-semibold bg-amber-500/15 text-amber-300 rounded-md border border-amber-500/20">
            Tool
          </span>
        </div>
      </div>

      {/* Body */}
      <div className="px-3 py-2">
        <p className="text-[10px] text-zinc-400 truncate leading-relaxed" title={data.description}>
          {data.description}
        </p>
        <div className="flex items-center gap-1.5 mt-1.5">
          {data.endpointType === 'mcp' ? (
            <>
              <span className="inline-flex items-center px-1.5 py-0.5 text-[9px] font-mono font-semibold bg-purple-500/15 text-purple-300 rounded-md border border-purple-500/20">
                MCP
              </span>
              <p className="text-[10px] text-zinc-500 truncate flex-1 font-mono" title={data.mcpToolName}>
                {data.mcpToolName || 'unnamed'}
              </p>
            </>
          ) : (
            <>
              <span className="inline-flex items-center px-1.5 py-0.5 text-[9px] font-mono font-semibold bg-zinc-700/60 text-zinc-300 rounded-md border border-zinc-600/40">
                {data.endpointMethod}
              </span>
              <p className="text-[10px] text-zinc-500 truncate flex-1 font-mono" title={data.endpointUrl}>
                {(() => {
                  try {
                    return new URL(data.endpointUrl).hostname;
                  } catch {
                    return data.endpointUrl;
                  }
                })()}
              </p>
            </>
          )}
        </div>
      </div>

      {/* Note: Tools don't have output handles - they only receive and respond */}
    </div>
  );
}

export default memo(ToolNode);
