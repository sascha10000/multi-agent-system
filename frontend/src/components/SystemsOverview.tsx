'use client';

import { useState, useEffect, useCallback } from 'react';
import type { SystemConfigJson } from '../types/agent';
import type { SystemSummary, ListSystemsResponse, SystemConfigResponse } from '../types/api';
import { authFetch } from '../lib/auth';

const API_BASE = process.env.NEXT_PUBLIC_API_BASE || '/api/v1';

interface SystemsOverviewProps {
  onSelectSystem: (name: string, config: SystemConfigJson) => void;
  onNewSystem: () => void;
  onImportJson: () => void;
  orgId?: string | null;
}

export default function SystemsOverview({ onSelectSystem, onNewSystem, onImportJson, orgId }: SystemsOverviewProps) {
  const [systems, setSystems] = useState<SystemSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deletingName, setDeletingName] = useState<string | null>(null);
  const [openingName, setOpeningName] = useState<string | null>(null);

  const fetchSystems = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const url = orgId ? `${API_BASE}/systems?org_id=${encodeURIComponent(orgId)}` : `${API_BASE}/systems`;
      const res = await authFetch(url);
      if (!res.ok) throw new Error('Failed to fetch systems');
      const data: ListSystemsResponse = await res.json();
      // Sort by created_at descending
      const sorted = [...data.systems].sort(
        (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      );
      setSystems(sorted);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load systems');
      setSystems([]);
    } finally {
      setLoading(false);
    }
  }, [orgId]);

  useEffect(() => {
    fetchSystems();
  }, [fetchSystems]);

  const handleOpen = useCallback(async (name: string) => {
    setOpeningName(name);
    setError(null);
    try {
      const res = await authFetch(`${API_BASE}/systems/${encodeURIComponent(name)}/config`);
      if (!res.ok) throw new Error(`Failed to load system "${name}"`);
      const data: SystemConfigResponse = await res.json();
      onSelectSystem(name, data.config);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to open system');
    } finally {
      setOpeningName(null);
    }
  }, [onSelectSystem]);

  const handleDelete = useCallback(async (name: string) => {
    if (!confirm(`Delete system "${name}"? This cannot be undone.`)) return;
    setDeletingName(name);
    try {
      const res = await authFetch(`${API_BASE}/systems/${encodeURIComponent(name)}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error(`Failed to delete system "${name}"`);
      setSystems((prev) => prev.filter((s) => s.name !== name));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete system');
    } finally {
      setDeletingName(null);
    }
  }, []);

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;

    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  };

  return (
    <div className="min-h-screen bg-zinc-950 text-zinc-100">
      {/* Header */}
      <div className="border-b border-zinc-800 bg-zinc-900/50">
        <div className="max-w-5xl mx-auto px-6 py-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold text-zinc-100">Multi-Agent Systems</h1>
              <p className="text-sm text-zinc-400 mt-1">Create, manage, and chat with your agent systems</p>
            </div>
            <div className="flex gap-3">
              <button
                onClick={onImportJson}
                className="flex items-center gap-2 px-4 py-2.5 bg-zinc-800 text-zinc-200 text-sm font-medium rounded-lg border border-zinc-700 hover:bg-zinc-700 hover:border-zinc-600 transition-colors"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m4-8l-4 4m0 0l-4-4m4 4V4" />
                </svg>
                Import JSON
              </button>
              <button
                onClick={onNewSystem}
                className="flex items-center gap-2 px-4 py-2.5 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 transition-colors"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                </svg>
                New System
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Error banner */}
      {error && (
        <div className="max-w-5xl mx-auto px-6 pt-4">
          <div className="px-4 py-3 bg-red-900/30 border border-red-800/50 rounded-lg flex items-center justify-between">
            <p className="text-red-300 text-sm">{error}</p>
            <button onClick={() => setError(null)} className="text-red-400 hover:text-red-300 ml-4">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>
      )}

      {/* Content */}
      <div className="max-w-5xl mx-auto px-6 py-8">
        {loading ? (
          <div className="flex flex-col items-center justify-center py-20">
            <svg className="w-8 h-8 animate-spin text-zinc-500 mb-4" fill="none" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
            </svg>
            <p className="text-zinc-500">Loading systems...</p>
          </div>
        ) : systems.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <div className="w-16 h-16 rounded-full bg-zinc-800 flex items-center justify-center mb-6">
              <svg className="w-8 h-8 text-zinc-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" />
              </svg>
            </div>
            <h2 className="text-xl font-semibold text-zinc-200 mb-2">No systems yet</h2>
            <p className="text-zinc-400 mb-8 max-w-sm">
              Create a new multi-agent system or import an existing JSON configuration to get started.
            </p>
            <div className="flex gap-3">
              <button
                onClick={onImportJson}
                className="flex items-center gap-2 px-4 py-2.5 bg-zinc-800 text-zinc-200 text-sm font-medium rounded-lg border border-zinc-700 hover:bg-zinc-700 transition-colors"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m4-8l-4 4m0 0l-4-4m4 4V4" />
                </svg>
                Import JSON
              </button>
              <button
                onClick={onNewSystem}
                className="flex items-center gap-2 px-4 py-2.5 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 transition-colors"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                </svg>
                New System
              </button>
            </div>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {systems.map((system) => (
              <div
                key={system.name}
                className="group bg-zinc-900 border border-zinc-800 rounded-xl p-5 hover:border-zinc-600 hover:bg-zinc-800/50 transition-all cursor-pointer"
                onClick={() => handleOpen(system.name)}
              >
                <div className="flex items-start justify-between mb-3">
                  <h3 className="text-base font-semibold text-zinc-100 truncate flex-1 mr-2">
                    {system.name}
                  </h3>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDelete(system.name);
                    }}
                    disabled={deletingName === system.name}
                    className="p-1.5 text-zinc-500 hover:text-red-400 hover:bg-red-900/20 rounded opacity-0 group-hover:opacity-100 transition-all disabled:opacity-50 flex-shrink-0"
                    title="Delete system"
                  >
                    {deletingName === system.name ? (
                      <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                        <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                      </svg>
                    ) : (
                      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                    )}
                  </button>
                </div>

                <div className="flex items-center gap-2 text-sm text-zinc-400 mb-3">
                  <svg className="w-4 h-4 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z" />
                  </svg>
                  <span>{system.agent_count} agent{system.agent_count !== 1 ? 's' : ''}</span>
                </div>

                {/* Agent name chips */}
                <div className="flex flex-wrap gap-1.5 mb-4">
                  {system.agents.slice(0, 5).map((agentName) => (
                    <span
                      key={agentName}
                      className="inline-flex items-center gap-1 px-2 py-0.5 text-[11px] font-medium bg-zinc-800/80 text-zinc-300 rounded-md border border-zinc-700/60"
                    >
                      <span className="w-1.5 h-1.5 rounded-full bg-blue-400/60 flex-shrink-0" />
                      {agentName}
                    </span>
                  ))}
                  {system.agents.length > 5 && (
                    <span className="inline-flex items-center px-2 py-0.5 text-[11px] text-zinc-500 font-medium">
                      +{system.agents.length - 5} more
                    </span>
                  )}
                </div>

                <div className="flex items-center justify-between">
                  <span className="text-xs text-zinc-500">{formatDate(system.created_at)}</span>
                  {openingName === system.name ? (
                    <svg className="w-4 h-4 animate-spin text-blue-400" fill="none" viewBox="0 0 24 24">
                      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                      <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                    </svg>
                  ) : (
                    <svg className="w-4 h-4 text-zinc-600 group-hover:text-zinc-400 transition-colors" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                    </svg>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
