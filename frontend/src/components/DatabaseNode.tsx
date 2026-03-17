'use client';

import { memo } from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import type { DatabaseNodeData } from '../types/agent';

interface DatabaseNodeProps extends NodeProps {
  data: DatabaseNodeData;
}

function DatabaseNode({ data, selected }: DatabaseNodeProps) {
  // Extract a safe display string from the connection string (hide passwords)
  const displayConnection = (() => {
    try {
      const cs = data.connectionString;
      // For sqlite, just show the file path
      if (cs.startsWith('sqlite://')) return cs.replace('sqlite://', '');
      // For postgres/mysql, show host/db only (strip user:pass)
      const match = cs.match(/:\/\/(?:[^@]+@)?(.+)/);
      return match ? match[1] : cs;
    } catch {
      return data.connectionString;
    }
  })();

  return (
    <div
      className={`
        min-w-[200px] max-w-[260px] rounded-xl border shadow-lg transition-all
        bg-zinc-900/95 backdrop-blur-sm
        ${selected
          ? 'border-cyan-500/70 shadow-cyan-500/10 ring-1 ring-cyan-500/20'
          : 'border-zinc-700/60 hover:border-zinc-600'}
        hover:shadow-xl
      `}
    >
      {/* Input Handle (top) - Databases only receive, they don't initiate */}
      <Handle
        type="target"
        position={Position.Top}
        className="!w-2.5 !h-2.5 !bg-cyan-400 !border-[1.5px] !border-zinc-900 !-top-[5px]"
      />

      {/* Header */}
      <div className="px-3 py-2.5 border-b border-zinc-700/50 bg-cyan-900/20 rounded-t-xl">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0">
            <div className="w-5 h-5 rounded-md bg-cyan-500/15 flex items-center justify-center flex-shrink-0">
              {/* Database/cylinder icon */}
              <svg
                className="w-3 h-3 text-cyan-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <ellipse cx="12" cy="5" rx="9" ry="3" strokeWidth={2} />
                <path strokeWidth={2} d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5" />
                <path strokeWidth={2} d="M3 12c0 1.66 4.03 3 9 3s9-1.34 9-3" />
              </svg>
            </div>
            <h3 className="font-semibold text-zinc-100 truncate text-sm leading-tight">
              {data.name}
            </h3>
          </div>
          <span className="inline-flex items-center px-1.5 py-0.5 text-[10px] font-semibold bg-cyan-500/15 text-cyan-300 rounded-md border border-cyan-500/20">
            DB
          </span>
        </div>
      </div>

      {/* Body */}
      <div className="px-3 py-2">
        {data.description && (
          <p className="text-[10px] text-zinc-400 truncate leading-relaxed" title={data.description}>
            {data.description}
          </p>
        )}
        <div className="flex items-center gap-1.5 mt-1.5">
          <span className="inline-flex items-center px-1.5 py-0.5 text-[9px] font-mono font-semibold bg-cyan-500/15 text-cyan-300 rounded-md border border-cyan-500/20 uppercase">
            {data.databaseType}
          </span>
          <p className="text-[10px] text-zinc-500 truncate flex-1 font-mono" title={data.connectionString}>
            {displayConnection}
          </p>
        </div>
        <div className="flex items-center gap-2 mt-1">
          {data.readOnly && (
            <span className="text-[9px] text-emerald-400/70 font-medium">read-only</span>
          )}
        </div>
      </div>

      {/* Note: Databases don't have output handles - they only receive and respond */}
    </div>
  );
}

export default memo(DatabaseNode);
