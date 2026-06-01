import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  GAME_CLASSES,
  GAME_TASKS,
  RAIDS
} from '$lib/data';
import {
  buildStoreGameClasses,
  buildStoreGameTasks,
  buildSyncRaidPayload,
  buildSyncTaskPayload
} from '$lib/utils/store-data';
import { getActiveRosterPreference, setActiveRosterPreference } from '$lib/services/roster-preferences';
import type {
  BootstrapSnapshot,
  Character,
  CharacterSettings,
  GameClass,
  GameTask,
  Roster,
  SystemSettings,
  TodoMatrixResponse
} from '$lib/types/store';

export type {
  BootstrapSnapshot,
  Character,
  CharacterRaidState,
  CharacterSettings,
  CharacterTaskState,
  GameClass,
  GameTask,
  Raid,
  RaidGate,
  RaidGateState,
  RaidSettingsEntry,
  Roster,
  SystemSettings,
  TodoCharacter,
  TodoMatrixResponse,
  TodoRaid,
  TodoRaidGate,
  TodoSettingsEntry,
  TodoTask
} from '$lib/types/store';
// Writable Stores
export const rosters = writable<Roster[]>([]);
export const activeRosterId = writable<string>('');
export const characters = writable<Character[]>([]);
export const gameClasses = writable<GameClass[]>([]);
export const gameTasks = writable<GameTask[]>([]);
export const todoMatrix = writable<TodoMatrixResponse | null>(null);
export const activeFilterCharId = writable<number | null>(null);
export const nextDailyReset = writable<string>('');
export const updateAvailable = writable<boolean>(false);
export const currentAppVersion = writable<string>('');
export const latestAppVersion = writable<string | null>(null);
export const isUpdateChecking = writable<boolean>(false);
export const updateReleaseNotes = writable<string | null>(null);

// Derived Store - filter characters by active roster
export const activeCharacters = derived(
  [characters, activeRosterId],
  ([$characters, $activeRosterId]) => {
    if (!$activeRosterId) return [];
    return $characters.filter(char => char.roster_id === $activeRosterId);
  }
);

async function reloadCurrentRosterData() {
  const currentRosterId = get(activeRosterId);
  if (!currentRosterId) return;

  await loadCharacters(currentRosterId);
  await loadTodoMatrix(currentRosterId);
}

async function reloadCurrentTodoMatrix() {
  const currentRosterId = get(activeRosterId);
  if (currentRosterId) {
    await loadTodoMatrix(currentRosterId);
  }
}

async function handleEncounterSyncComplete(event: { payload: unknown }) {
  await reloadCurrentTodoMatrix();
  const payload = event.payload as { synced_count?: number } | null;
  if (payload?.synced_count && payload.synced_count > 0) {
    dispatchEvent(new CustomEvent('raid-completed'));
  }
}

// Listen for sync events from backend
export function initializeSyncEvents() {
  // Listen for sync completion events and refresh UI when backend data changes
  listen('sync-finished', reloadCurrentRosterData);

  listen('encounter-sync-complete', handleEncounterSyncComplete);
  listen('encounters-force-sync-complete', handleEncounterSyncComplete);
  listen('encounters-auto-sync-complete', handleEncounterSyncComplete);

  // Listen for sync error events
  listen('sync-error', (event) => {
    console.error('Sync error event received:', event.payload);
    // Could show a toast notification here
  });
}

// Load data from Tauri backend
export async function loadRosters() {
  try {
    const result = await invoke<Roster[]>('get_rosters');
    const uniqueRosters = Array.from(
      new Map(result.map(roster => [roster.id, roster])).values()
    ).sort((a, b) =>
      (a.roster_display_order ?? 0) - (b.roster_display_order ?? 0)
      || a.roster_name.localeCompare(b.roster_name)
    );
    rosters.set(uniqueRosters);
    
    // Sync all rosters to ensure raid data is initialized
    for (const roster of uniqueRosters) {
      try {
        await checkAndSyncRosterIfNeeded(roster.id);
      } catch (error) {
        console.error(`STORE: Failed to sync roster ${roster.id}:`, error);
      }
    }
    
    return uniqueRosters;
  } catch (error) {
    console.error('Failed to load rosters:', error);
    return [];
  }
}

export async function loadCharacters(rosterId?: string) {
  try {
    let result: Character[];
    if (rosterId) {
      result = await invoke<Character[]>('get_characters', { rosterId });
      
      // Sync character data including raids for this roster
      await checkAndSyncRosterIfNeeded(rosterId);
    } else {
      // For dashboard, load all characters from existing store
      result = get(characters);
    }
    
    const uniqueCharacters = Array.from(
      new Map(result.map(character => [character.char_id, character])).values()
    );
    characters.set(uniqueCharacters);
  } catch (error) {
    console.error('Failed to load characters:', error);
  }
}

