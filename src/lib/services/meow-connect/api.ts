import type { MeowConnectSupabaseConfig } from './types';

export async function meowConnectSupabaseRequest<T>(
  config: MeowConnectSupabaseConfig,
  path: string,
  init: RequestInit = {}
): Promise<T> {
  const url = `${config.url.replace(/\/$/, '')}/rest/v1/${path.replace(/^\//, '')}`;
  const response = await fetch(url, {
    ...init,
    headers: {
      apikey: config.anonKey,
      authorization: `Bearer ${config.accessToken || config.anonKey}`,
      'content-type': 'application/json',
      ...(init.headers || {})
    }
  });

  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`MeowConnect Supabase request failed (${response.status}): ${detail}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json() as Promise<T>;
}
