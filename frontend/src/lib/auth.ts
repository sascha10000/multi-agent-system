// Token management and authenticated fetch wrapper

const API_BASE = process.env.NEXT_PUBLIC_API_BASE || '/api/v1';

const TOKEN_KEY = 'mas_access_token';
const REFRESH_KEY = 'mas_refresh_token';
const USER_KEY = 'mas_user';
const ACTIVE_ORG_KEY = 'mas_active_org';

export interface AuthUser {
  id: string;
  email: string;
  display_name: string;
  created_at: string;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
}

export interface LoginResponse {
  user: AuthUser;
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
}

export interface OrgWithRole {
  id: string;
  name: string;
  slug: string;
  parent_id: string | null;
  role: 'owner' | 'admin' | 'member';
  created_at: string;
  updated_at: string;
}

// ─── Token Storage ──────────────────────────────────────

export function getAccessToken(): string | null {
  if (typeof window === 'undefined') return null;
  return localStorage.getItem(TOKEN_KEY);
}

export function getRefreshToken(): string | null {
  if (typeof window === 'undefined') return null;
  return localStorage.getItem(REFRESH_KEY);
}

export function getStoredUser(): AuthUser | null {
  if (typeof window === 'undefined') return null;
  const raw = localStorage.getItem(USER_KEY);
  if (!raw) return null;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

export function getActiveOrg(): string | null {
  if (typeof window === 'undefined') return null;
  return localStorage.getItem(ACTIVE_ORG_KEY);
}

export function setActiveOrg(orgId: string): void {
  localStorage.setItem(ACTIVE_ORG_KEY, orgId);
}

export function storeAuth(response: LoginResponse): void {
  localStorage.setItem(TOKEN_KEY, response.access_token);
  localStorage.setItem(REFRESH_KEY, response.refresh_token);
  localStorage.setItem(USER_KEY, JSON.stringify(response.user));
}

export function clearAuth(): void {
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(REFRESH_KEY);
  localStorage.removeItem(USER_KEY);
  localStorage.removeItem(ACTIVE_ORG_KEY);
}

export function isAuthenticated(): boolean {
  return !!getAccessToken();
}

// ─── Auth API Calls ─────────────────────────────────────

export async function login(email: string, password: string): Promise<LoginResponse> {
  const res = await fetch(`${API_BASE}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password }),
  });

  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: 'Login failed' }));
    throw new Error(err.error || 'Login failed');
  }

  const data: LoginResponse = await res.json();
  storeAuth(data);
  return data;
}

export async function register(
  email: string,
  displayName: string,
  password: string,
): Promise<LoginResponse> {
  const res = await fetch(`${API_BASE}/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, display_name: displayName, password }),
  });

  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: 'Registration failed' }));
    throw new Error(err.error || 'Registration failed');
  }

  const data: LoginResponse = await res.json();
  storeAuth(data);
  return data;
}

export function logout(): void {
  clearAuth();
  window.location.reload();
}

// ─── Token Refresh ──────────────────────────────────────

let refreshPromise: Promise<string> | null = null;

async function refreshAccessToken(): Promise<string> {
  // Deduplicate concurrent refresh attempts
  if (refreshPromise) return refreshPromise;

  refreshPromise = (async () => {
    const refreshToken = getRefreshToken();
    if (!refreshToken) {
      clearAuth();
      throw new Error('No refresh token');
    }

    const res = await fetch(`${API_BASE}/auth/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: refreshToken }),
    });

    if (!res.ok) {
      clearAuth();
      throw new Error('Token refresh failed');
    }

    const data = await res.json();
    localStorage.setItem(TOKEN_KEY, data.access_token);
    localStorage.setItem(REFRESH_KEY, data.refresh_token);
    return data.access_token as string;
  })();

  try {
    return await refreshPromise;
  } finally {
    refreshPromise = null;
  }
}

// ─── Authenticated Fetch ────────────────────────────────

/**
 * Drop-in replacement for fetch() that auto-attaches the Bearer token
 * and handles 401 by refreshing the token once.
 */
export async function authFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  const token = getAccessToken();

  const headers = new Headers(init?.headers);
  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  let res = await fetch(input, { ...init, headers });

  // On 401, try refreshing the token once
  if (res.status === 401 && getRefreshToken()) {
    try {
      const newToken = await refreshAccessToken();
      headers.set('Authorization', `Bearer ${newToken}`);
      res = await fetch(input, { ...init, headers });
    } catch {
      // Refresh failed — user needs to log in again
      clearAuth();
      window.location.reload();
      throw new Error('Session expired');
    }
  }

  return res;
}
