'use client';

import { useState } from 'react';
import { login, register, type LoginResponse } from '../lib/auth';

interface LoginPageProps {
  onSuccess: (response: LoginResponse) => void;
}

export default function LoginPage({ onSuccess }: LoginPageProps) {
  const [isRegister, setIsRegister] = useState(false);
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      let response: LoginResponse;
      if (isRegister) {
        response = await register(email, displayName, password);
      } else {
        response = await login(email, password);
      }
      onSuccess(response);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-zinc-950 flex items-center justify-center p-4">
      <div className="w-full max-w-md">
        {/* Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-white mb-2">
            Multi-Agent System
          </h1>
          <p className="text-zinc-400 text-sm">
            {isRegister ? 'Create your account' : 'Sign in to continue'}
          </p>
        </div>

        {/* Form Card */}
        <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-6">
          <form onSubmit={handleSubmit} className="space-y-4">
            {isRegister && (
              <div>
                <label className="block text-sm font-medium text-zinc-300 mb-1">
                  Display Name
                </label>
                <input
                  type="text"
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-md text-white placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  placeholder="Your name"
                  required={isRegister}
                />
              </div>
            )}

            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">
                Email
              </label>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-md text-white placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                placeholder="you@example.com"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-zinc-300 mb-1">
                Password
              </label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-md text-white placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                placeholder={isRegister ? 'At least 8 characters' : 'Your password'}
                required
                minLength={isRegister ? 8 : undefined}
              />
            </div>

            {error && (
              <div className="text-red-400 text-sm bg-red-500/10 border border-red-500/20 rounded-md px-3 py-2">
                {error}
              </div>
            )}

            <button
              type="submit"
              disabled={loading}
              className="w-full py-2 px-4 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-600/50 text-white rounded-md font-medium transition-colors"
            >
              {loading
                ? (isRegister ? 'Creating account...' : 'Signing in...')
                : (isRegister ? 'Create Account' : 'Sign In')}
            </button>
          </form>

          {/* Toggle */}
          <div className="mt-4 text-center text-sm text-zinc-400">
            {isRegister ? (
              <>
                Already have an account?{' '}
                <button
                  onClick={() => { setIsRegister(false); setError(''); }}
                  className="text-blue-400 hover:text-blue-300"
                >
                  Sign in
                </button>
              </>
            ) : (
              <>
                Don&apos;t have an account?{' '}
                <button
                  onClick={() => { setIsRegister(true); setError(''); }}
                  className="text-blue-400 hover:text-blue-300"
                >
                  Create one
                </button>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
