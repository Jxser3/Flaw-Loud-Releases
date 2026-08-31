export const API_BASE_URL = (import.meta.env.VITE_API_BASE_URL || 'https://flaw-loud-releases-production-4186.up.railway.app').replace(/\/$/, '');

export type AuthUser = {
  id: string;
  username: string;
  role: 'user' | 'admin';
  createdAt: string | null;
  lastLoginAt: string | null;
  lastSeenAt: string | null;
  status?: 'online' | 'offline';
};

export type AdminStats = { totalUsers: number; onlineUsers: number; admins: number };

export class ApiError extends Error {
  constructor(public status: number, message: string) { super(message); }
}

const friendlyMessage = (status: number) => {
  if (status === 400) return 'Check the information and try again.';
  if (status === 401) return 'The username or password is incorrect.';
  if (status === 403) return 'You do not have permission to access this area.';
  if (status === 404) return 'The requested information could not be found.';
  if (status === 409) return 'That username is already in use.';
  if (status === 429) return 'Too many attempts. Please wait a moment and try again.';
  return 'Flaw Loud could not connect. Please try again.';
};

export async function apiRequest<T>(path: string, options: RequestInit = {}): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`${API_BASE_URL}${path}`, {
      ...options,
      credentials: 'include',
      headers: { 'Content-Type': 'application/json', ...options.headers },
    });
  } catch {
    throw new ApiError(0, 'Flaw Loud could not connect. Check your connection and try again.');
  }

  if (!response.ok) {
    if (response.status === 401 && !path.startsWith('/api/auth/')) {
      window.dispatchEvent(new CustomEvent('flaw-session-expired'));
    }
    throw new ApiError(response.status, friendlyMessage(response.status));
  }

  return response.status === 204 ? undefined as T : response.json();
}

export const authApi = {
  me: () => apiRequest<{ user: AuthUser }>('/api/auth/me'),
  login: (username: string, password: string, rememberMe: boolean) => apiRequest<{ user: AuthUser }>('/api/auth/login', { method: 'POST', body: JSON.stringify({ username, password, rememberMe }) }),
  register: (username: string, password: string) => apiRequest<{ user: AuthUser }>('/api/auth/register', { method: 'POST', body: JSON.stringify({ username, password }) }),
  logout: () => apiRequest<void>('/api/auth/logout', { method: 'POST' }),
};

export const adminApi = {
  stats: () => apiRequest<AdminStats>('/api/admin/stats'),
  users: () => apiRequest<{ users: AuthUser[] }>('/api/admin/users'),
  user: (id: string) => apiRequest<{ user: AuthUser }>(`/api/admin/users/${encodeURIComponent(id)}`),
};
