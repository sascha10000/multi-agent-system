'use client';

import { useState, useEffect } from 'react';
import type { ToolNodeData, HttpMethod, ResponseFormat, EndpointType } from '../types/agent';

interface ToolModalProps {
  isOpen: boolean;
  tool: ToolNodeData | null;
  onSave: (data: ToolNodeData) => void;
  onDelete: () => void;
  onClose: () => void;
}

export default function ToolModal({
  isOpen,
  tool,
  onSave,
  onDelete,
  onClose,
}: ToolModalProps) {
  const [formData, setFormData] = useState<ToolNodeData | null>(null);
  const [headersText, setHeadersText] = useState('');
  const [jsonError, setJsonError] = useState<string | null>(null);

  useEffect(() => {
    if (tool) {
      setFormData({ ...tool });
      // Convert headers object to text format
      const headersStr = Object.entries(tool.headers || {})
        .map(([k, v]) => `${k}: ${v}`)
        .join('\n');
      setHeadersText(headersStr);
      setJsonError(null);
    }
  }, [tool]);

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
    // Parse headers text to object
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

  const validateJson = (jsonStr: string, fieldName: string): boolean => {
    if (!jsonStr.trim()) return true; // Empty is valid
    try {
      JSON.parse(jsonStr);
      setJsonError(null);
      return true;
    } catch {
      setJsonError(`Invalid JSON in ${fieldName}`);
      return false;
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formData) return;

    // Validate JSON fields (body template only for HTTP)
    if (formData.endpointType === 'http' && !validateJson(formData.bodyTemplate, 'Body Template')) return;
    if (formData.endpointType !== 'mcp' && !validateJson(formData.parameters, 'Parameters Schema')) return;

    // MCP requires tool name
    if (formData.endpointType === 'mcp' && !formData.mcpToolName?.trim()) {
      setJsonError('MCP Tool Name is required');
      return;
    }

    onSave(formData);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70">
      <div className="bg-zinc-900 rounded-xl shadow-2xl w-full max-w-2xl max-h-[90vh] overflow-y-auto border border-zinc-700">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-zinc-700 bg-amber-900/20">
          <div className="flex items-center gap-2">
            <svg
              className="w-5 h-5 text-amber-400"
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
            <h2 className="text-lg font-semibold text-zinc-100">Edit Tool</h2>
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
              <label className="block text-sm font-medium text-zinc-300 mb-1">
                Name
              </label>
              <input
                type="text"
                name="name"
                value={formData.name}
                onChange={handleChange}
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 outline-none"
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
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 outline-none resize-none"
                placeholder="Describe what this tool does..."
              />
            </div>
          </div>

          {/* Endpoint Settings */}
          <div className="pt-4 border-t border-zinc-700">
            <h3 className="text-sm font-medium text-zinc-200 mb-3">Endpoint Configuration</h3>

            <div className="space-y-4">
              {/* Endpoint Type Selector */}
              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Endpoint Type
                </label>
                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={() => setFormData((prev) => prev ? { ...prev, endpointType: 'mcp' } : null)}
                    className={`flex-1 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                      formData.endpointType === 'mcp'
                        ? 'bg-purple-600 text-white'
                        : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
                    }`}
                  >
                    MCP (Model Context Protocol)
                  </button>
                  <button
                    type="button"
                    onClick={() => setFormData((prev) => prev ? { ...prev, endpointType: 'http' } : null)}
                    className={`flex-1 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                      formData.endpointType === 'http'
                        ? 'bg-blue-600 text-white'
                        : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
                    }`}
                  >
                    HTTP (Custom Request)
                  </button>
                </div>
                <p className="mt-1 text-xs text-zinc-500">
                  {formData.endpointType === 'mcp'
                    ? 'MCP uses JSON-RPC 2.0 protocol. Just provide the server URL and tool name.'
                    : 'HTTP allows custom request formatting with headers and body templates.'}
                </p>
              </div>

              {/* URL and Method Row */}
              <div className="flex gap-4">
                {formData.endpointType === 'http' && (
                  <div className="w-32">
                    <label className="block text-sm font-medium text-zinc-300 mb-1">
                      Method
                    </label>
                    <select
                      name="endpointMethod"
                      value={formData.endpointMethod}
                      onChange={handleChange}
                      className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 outline-none"
                    >
                      <option value="GET">GET</option>
                      <option value="POST">POST</option>
                      <option value="PUT">PUT</option>
                      <option value="DELETE">DELETE</option>
                      <option value="PATCH">PATCH</option>
                    </select>
                  </div>
                )}

                <div className="flex-1">
                  <label className="block text-sm font-medium text-zinc-300 mb-1">
                    {formData.endpointType === 'mcp' ? 'MCP Server URL' : 'URL'}
                  </label>
                  <input
                    type="url"
                    name="endpointUrl"
                    value={formData.endpointUrl}
                    onChange={handleChange}
                    className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 outline-none font-mono text-sm"
                    placeholder={formData.endpointType === 'mcp' ? 'https://example.com/mcp' : 'https://api.example.com/endpoint'}
                    required
                  />
                </div>
              </div>

              {/* MCP Tool Name (only for MCP) */}
              {formData.endpointType === 'mcp' && (
                <div>
                  <label className="block text-sm font-medium text-zinc-300 mb-1">
                    MCP Tool Name <span className="text-zinc-500 font-normal">(the tool to call on the server)</span>
                  </label>
                  <input
                    type="text"
                    name="mcpToolName"
                    value={formData.mcpToolName}
                    onChange={handleChange}
                    className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 outline-none font-mono text-sm"
                    placeholder="search_jobs"
                    required={formData.endpointType === 'mcp'}
                  />
                  <p className="mt-1 text-xs text-zinc-500">
                    This is the name of the tool registered on the MCP server (e.g., &quot;search_jobs&quot;, &quot;get_weather&quot;)
                  </p>
                </div>
              )}

              {/* Headers (for both, but more common with HTTP) */}
              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Headers <span className="text-zinc-500 font-normal">(one per line: Key: Value)</span>
                </label>
                <textarea
                  value={headersText}
                  onChange={handleHeadersChange}
                  rows={formData.endpointType === 'mcp' ? 2 : 3}
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 outline-none resize-none font-mono text-sm"
                  placeholder={formData.endpointType === 'mcp'
                    ? 'Authorization: Bearer ${API_KEY}'
                    : 'Authorization: Bearer ${API_KEY}\nContent-Type: application/json'}
                />
                <p className="mt-1 text-xs text-zinc-500">
                  Use {'${ENV_VAR}'} for environment variables
                </p>
              </div>

              {/* Body Template (HTTP only) */}
              {formData.endpointType === 'http' && (
                <div>
                  <label className="block text-sm font-medium text-zinc-300 mb-1">
                    Body Template <span className="text-zinc-500 font-normal">(JSON)</span>
                  </label>
                  <textarea
                    name="bodyTemplate"
                    value={formData.bodyTemplate}
                    onChange={handleChange}
                    rows={4}
                    className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 outline-none resize-none font-mono text-sm"
                    placeholder='{"query": "${query}"}'
                  />
                  <p className="mt-1 text-xs text-zinc-500">
                    Use {'${param}'} for parameter substitution
                  </p>
                </div>
              )}
            </div>
          </div>

          {/* Parameters Schema (HTTP only — MCP servers advertise their own schema) */}
          {formData.endpointType !== 'mcp' && (
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
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 outline-none resize-none font-mono text-sm"
                  placeholder='{"type": "object", "properties": {...}}'
                />
              </div>
            </div>
          )}

          {/* Response Mapping */}
          <div className="pt-4 border-t border-zinc-700">
            <h3 className="text-sm font-medium text-zinc-200 mb-3">Response Mapping</h3>

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
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 outline-none font-mono text-sm"
                  placeholder="$.data.results"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Response Format
                </label>
                <select
                  name="responseFormat"
                  value={formData.responseFormat}
                  onChange={handleChange}
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 outline-none"
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
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 outline-none"
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
              Delete Tool
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
                className="px-4 py-2 text-sm font-medium text-white bg-amber-600 hover:bg-amber-700 rounded-lg transition-colors"
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
