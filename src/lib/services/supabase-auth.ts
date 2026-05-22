import { invoke } from '@tauri-apps/api/core';
import { createClient, type Session, type User } from '@supabase/supabase-js';

const SUPABASE_URL = 'https://jvpmxbjqfqdgmdzeltdg.supabase.co';
const SUPABASE_ANON_KEY =
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Imp2cG14YmpxZnFkZ21kemVsdGRnIiwicm9sZSI6ImFub24iLCJpYXQiOjE3Nzk0MDcwMDksImV4cCI6MjA5NDk4MzAwOX0.OzvInEBWVk8UAjRDR1u5jJ0ctnWPgj203hpr-FWS5K0';
const SUPABASE_AUTH_REDIRECT_URL = 'http://127.0.0.1:53682/supabase/callback';

export interface DiscordAuthResult {
  approved: boolean;
  user_id?: string;
  username?: string;
  message: string;
}

interface SupabaseOAuthCodeResult {
  code: string;
}

interface DiscordIdentity {
  discordId: string;
  username?: string;
  avatarUrl?: string;
}

export interface SupabaseDiscordProfile {
  userId: string;
  discordId: string;
  displayName: string;
  avatarUrl?: string;
  accessToken: string;
}

export const supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY, {
  auth: {
    autoRefreshToken: true,
    detectSessionInUrl: false,
    flowType: 'pkce',
    persistSession: true
  }
});

export function getSupabaseProjectUrl(): string {
  return SUPABASE_URL;
}

export function getSupabaseAnonKey(): string {
  return SUPABASE_ANON_KEY;
}

export async function getStoredSupabaseDiscordAuth(): Promise<DiscordAuthResult | null> {
  const { data, error } = await supabase.auth.getSession();
  if (error || !data.session?.user) {
    return null;
  }

  return verifySupabaseDiscordUser(data.session.user);
}

export async function signInWithSupabaseDiscord(): Promise<DiscordAuthResult> {
  const { data, error } = await supabase.auth.signInWithOAuth({
    provider: 'discord',
    options: {
      redirectTo: SUPABASE_AUTH_REDIRECT_URL,
      scopes: 'identify',
      skipBrowserRedirect: true
    }
  });

  if (error) {
    throw new Error(error.message);
  }
  if (!data.url) {
    throw new Error('Supabase did not return a Discord login URL.');
  }

  const callback = await invoke<SupabaseOAuthCodeResult>('authenticate_supabase_discord', {
    authUrl: data.url
  });

  const sessionResult = await supabase.auth.exchangeCodeForSession(callback.code);
  if (sessionResult.error) {
    throw new Error(sessionResult.error.message);
  }
  if (!sessionResult.data.user) {
    throw new Error('Supabase Discord login did not return a user.');
  }

  return verifySupabaseDiscordUser(sessionResult.data.user);
}

export async function getSupabaseAccessToken(): Promise<string | null> {
  const { data } = await supabase.auth.getSession();
  return data.session?.access_token ?? null;
}

export async function getSupabaseSession(): Promise<Session | null> {
  const { data } = await supabase.auth.getSession();
  return data.session ?? null;
}

export async function getCurrentSupabaseDiscordProfile(): Promise<SupabaseDiscordProfile> {
  const { data, error } = await supabase.auth.getSession();
  if (error) {
    throw new Error(error.message);
  }
  if (!data.session?.user) {
    throw new Error('MeowConnect requires Discord login.');
  }

  const identity = extractDiscordIdentity(data.session.user);
  if (!identity?.discordId) {
    throw new Error('Supabase session is missing Discord identity data.');
  }

  return {
    userId: data.session.user.id,
    discordId: identity.discordId,
    displayName: identity.username || identity.discordId,
    avatarUrl: identity.avatarUrl,
    accessToken: data.session.access_token
  };
}

async function verifySupabaseDiscordUser(user: User): Promise<DiscordAuthResult> {
  const identity = extractDiscordIdentity(user);
  if (!identity?.discordId) {
    return {
      approved: false,
      message: 'Supabase session is missing Discord identity data.'
    };
  }

  return invoke<DiscordAuthResult>('verify_discord_profile_auth', {
    discordId: identity.discordId,
    username: identity.username
  });
}

function extractDiscordIdentity(user: User): DiscordIdentity | null {
  const discordIdentity = user.identities?.find((identity) => identity.provider === 'discord');
  const identityData = (discordIdentity?.identity_data || {}) as Record<string, unknown>;
  const userMetadata = (user.user_metadata || {}) as Record<string, unknown>;

  const discordId = firstString(
    identityData.sub,
    identityData.provider_id,
    identityData.id,
    discordIdentity?.id,
    userMetadata.sub,
    userMetadata.provider_id,
    userMetadata.discord_id
  );

  if (!discordId) {
    return null;
  }

  return {
    discordId,
    username: firstString(
      identityData.full_name,
      identityData.name,
      identityData.user_name,
      identityData.preferred_username,
      userMetadata.full_name,
      userMetadata.name,
      userMetadata.user_name,
      userMetadata.preferred_username,
      user.email
    ),
    avatarUrl: firstString(identityData.avatar_url, identityData.picture, userMetadata.avatar_url, userMetadata.picture)
  };
}

function firstString(...values: unknown[]): string | undefined {
  return values.find((value): value is string => typeof value === 'string' && value.trim().length > 0)?.trim();
}
