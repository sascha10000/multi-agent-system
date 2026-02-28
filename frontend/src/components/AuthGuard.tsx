'use client';

import { useState, useEffect, useCallback } from 'react';
import {
  isAuthenticated,
  getStoredUser,
  type AuthUser,
  type LoginResponse,
} from '../lib/auth';
import LoginPage from './LoginPage';

interface AuthGuardProps {
  children: (user: AuthUser) => React.ReactNode;
}

/**
 * Wraps the app content and shows LoginPage if not authenticated.
 * Passes the current user to children via render prop.
 */
export default function AuthGuard({ children }: AuthGuardProps) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [checking, setChecking] = useState(true);

  useEffect(() => {
    if (isAuthenticated()) {
      const stored = getStoredUser();
      setUser(stored);
    }
    setChecking(false);
  }, []);

  const handleLoginSuccess = useCallback((response: LoginResponse) => {
    setUser(response.user);
  }, []);

  if (checking) {
    return (
      <div className="min-h-screen bg-zinc-950 flex items-center justify-center">
        <div className="text-zinc-400 text-sm">Loading...</div>
      </div>
    );
  }

  if (!user) {
    return <LoginPage onSuccess={handleLoginSuccess} />;
  }

  return <>{children(user)}</>;
}
