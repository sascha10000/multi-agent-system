'use client';

import { useState, useEffect } from 'react';
import type { DatabaseNodeData, DatabaseType } from '../types/agent';

interface DatabaseModalProps {
  isOpen: boolean;
  database: DatabaseNodeData | null;
  onSave: (data: DatabaseNodeData) => void;
  onDelete: () => void;
  onClose: () => void;
}

const PLACEHOLDER_MAP: Record<DatabaseType, string> = {
  sqlite: 'sqlite://path/to/database.db',
  postgres: 'postgres://user:password@localhost:5432/dbname',
  mysql: 'mysql://user:password@localhost:3306/dbname',
};

export default function DatabaseModal({
  isOpen,
  database,
  onSave,
  onDelete,
  onClose,
}: DatabaseModalProps) {
  const [formData, setFormData] = useState<DatabaseNodeData | null>(null);

  useEffect(() => {
    if (database) {
      setFormData({ ...database });
    }
  }, [database]);

  if (!isOpen || !formData) return null;

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>
  ) => {
    const { name, value, type } = e.target;

    if (type === 'number') {
      setFormData((prev) => prev ? { ...prev, [name]: parseFloat(value) || 0 } : null);
    } else if (type === 'checkbox') {
      const checked = (e.target as HTMLInputElement).checked;
      setFormData((prev) => prev ? { ...prev, [name]: checked } : null);
    } else {
      setFormData((prev) => prev ? { ...prev, [name]: value } : null);
    }
  };

  const handleTypeChange = (dbType: DatabaseType) => {
    setFormData((prev) => {
      if (!prev) return null;
      // Update connection string placeholder if it's still a default/placeholder
      const isDefault = Object.values(PLACEHOLDER_MAP).some((p) => prev.connectionString === p)
        || prev.connectionString === 'sqlite://data.db';
      return {
        ...prev,
        databaseType: dbType,
        connectionString: isDefault ? PLACEHOLDER_MAP[dbType] : prev.connectionString,
      };
    });
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formData) return;

    if (!formData.connectionString.trim()) {
      alert('Connection string is required');
      return;
    }

    onSave(formData);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70">
      <div className="bg-zinc-900 rounded-xl shadow-2xl w-full max-w-2xl max-h-[90vh] overflow-y-auto border border-zinc-700">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-zinc-700 bg-cyan-900/20">
          <div className="flex items-center gap-2">
            <svg
              className="w-5 h-5 text-cyan-400"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <ellipse cx="12" cy="5" rx="9" ry="3" strokeWidth={2} />
              <path strokeWidth={2} d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5" />
              <path strokeWidth={2} d="M3 12c0 1.66 4.03 3 9 3s9-1.34 9-3" />
            </svg>
            <h2 className="text-lg font-semibold text-zinc-100">Edit Database</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 text-zinc-400 hover:text-zinc-200 transition-colors"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          {/* Basic Info */}
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">
                Name
              </label>
              <input
                type="text"
                name="name"
                value={formData.name}
                onChange={handleChange}
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-cyan-500 focus:border-cyan-500 outline-none"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">
                Description
              </label>
              <textarea
                name="description"
                value={formData.description}
                onChange={handleChange}
                rows={2}
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-cyan-500 focus:border-cyan-500 outline-none resize-none"
                placeholder="Describe the data in this database..."
              />
            </div>
          </div>

          {/* Connection Settings */}
          <div className="pt-4 border-t border-zinc-700">
            <h3 className="text-sm font-medium text-zinc-200 mb-3">Connection</h3>

            <div className="space-y-4">
              {/* Database Type Selector */}
              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Database Type
                </label>
                <div className="flex gap-2">
                  {(['sqlite', 'postgres', 'mysql'] as DatabaseType[]).map((dbType) => (
                    <button
                      key={dbType}
                      type="button"
                      onClick={() => handleTypeChange(dbType)}
                      className={`flex-1 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                        formData.databaseType === dbType
                          ? 'bg-cyan-600 text-white'
                          : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
                      }`}
                    >
                      {dbType === 'sqlite' ? 'SQLite' : dbType === 'postgres' ? 'PostgreSQL' : 'MySQL'}
                    </button>
                  ))}
                </div>
              </div>

              {/* Connection String */}
              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Connection String
                </label>
                <input
                  type="text"
                  name="connectionString"
                  value={formData.connectionString}
                  onChange={handleChange}
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-cyan-500 focus:border-cyan-500 outline-none font-mono text-sm"
                  placeholder={PLACEHOLDER_MAP[formData.databaseType]}
                  required
                />
                <p className="mt-1 text-xs text-zinc-500">
                  {formData.databaseType === 'sqlite'
                    ? 'Path to the SQLite database file (relative to the server working directory)'
                    : `Full ${formData.databaseType === 'postgres' ? 'PostgreSQL' : 'MySQL'} connection URL including credentials`}
                </p>
              </div>
            </div>
          </div>

          {/* Pool & Safety Settings */}
          <div className="pt-4 border-t border-zinc-700">
            <h3 className="text-sm font-medium text-zinc-200 mb-3">Settings</h3>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Max Connections
                </label>
                <input
                  type="number"
                  name="maxConnections"
                  value={formData.maxConnections}
                  onChange={handleChange}
                  min="1"
                  max="100"
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-cyan-500 focus:border-cyan-500 outline-none"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Timeout (seconds)
                </label>
                <input
                  type="number"
                  name="timeoutSecs"
                  value={formData.timeoutSecs}
                  onChange={handleChange}
                  min="1"
                  max="300"
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-cyan-500 focus:border-cyan-500 outline-none"
                />
              </div>
            </div>

            <div className="mt-4">
              <label className="flex items-center gap-3 cursor-pointer">
                <input
                  type="checkbox"
                  name="readOnly"
                  checked={formData.readOnly}
                  onChange={handleChange}
                  className="w-4 h-4 rounded border-zinc-600 bg-zinc-800 text-cyan-500 focus:ring-cyan-500 focus:ring-offset-0"
                />
                <div>
                  <span className="text-sm font-medium text-zinc-300">Read-only mode</span>
                  <p className="text-xs text-zinc-500">
                    Only allow SELECT, WITH, EXPLAIN, SHOW, DESCRIBE, and PRAGMA queries
                  </p>
                </div>
              </label>
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center justify-between pt-4 border-t border-zinc-700">
            <button
              type="button"
              onClick={onDelete}
              className="px-4 py-2 text-sm font-medium text-red-400 hover:text-red-300 hover:bg-red-900/30 rounded-lg transition-colors"
            >
              Delete Database
            </button>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 text-sm font-medium text-zinc-400 hover:bg-zinc-800 rounded-lg transition-colors"
              >
                Cancel
              </button>
              <button
                type="submit"
                className="px-4 py-2 text-sm font-medium text-white bg-cyan-600 hover:bg-cyan-700 rounded-lg transition-colors"
              >
                Save Changes
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
}
