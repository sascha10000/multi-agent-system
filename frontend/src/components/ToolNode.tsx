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
        min-w-[180px] rounded-lg border-2 shadow-md transition-all
        bg-zinc-800 dark:bg-zinc-800
        ${selected ? 'border-amber-500 shadow-lg' : 'border-zinc-600 dark:border-zinc-600'}
        hover:shadow-lg
      `}
    >
      {/* Input Handle (top) - Tools only receive, they don't initiate */}
      <Handle
        type="target"
        position={Position.Top}
        className="!w-3 !h-3 !bg-amber-500 !border-2 !border-zinc-800"
      />

      {/* Header */}
      <div className="px-3 py-2 border-b border-zinc-700 bg-amber-900/40 rounded-t-lg">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            {/* Tool/Wrench icon */}
            <svg
              className="w-4 h-4 text-amber-400"
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
            <h3 className="font-semibold text-zinc-100 truncate text-sm">
              {data.name}
            </h3>
          </div>
          <span className="px-1.5 py-0.5 text-[10px] font-medium bg-amber-800 text-amber-200 rounded">
            Tool
          </span>
        </div>
      </div>

      {/* Body */}
      <div className="px-3 py-2">
        <p className="text-[10px] text-zinc-400 truncate" title={data.description}>
          {data.description}
        </p>
        <div className="flex items-center gap-1 mt-1">
          {data.endpointType === 'mcp' ? (
            <>
              <span className="px-1 py-0.5 text-[9px] font-mono bg-purple-800 text-purple-200 rounded">
                MCP
              </span>
              <p className="text-[10px] text-zinc-500 truncate flex-1" title={data.mcpToolName}>
                {data.mcpToolName || 'unnamed'}
              </p>
            </>
          ) : (
            <>
              <span className="px-1 py-0.5 text-[9px] font-mono bg-zinc-700 text-zinc-300 rounded">
                {data.endpointMethod}
              </span>
              <p className="text-[10px] text-zinc-500 truncate flex-1" title={data.endpointUrl}>
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
