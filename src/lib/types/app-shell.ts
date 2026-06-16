export type AppTab = 'dashboard' | 'todo' | 'marketplace' | 'settings' | 'progression' | 'raid-management' | 'updates';
// Temporarily disabled due to Supabase realtime message limits
// export type MeowConnectSection = 'together' | 'logs' | 'settings';
// export type MeowConnectHeaderState = 'inactive' | 'connecting' | 'active' | 'sleeping' | 'offline' | 'login_required';
export type DiscordAuthState = 'checking' | 'login' | 'authorizing' | 'welcome' | 'approved' | 'denied' | 'error';