export async function loadGameClasses() {
  try {
    // Load game classes directly from frontend data file (project rules: no hardcoded data in backend)
    gameClasses.set(buildStoreGameClasses(GAME_CLASSES));
  } catch (error) {
    console.error('Failed to load game classes:', error);
  }
}

export async function loadGameTasks() {
  try {
    // Load game tasks directly from frontend data file (project rules: no hardcoded data in backend)
    gameTasks.set(buildStoreGameTasks(GAME_TASKS));
  } catch (error) {
    console.error('Failed to load game tasks:', error);
  }
}

// Helper functions
export function addRoster(roster: Roster) {
  rosters.update(current => [...current, roster]);
  // Automatically set the new roster as active if no roster is currently active
  activeRosterId.update(current => current || roster.id);
}

export function addRosterAndSetActive(roster: Roster) {
  rosters.update(current => [...current, roster]);
  // Always set the new roster as active
  activeRosterId.set(roster.id);
}

export function removeRoster(rosterId: string) {
  rosters.update(current => current.filter(r => r.id !== rosterId));
  // If removed roster was active, clear active roster
  activeRosterId.update(current => current === rosterId ? '' : current);
}

export async function setActiveRoster(rosterId: string) {
  activeRosterId.set(rosterId);
  setActiveRosterPreference(rosterId);
  await loadCharacters(rosterId); // Load characters for the selected roster
}

export function addCharacter(character: Character) {
  characters.update(current => [...current, character]);
}

export function removeCharacter(charId: number) {
  characters.update(current => current.filter(c => c.char_id !== charId));
}

export async function updateCharacter(charId: number, updates: Partial<CharacterSettings>) {
  try {
    // Call backend handler with request object
    await invoke('update_character_settings', {
      request: {
        characterId: charId,
        settings: updates
      }
    });
    
    // Update local store for immediate UI feedback
    characters.update(all => {
      if (updates.removed_from_roster) {
        return all.filter(c => c.char_id !== charId);
      }
      return all.map(c => c.char_id === charId ? { ...c, ...updates } : c);
    });
  } catch (error) {
    console.error('STORE: Failed to save character settings:', error);
    throw error;
  }
}

// Sync roster configurations with frontend data
export async function syncRosterConfigs(rosterId: string) {
  try {
    // Call the new backend sync function
    await invoke('sync_roster_data', {
      rosterId: rosterId,
      tasks: buildSyncTaskPayload(GAME_TASKS),
      raids: buildSyncRaidPayload(RAIDS)
    });
  } catch (error) {
    console.error('STORE: Failed to sync roster configs:', error);
    throw error;
  }
}

// Check and sync roster if needed (for new rosters)
export async function checkAndSyncRosterIfNeeded(rosterId: string) {
  try {
    const result = await invoke('sync_roster_data', {
      rosterId: rosterId,
      tasks: buildSyncTaskPayload(GAME_TASKS),
      raids: buildSyncRaidPayload(RAIDS)
    });
    
    return result;
  } catch (error) {
    console.error(`STORE: Failed to check-and-sync roster ${rosterId}:`, error);
    throw error;
  }
}

// Update Todo Config with optimistic updates
export async function updateTodoConfig(charId: number, contentId: string, isTracked: boolean) {
  try {
    // Call new backend handler
    await invoke('update_task_status', {
      character_id: charId,
      task_id: contentId,
      tracked: isTracked,
      completed: false
    });
  } catch (error) {
    console.error(`STORE: Failed to update todo config:`, error);
    throw error;
  }
}

// Update Raid Config
export async function updateRaidConfig(charId: number, raidId: string, tracked: boolean) {
  try {
    // Call new backend handler
    await invoke('update_raid_config', {
      character_id: charId,
      raid_id: raidId,
      tracked: tracked
    });
  } catch (error) {
    console.error(`STORE: Failed to update raid config:`, error);
    throw error;
  }
}

// Mark task as completed
export async function markTaskCompleted(charId: number, contentId: string, completed: boolean) {
  try {
    await invoke('mark_task_completed', {
      character_id: charId,
      content_id: contentId,
      completed: completed
    });
  } catch (error) {
    console.error(`STORE: Failed to mark task completed:`, error);
    throw error;
  }
}

