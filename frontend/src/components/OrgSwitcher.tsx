'use client';

import { useState, useEffect, useRef, useCallback } from 'react';
import { authFetch, getActiveOrg, setActiveOrg, type OrgWithRole } from '../lib/auth';

const API_BASE = process.env.NEXT_PUBLIC_API_BASE || '/api/v1';

interface OrgSwitcherProps {
  onOrgChange?: (orgId: string) => void;
}

export default function OrgSwitcher({ onOrgChange }: OrgSwitcherProps) {
  const [orgs, setOrgs] = useState<OrgWithRole[]>([]);
  const [activeOrgId, setActiveOrgId] = useState<string | null>(getActiveOrg());
  const [open, setOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState('');
  const [newSlug, setNewSlug] = useState('');
  const dropdownRef = useRef<HTMLDivElement>(null);

  const fetchOrgs = useCallback(async () => {
    try {
      const res = await authFetch(`${API_BASE}/orgs`);
      if (res.ok) {
        const data: OrgWithRole[] = await res.json();
        setOrgs(data);

        // Auto-select first org if none active
        if (!activeOrgId && data.length > 0) {
          const firstId = data[0].id;
          setActiveOrgId(firstId);
          setActiveOrg(firstId);
          onOrgChange?.(firstId);
        }
      }
    } catch {
      // Silently fail
    }
  }, [activeOrgId, onOrgChange]);

  useEffect(() => {
    fetchOrgs();
  }, [fetchOrgs]);

  // Close dropdown on outside click
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setOpen(false);
        setCreating(false);
      }
    };
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, []);

  const handleSelect = (orgId: string) => {
    setActiveOrgId(orgId);
    setActiveOrg(orgId);
    setOpen(false);
    onOrgChange?.(orgId);
  };

  const handleCreate = async () => {
    if (!newName.trim() || !newSlug.trim()) return;

    try {
      const res = await authFetch(`${API_BASE}/orgs`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: newName, slug: newSlug }),
      });

      if (res.ok) {
        const data = await res.json();
        setCreating(false);
        setNewName('');
        setNewSlug('');
        await fetchOrgs();
        handleSelect(data.org?.id || data.id);
      }
    } catch {
      // Silently fail
    }
  };

  const activeOrg = orgs.find((o) => o.id === activeOrgId);

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-2 px-3 py-1.5 bg-zinc-800 border border-zinc-700 rounded-md hover:bg-zinc-700 transition-colors text-sm"
      >
        <svg className="w-4 h-4 text-zinc-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" />
        </svg>
        <span className="text-zinc-200 max-w-[150px] truncate">
          {activeOrg?.name || 'Select org'}
        </span>
        <svg className="w-3 h-3 text-zinc-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {open && (
        <div className="absolute top-full left-0 mt-1 w-64 bg-zinc-900 border border-zinc-700 rounded-lg shadow-lg z-50">
          <div className="py-1">
            {orgs.map((org) => (
              <button
                key={org.id}
                onClick={() => handleSelect(org.id)}
                className={`w-full text-left px-3 py-2 text-sm hover:bg-zinc-800 flex items-center justify-between ${
                  org.id === activeOrgId ? 'bg-zinc-800 text-white' : 'text-zinc-300'
                }`}
              >
                <span className="truncate">{org.name}</span>
                <span className="text-xs text-zinc-500 ml-2 flex-shrink-0">{org.role}</span>
              </button>
            ))}
          </div>

          <div className="border-t border-zinc-700 p-2">
            {creating ? (
              <div className="space-y-2">
                <input
                  type="text"
                  value={newName}
                  onChange={(e) => {
                    setNewName(e.target.value);
                    setNewSlug(e.target.value.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, ''));
                  }}
                  placeholder="Organization name"
                  className="w-full px-2 py-1.5 bg-zinc-800 border border-zinc-700 rounded text-sm text-white placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                  autoFocus
                />
                <input
                  type="text"
                  value={newSlug}
                  onChange={(e) => setNewSlug(e.target.value)}
                  placeholder="slug"
                  className="w-full px-2 py-1.5 bg-zinc-800 border border-zinc-700 rounded text-sm text-white placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                />
                <div className="flex gap-2">
                  <button
                    onClick={handleCreate}
                    className="flex-1 px-2 py-1.5 bg-blue-600 text-white text-xs rounded hover:bg-blue-700"
                  >
                    Create
                  </button>
                  <button
                    onClick={() => { setCreating(false); setNewName(''); setNewSlug(''); }}
                    className="px-2 py-1.5 text-zinc-400 text-xs hover:text-zinc-200"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            ) : (
              <button
                onClick={() => setCreating(true)}
                className="w-full text-left px-2 py-1.5 text-sm text-zinc-400 hover:text-zinc-200 flex items-center gap-2"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                </svg>
                New Organization
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
