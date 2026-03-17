'use client';

import { memo } from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import type { RestApiNodeData } from '../types/agent';

interface RestApiNodeProps extends NodeProps {
  data: RestApiNodeData;
}

function RestApiNode({ data, selected }: RestApiNodeProps) {
  return (
    <div
      className={`
        min-w-[200px] max-w-[260px] rounded-xl border shadow-lg transition-all
        bg-zinc-900/95 backdrop-blur-sm
        ${selected
          ? 'border-rose-500/70 shadow-rose-500/10 ring-1 ring-rose-500/20'
          : 'border-zinc-700/60 hover:border-zinc-600'}
        hover:shadow-xl
      `}
    >
      {/* Input Handle (top) - REST APIs only receive, they don't initiate */}
      <Handle
        type="target"
        position={Position.Top}
        className="!w-2.5 !h-2.5 !bg-rose-400 !border-[1.5px] !border-zinc-900 !-top-[5px]"
      />

      {/* Header */}
      <div className="px-3 py-2.5 border-b border-zinc-700/50 bg-rose-900/20 rounded-t-xl">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0">
            <div className="w-5 h-5 rounded-md bg-rose-500/15 flex items-center justify-center flex-shrink-0">
              <svg
                className="w-3 h-3 text-rose-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
                />
              </svg>
            </div>
            <h3 className="font-semibold text-zinc-100 truncate text-sm leading-tight">
              {data.name}
            </h3>
          </div>
          <span className="inline-flex items-center px-1.5 py-0.5 text-[10px] font-semibold bg-rose-500/15 text-rose-300 rounded-md border border-rose-500/20">
            REST
          </span>
        </div>
      </div>

      {/* Body */}
      <div className="px-3 py-2">
        <p className="text-[10px] text-zinc-400 truncate leading-relaxed" title={data.description}>
          {data.description}
        </p>
        <div className="flex items-center gap-1.5 mt-1.5">
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
        </div>
      </div>
    </div>
  );
}

export default memo(RestApiNode);
