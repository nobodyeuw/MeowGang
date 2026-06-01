import { invoke } from '@tauri-apps/api/core';
import type { FriendOption } from '$lib/components/meow-connect/types';

export function loadDiscordWhitelistMembers(): Promise<FriendOption[]> {
  return invoke<FriendOption[]>('get_discord_whitelist_members');
}