// Mark raid gate as completed
export async function markRaidGateCompleted(charId: number, raidId: string, gate: string, cleared: boolean) {
  try {
    await invoke('update_raid_gate_status', {
      character_id: charId,
      raid_id: raidId,
      gate: gate,
      cleared: cleared,
      clear_time: new Date().toISOString()
    });
    
    if (cleared) {
      dispatchEvent(new CustomEvent('raid-completed'));
    }
  } catch (error) {
    console.error(`STORE: Failed to mark raid gate completed:`, error);
    throw error;
  }
}

// Scrape roster from LostArk Bible
export async function scrapeRoster(rosterName: string) {
  try {
    const result = await invoke('scrape_roster', {
      rosterName: rosterName
    });
    
    // Reload rosters and characters after scraping
    await loadRosters();
    await loadCharacters(get(activeRosterId));
    
    return result;
  } catch (error) {
    console.error(`STORE: Failed to scrape roster:`, error);
    throw error;
  }
}

// Get system settings
export async function getSystemSettings() {
  try {
    const result = await invoke<SystemSettings>('get_system_settings');
    return result;
  } catch (error) {
    console.error('Failed to get system settings:', error);
    return {
      next_daily_reset: '',
      next_weekly_reset: '',
      current_theme: 'dark'
    };
  }
}

// Update system settings
export async function updateSystemSettings(settings: Partial<SystemSettings>) {
  try {
    await invoke('update_system_settings', settings);
  } catch (error) {
    console.error('Failed to update system settings:', error);
    throw error;
  }
}


// Load Todo Matrix data
export async function loadTodoMatrix(rosterId: string) {
  try {
    const result = await invoke<TodoMatrixResponse>('get_todo_matrix', { 
      rosterId 
    });
    
    todoMatrix.set(result);
    
    return result;
  } catch (error) {
    console.error('Failed to load todo matrix:', error);
    return null;
  }
}

// Update task status
export async function updateTaskStatus(characterId: number, taskId: string, tracked: boolean, completed: boolean) {
  try {
    await invoke('update_task_status', {
      characterId,
      taskId,
      tracked,
      completed
    });
    
    // Reload todo matrix to reflect changes
    const currentRosterId = get(activeRosterId);
    if (currentRosterId) {
      await loadTodoMatrix(currentRosterId);
    }
  } catch (error) {
    console.error('Failed to update task status:', error);
    throw error;
  }
}

// Update raid gate status
export async function updateRaidGateStatus(characterId: number, raidId: string, gate: string, cleared: boolean) {
  try {
    await invoke('update_raid_gate_status', {
      characterId,
      raidId,
      gate,
      cleared
    });
    
    // Reload todo matrix to reflect changes
    const currentRosterId = get(activeRosterId);
    if (currentRosterId) {
      await loadTodoMatrix(currentRosterId);
    }
  } catch (error) {
    console.error('Failed to update raid gate status:', error);
    throw error;
  }
}

export async function checkForAppUpdates() {
  try {
    isUpdateChecking.set(true);
    const updateData: any = await invoke('check_for_updates');
    currentAppVersion.set(updateData.current_version ?? '');
    latestAppVersion.set(updateData.latest_version ?? null);
    updateAvailable.set(!!updateData.update_available);
    updateReleaseNotes.set(updateData.body ?? null);
    return updateData;
  } catch (error) {
    console.error('Failed to fetch update info:', error);
    return null;
  } finally {
    isUpdateChecking.set(false);
  }
}

// Update roster task status
export async function updateRosterTaskStatus(rosterId: string, taskId: string, completed: boolean) {
  try {
    await invoke('update_roster_task_status', {
      rosterId,
      taskId,
      completed
    });
    
    // Reload todo matrix to reflect changes
    await loadTodoMatrix(rosterId);
  } catch (error) {
    console.error('Failed to update roster task status:', error);
    throw error;
  }
}

// Initialize app on startup
export async function initializeApp() {
  try {
    // Initialize sync events
    initializeSyncEvents();

    // Load bootstrap snapshot (rosters + all characters + reset info)
    const snapshot = await invoke<BootstrapSnapshot>('get_app_bootstrap_snapshot');
    rosters.set(snapshot.rosters ?? []);
    characters.set(snapshot.characters ?? []);
    nextDailyReset.set(snapshot.next_daily_reset ?? '');

    // Load static frontend-owned data
    await loadGameClasses();
    await loadGameTasks();
    
    // Restore active roster from localStorage
    const savedRosterId = getActiveRosterPreference();
    if (savedRosterId && (snapshot.rosters ?? []).some((roster) => roster.id === savedRosterId)) {
      activeRosterId.set(savedRosterId);
    } else if ((snapshot.rosters ?? []).length > 0) {
      activeRosterId.set(snapshot.rosters[0].id);
    }
  } catch (error) {
    console.error('Failed to initialize app:', error);
    throw error;
  }
}
