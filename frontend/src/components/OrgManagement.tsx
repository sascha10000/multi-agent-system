'use client';

import { useState, useEffect, useCallback } from 'react';
import { authFetch } from '../lib/auth';

const API_BASE = process.env.NEXT_PUBLIC_API_BASE || '/api/v1';

interface Member {
  user_id: string;
  email: string;
  display_name: string;
  role: 'owner' | 'admin' | 'member';
  joined_at: string;
}

interface OrgDetails {
  id: string;
  name: string;
  slug: string;
  parent_id: string | null;
  role: 'owner' | 'admin' | 'member';
}

interface OrgManagementProps {
  orgId: string;
  onClose: () => void;
}

export default function OrgManagement({ orgId, onClose }: OrgManagementProps) {
  const [org, setOrg] = useState<OrgDetails | null>(null);
  const [members, setMembers] = useState<Member[]>([]);
  const [systems, setSystems] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [inviteEmail, setInviteEmail] = useState('');
  const [inviteRole, setInviteRole] = useState('member');
  const [inviting, setInviting] = useState(false);
  const [error, setError] = useState('');

  const fetchData = useCallback(async () => {
    setLoading(true);
    try {
      const [orgRes, membersRes, systemsRes] = await Promise.all([
        authFetch(`${API_BASE}/orgs/${orgId}`),
        authFetch(`${API_BASE}/orgs/${orgId}/members`),
        authFetch(`${API_BASE}/orgs/${orgId}/systems`),
      ]);

      if (orgRes.ok) setOrg(await orgRes.json());
      if (membersRes.ok) setMembers(await membersRes.json());
      if (systemsRes.ok) setSystems(await systemsRes.json());
    } catch {
      setError('Failed to load organization data');
    } finally {
      setLoading(false);
    }
  }, [orgId]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleInvite = async () => {
    if (!inviteEmail.trim()) return;
    setInviting(true);
    setError('');

    try {
      const res = await authFetch(`${API_BASE}/orgs/${orgId}/members`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: inviteEmail, role: inviteRole }),
      });

      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        throw new Error(data.error || 'Failed to add member');
      }

      setInviteEmail('');
      fetchData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to add member');
    } finally {
      setInviting(false);
    }
  };

  const handleRemoveMember = async (userId: string) => {
    try {
      await authFetch(`${API_BASE}/orgs/${orgId}/members/${userId}`, {
        method: 'DELETE',
      });
      fetchData();
    } catch {
      setError('Failed to remove member');
    }
  };

  const handleChangeRole = async (userId: string, newRole: string) => {
    try {
      await authFetch(`${API_BASE}/orgs/${orgId}/members/${userId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ role: newRole }),
      });
      fetchData();
    } catch {
      setError('Failed to update role');
    }
  };

  const canManage = org?.role === 'owner' || org?.role === 'admin';

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="bg-zinc-900 border border-zinc-700 rounded-xl w-full max-w-2xl max-h-[80vh] overflow-y-auto">
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-zinc-800">
          <div>
            <h2 className="text-lg font-semibold text-white">
              {org?.name || 'Organization'}
            </h2>
            {org && (
              <p className="text-sm text-zinc-400 mt-0.5">
                {org.slug} &middot; Your role: {org.role}
              </p>
            )}
          </div>
          <button
            onClick={onClose}
            className="p-2 text-zinc-400 hover:text-white rounded-lg hover:bg-zinc-800"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {loading ? (
          <div className="p-10 text-center text-zinc-500">Loading...</div>
        ) : (
          <div className="p-5 space-y-6">
            {error && (
              <div className="text-red-400 text-sm bg-red-500/10 border border-red-500/20 rounded-md px-3 py-2">
                {error}
              </div>
            )}

            {/* Members Section */}
            <div>
              <h3 className="text-sm font-medium text-zinc-300 mb-3">
                Members ({members.length})
              </h3>
              <div className="space-y-2">
                {members.map((member) => (
                  <div
                    key={member.user_id}
                    className="flex items-center justify-between bg-zinc-800/50 rounded-lg px-4 py-3"
                  >
                    <div>
                      <div className="text-sm text-white">{member.display_name}</div>
                      <div className="text-xs text-zinc-500">{member.email}</div>
                    </div>
                    <div className="flex items-center gap-2">
                      {canManage ? (
                        <select
                          value={member.role}
                          onChange={(e) => handleChangeRole(member.user_id, e.target.value)}
                          className="text-xs bg-zinc-700 border border-zinc-600 rounded px-2 py-1 text-zinc-200"
                        >
                          <option value="owner">Owner</option>
                          <option value="admin">Admin</option>
                          <option value="member">Member</option>
                        </select>
                      ) : (
                        <span className="text-xs text-zinc-500 px-2">{member.role}</span>
                      )}
                      {canManage && (
                        <button
                          onClick={() => handleRemoveMember(member.user_id)}
                          className="text-zinc-500 hover:text-red-400 p-1"
                          title="Remove member"
                        >
                          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                          </svg>
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>

              {/* Invite */}
              {canManage && (
                <div className="mt-3 flex gap-2">
                  <input
                    type="email"
                    value={inviteEmail}
                    onChange={(e) => setInviteEmail(e.target.value)}
                    placeholder="Email address"
                    className="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-md text-sm text-white placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                  />
                  <select
                    value={inviteRole}
                    onChange={(e) => setInviteRole(e.target.value)}
                    className="px-2 py-2 bg-zinc-800 border border-zinc-700 rounded-md text-sm text-zinc-200"
                  >
                    <option value="member">Member</option>
                    <option value="admin">Admin</option>
                    <option value="owner">Owner</option>
                  </select>
                  <button
                    onClick={handleInvite}
                    disabled={inviting || !inviteEmail.trim()}
                    className="px-4 py-2 bg-blue-600 text-white text-sm rounded-md hover:bg-blue-700 disabled:opacity-50"
                  >
                    {inviting ? '...' : 'Add'}
                  </button>
                </div>
              )}
            </div>

            {/* Systems Section */}
            <div>
              <h3 className="text-sm font-medium text-zinc-300 mb-3">
                Systems ({systems.length})
              </h3>
              {systems.length === 0 ? (
                <p className="text-sm text-zinc-500">
                  No systems assigned to this organization yet.
                </p>
              ) : (
                <div className="flex flex-wrap gap-2">
                  {systems.map((name) => (
                    <span
                      key={name}
                      className="px-3 py-1.5 text-sm bg-zinc-800 text-zinc-300 rounded-lg border border-zinc-700"
                    >
                      {name}
                    </span>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
