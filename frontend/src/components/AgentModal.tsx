'use client';

import { useState, useEffect } from 'react';
import type { AgentNodeData, RoutingBehavior } from '../types/agent';

interface AgentModalProps {
  isOpen: boolean;
  agent: AgentNodeData | null;
  onSave: (data: AgentNodeData) => void;
  onDelete: () => void;
  onClose: () => void;
}

export default function AgentModal({
  isOpen,
  agent,
  onSave,
  onDelete,
  onClose,
}: AgentModalProps) {
  const [formData, setFormData] = useState<AgentNodeData | null>(null);
  const [errors, setErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    if (agent) {
      setFormData({ ...agent });
      setErrors({});
    }
  }, [agent]);

  const validateForm = (data: AgentNodeData): Record<string, string> => {
    const errs: Record<string, string> = {};

    if (!data.name.trim()) {
      errs.name = 'Name is required';
    }
    if (data.temperature < 0 || data.temperature > 2) {
      errs.temperature = 'Must be between 0 and 2';
    }
    if (data.maxTokens < 1 || data.maxTokens > 128000) {
      errs.maxTokens = 'Must be between 1 and 128,000';
    }

    return errs;
  };

  if (!isOpen || !formData) return null;

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>
  ) => {
    const { name, value, type } = e.target;

    if (type === 'checkbox') {
      const checked = (e.target as HTMLInputElement).checked;
      setFormData((prev) => prev ? { ...prev, [name]: checked } : null);
    } else if (type === 'number') {
      setFormData((prev) => prev ? { ...prev, [name]: parseFloat(value) || 0 } : null);
    } else {
      setFormData((prev) => prev ? { ...prev, [name]: value } : null);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formData) return;

    const validationErrors = validateForm(formData);
    setErrors(validationErrors);

    if (Object.keys(validationErrors).length === 0) {
      onSave(formData);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70">
      <div className="bg-zinc-900 rounded-xl shadow-2xl w-full max-w-lg max-h-[90vh] overflow-y-auto border border-zinc-700">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-zinc-700">
          <h2 className="text-lg font-semibold text-zinc-100">Edit Agent</h2>
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
                className={`w-full px-3 py-2 bg-zinc-800 border rounded-lg text-zinc-100 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none ${
                  errors.name ? 'border-red-500' : 'border-zinc-600'
                }`}
              />
              {errors.name && (
                <p className="text-red-400 text-xs mt-1">{errors.name}</p>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">
                System Prompt
              </label>
              <textarea
                name="systemPrompt"
                value={formData.systemPrompt}
                onChange={handleChange}
                rows={3}
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none resize-none"
              />
            </div>
          </div>

          {/* LLM Settings */}
          <div className="pt-4 border-t border-zinc-700">
            <h3 className="text-sm font-medium text-zinc-200 mb-3">LLM Settings</h3>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Provider
                </label>
                <input
                  type="text"
                  name="provider"
                  value={formData.provider}
                  onChange={handleChange}
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Model
                </label>
                <input
                  type="text"
                  name="model"
                  value={formData.model}
                  onChange={handleChange}
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Temperature <span className="text-zinc-500 font-normal">(0-2)</span>
                </label>
                <input
                  type="number"
                  name="temperature"
                  value={formData.temperature}
                  onChange={handleChange}
                  min="0"
                  max="2"
                  step="0.1"
                  className={`w-full px-3 py-2 bg-zinc-800 border rounded-lg text-zinc-100 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none ${
                    errors.temperature ? 'border-red-500' : 'border-zinc-600'
                  }`}
                />
                {errors.temperature && (
                  <p className="text-red-400 text-xs mt-1">{errors.temperature}</p>
                )}
              </div>

              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Max Tokens <span className="text-zinc-500 font-normal">(1-128k)</span>
                </label>
                <input
                  type="number"
                  name="maxTokens"
                  value={formData.maxTokens}
                  onChange={handleChange}
                  min="1"
                  max="128000"
                  step="1"
                  className={`w-full px-3 py-2 bg-zinc-800 border rounded-lg text-zinc-100 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none ${
                    errors.maxTokens ? 'border-red-500' : 'border-zinc-600'
                  }`}
                />
                {errors.maxTokens && (
                  <p className="text-red-400 text-xs mt-1">{errors.maxTokens}</p>
                )}
              </div>
            </div>
          </div>

          {/* Chat & Routing Settings */}
          <div className="pt-4 border-t border-zinc-700">
            <h3 className="text-sm font-medium text-zinc-200 mb-3">Chat & Routing</h3>

            <div className="space-y-3">
              <label className="flex items-center gap-3 cursor-pointer">
                <input
                  type="checkbox"
                  name="entryPoint"
                  checked={formData.entryPoint}
                  onChange={handleChange}
                  className="w-4 h-4 text-green-500 bg-zinc-800 border-zinc-600 rounded focus:ring-green-500"
                />
                <div>
                  <span className="text-sm text-zinc-300">Entry point</span>
                  <p className="text-xs text-zinc-500">Chat messages are sent to this agent</p>
                </div>
              </label>

              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Routing Behavior
                </label>
                <select
                  name="routingBehavior"
                  value={formData.routingBehavior}
                  onChange={handleChange}
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                >
                  <option value="best">Best - Forward to most appropriate agent</option>
                  <option value="all">All - Forward to all connected agents</option>
                  <option value="direct_first">Direct First - Try to answer, then forward</option>
                </select>
                <p className="text-xs text-zinc-500 mt-1">Applies when agent has connections to other agents</p>
              </div>
              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Max Turns <span className="text-zinc-500 font-normal">(0 = unlimited)</span>
                </label>
                <input
                  type="number"
                  name="maxTurns"
                  value={formData.maxTurns}
                  onChange={handleChange}
                  min="0"
                  max="100"
                  step="1"
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-600 rounded-lg text-zinc-100 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                />
                <p className="text-xs text-zinc-500 mt-1">How many follow-up rounds with other agents (1 = single exchange)</p>
              </div>
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center justify-between pt-4 border-t border-zinc-700">
            <button
              type="button"
              onClick={onDelete}
              className="px-4 py-2 text-sm font-medium text-red-400 hover:text-red-300 hover:bg-red-900/30 rounded-lg transition-colors"
            >
              Delete Agent
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
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
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
