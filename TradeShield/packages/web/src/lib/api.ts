const API_BASE = '/api/v1';

async function fetchApi<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(error.error || error.message || `HTTP ${response.status}`);
  }

  return response.json();
}

/**
 * Download a file from the API (e.g., PDF certificate).
 * Triggers a browser download dialog.
 */
async function downloadFile(endpoint: string, filename: string) {
  const response = await fetch(`${API_BASE}${endpoint}`, {
    credentials: 'include',
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Download failed' }));
    throw new Error(error.error || `HTTP ${response.status}`);
  }

  const blob = await response.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

// Dashboard API
export const dashboardApi = {
  getSummary: () => fetchApi<any>('/dashboard/summary'),
  getTimeline: (params?: { period?: string; start_date?: string; end_date?: string }) => {
    const searchParams = new URLSearchParams(params as any);
    return fetchApi<any>(`/dashboard/timeline?${searchParams}`);
  },
  getDevices: () => fetchApi<any>('/dashboard/devices'),
};

// Devices API
export const devicesApi = {
  list: () => fetchApi<any>('/devices'),
  get: (id: string) => fetchApi<any>(`/devices/${id}`),
};

// Events API
export const eventsApi = {
  list: (params?: {
    page?: number;
    limit?: number;
    device_id?: string;
    start_date?: string;
    end_date?: string;
  }) => {
    const searchParams = new URLSearchParams(params as any);
    return fetchApi<any>(`/events?${searchParams}`);
  },
};

// Credits API
export const creditsApi = {
  list: (params?: { page?: number; limit?: number; status?: string }) => {
    const searchParams = new URLSearchParams(params as any);
    return fetchApi<any>(`/credits?${searchParams}`);
  },
  issue: (data: { source_org_id: string; period_start: string; period_end: string }) =>
    fetchApi<any>('/credits/issue', {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  downloadCertificate: (creditId: string) =>
    downloadFile(`/credits/${creditId}/certificate`, `washtrap-certificate-${creditId.slice(0, 8)}.pdf`),
};

// Auth API
export const authApi = {
  login: (email: string, password: string) =>
    fetchApi<any>('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    }),
  logout: () => fetchApi<any>('/auth/logout', { method: 'POST' }),
  me: () => fetchApi<any>('/auth/me'),
};
