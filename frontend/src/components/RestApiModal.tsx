'use client';

import { useState, useEffect } from 'react';
import type { RestApiNodeData, HttpMethod, ResponseFormat } from '../types/agent';

interface RestApiModalProps {
  isOpen: boolean;
  restApi: RestApiNodeData | null;
  onSave: (data: RestApiNodeData) => void;
  onDelete: () => void;
  onClose: () => void;
}

export default function RestApiModal({
  isOpen,
  restApi,
  onSave,
  onDelete,
  onClose,
}: RestApiModalProps) {
  const [formData, setFormData] = useState<RestApiNodeData | null>(null);
  const [headersText, setHeadersText] = useState('');
  const [jsonError, setJsonError] = useState<string | null>(null);

  useEffect(() => {
    if (restApi) {
      setFormData({ ...restApi });
      const headersStr = Object.entries(restApi.headers || {})
        .map(([k, v]) => `${k}: ${v}`)
        .join('\n');
      setHeadersText(headersStr);
      setJsonError(null);
    }
  }, [restApi]);

  if (!isOpen || !formData) return null;

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>
  ) => {
    const { name, value, type } = e.target;

    if (type === 'number') {
      setFormData((prev) => prev ? { ...prev, [name]: parseFloat(value) || 0 } : null);
    } else {
      setFormData((prev) => prev ? { ...prev, [name]: value } : null);
    }
  };

  const handleHeadersChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setHeadersText(e.target.value);
    const headers: Record<string, string> = {};
    e.target.value.split('\n').forEach((line) => {
      const colonIdx = line.indexOf(':');
      if (colonIdx > 0) {
        const key = line.substring(0, colonIdx).trim();
        const value = line.substring(colonIdx + 1).trim();
        if (key) {
          headers[key] = value;
        }
      }
    });
    setFormData((prev) => prev ? { ...prev, headers } : null);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formData) return;

    // Validate body template JSON if provided
    if (formData.bodyTemplate.trim()) {
      try {
        JSON.parse(formData.bodyTemplate);
      } catch {
        setJsonError('Invalid JSON in Body Template');
        return;
      }
    }

    // Validate parameters JSON if provided
    if (formData.parameters.trim()) {
      try {
        JSON.parse(formData.parameters);
      } catch {
        setJsonError('Invalid JSON in Parameters Schema');
        return;
      }
    }

    setJsonError(null);
    onSave(formData);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70">
      <div className="bg-zinc-900 rounded-xl shadow-2xl w-full max-w-lg max-h-[90vh] overflow-y-auto border border-zinc-700">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-zinc-700 bg-rose-900/20">
          <div className="flex items-center gap-2">
            <svg
              className="w-5 h-5 text-rose-400"
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
            <h2 className="text-lg font-semibold text-zinc-100">Edit REST API</h2>
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
          {/* JSON Error Banner */}
          {jsonError && (
            <div className="p-3 bg-red-900/50 border border-red-700 rounded-lg text-red-200 text-sm">
              {jsonError}
            </div>
          )}

          {/* Basic Info */}
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">Name</label>
              <input
                type="text"
                name="name"
                value={formData.name}
                onChange={handleChange}
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-rose-500 focus:border-rose-500 outline-none"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">Description</label>
              <textarea
                name="description"
                value={formData.description}
                onChange={handleChange}
                rows={2}
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-rose-500 focus:border-rose-500 outline-none resize-none"
                placeholder="Describe what this API does..."
              />
            </div>
          </div>

          {/* Endpoint */}
          <div className="pt-4 border-t border-zinc-700">
            <h3 className="text-sm font-medium text-zinc-200 mb-3">Endpoint</h3>

            <div className="flex gap-4">
              <div className="w-32">
                <label className="block text-sm font-medium text-zinc-300 mb-1">Method</label>
                <select
                  name="endpointMethod"
                  value={formData.endpointMethod}
                  onChange={handleChange}
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-rose-500 focus:border-rose-500 outline-none"
                >
                  <option value="GET">GET</option>
                  <option value="POST">POST</option>
                  <option value="PUT">PUT</option>
                  <option value="DELETE">DELETE</option>
                  <option value="PATCH">PATCH</option>
                </select>
              </div>

              <div className="flex-1">
                <label className="block text-sm font-medium text-zinc-300 mb-1">URL</label>
                <input
                  type="url"
                  name="endpointUrl"
                  value={formData.endpointUrl}
                  onChange={handleChange}
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-rose-500 focus:border-rose-500 outline-none font-mono text-sm"
                  placeholder="https://api.example.com/endpoint"
                  required
                />
              </div>
            </div>
          </div>

          {/* Headers */}
          <div>
            <label className="block text-sm font-medium text-zinc-300 mb-1">
              Headers <span className="text-zinc-500 font-normal">(one per line: Key: Value)</span>
            </label>
            <textarea
              value={headersText}
              onChange={handleHeadersChange}
              rows={3}
              className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-rose-500 focus:border-rose-500 outline-none resize-none font-mono text-sm"
              placeholder={'Authorization: Bearer ${API_KEY}\nContent-Type: application/json'}
            />
          </div>

          {/* Body Template */}
          <div>
            <label className="block text-sm font-medium text-zinc-300 mb-1">
              Body Template <span className="text-zinc-500 font-normal">(JSON, optional)</span>
            </label>
            <textarea
              name="bodyTemplate"
              value={formData.bodyTemplate}
              onChange={handleChange}
              rows={4}
              className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-rose-500 focus:border-rose-500 outline-none resize-none font-mono text-sm"
              placeholder='{"query": "${query}"}'
            />
            <p className="mt-1 text-xs text-zinc-500">
              Use {'${param}'} for parameter substitution
            </p>
          </div>

          {/* Parameters Schema */}
          <div className="pt-4 border-t border-zinc-700">
            <h3 className="text-sm font-medium text-zinc-200 mb-3">Parameters Schema</h3>
            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">
                JSON Schema <span className="text-zinc-500 font-normal">(shown to LLM)</span>
              </label>
              <textarea
                name="parameters"
                value={formData.parameters}
                onChange={handleChange}
                rows={6}
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-rose-500 focus:border-rose-500 outline-none resize-none font-mono text-sm"
                placeholder='{"type": "object", "properties": {...}}'
              />
            </div>
          </div>

          {/* Response */}
          <div className="pt-4 border-t border-zinc-700">
            <h3 className="text-sm font-medium text-zinc-200 mb-3">Response</h3>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Extract Path <span className="text-zinc-500 font-normal">(JSONPath)</span>
                </label>
                <input
                  type="text"
                  name="extractPath"
                  value={formData.extractPath}
                  onChange={handleChange}
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-rose-500 focus:border-rose-500 outline-none font-mono text-sm"
                  placeholder="$.data.results"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">Format</label>
                <select
                  name="responseFormat"
                  value={formData.responseFormat}
                  onChange={handleChange}
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-rose-500 focus:border-rose-500 outline-none"
                >
                  <option value="json">JSON</option>
                  <option value="text">Plain Text</option>
                  <option value="markdown">Markdown</option>
                </select>
              </div>
            </div>
          </div>

          {/* Timeout */}
          <div className="pt-4 border-t border-zinc-700">
            <div className="w-32">
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
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-rose-500 focus:border-rose-500 outline-none"
              />
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center justify-between pt-4 border-t border-zinc-700">
            <button
              type="button"
              onClick={onDelete}
              className="px-4 py-2 text-sm font-medium text-red-400 hover:text-red-300 hover:bg-red-900/30 rounded-lg transition-colors"
            >
              Delete REST API
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
                className="px-4 py-2 text-sm font-medium text-white bg-rose-600 hover:bg-rose-700 rounded-lg transition-colors"
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
