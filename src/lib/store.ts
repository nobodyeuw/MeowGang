
import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { GAME_TASKS } from './data/tasks';
import { RAIDS } from './data/raids';
import { GAME_CLASSES, type GameClass as FrontendGameClass } from '$lib/data/classes';
import { type GameTask as FrontendGameTask } from '$lib/data/tasks';

// Types matching our backend structure
export interface Roster {
  id: string;
  roster_name: string;
  last_updated?: string;
}

export interface CharacterSettings {
  earns_gold?: boolean;
  hide_from_dashboard?: boolean;
}

export interface Character {
  char_id: number;
  char_name: string;
  roster_id: string;
  roster_name: string;
  class_id: string;
  item_level: number;
  combat_power: number;
  display_order: number;
  earns_gold: boolean;
  hide_from_dashboard?: boolean;
  icon_id?: string;
  class_display_name?: string;
  last_active?: string;
}

export interface GameClass {
  id: string;
  display_name: string;
  icon_id: string;
}

export interface GameTask {
  id: string;
  name: string;
  category: string;
  reset_schedule: string;
  logic_type: string;
  max_rest_value?: number;
}

export interface Raid {
  id: string;
  name: string;
  difficulty: string;
  gates: RaidGate[];
}

export interface RaidGate {
  gate: string;
  min_ilvl: number;
  tradable_gold: number;
  bound_gold: number;
  box_price: number;
}

export interface TodoSettingsEntry {
  content_id: string;
  content_name: string;
  tracked: boolean;
  category: string;
  reset_schedule: string;
}

export interface RaidSettingsEntry {
  raid_id: string;
  raid_name: string;
  difficulty: string;
  take_gold: boolean;
  buy_box: boolean;
  gate_count: number;
  completion_status: number;
  max_difficulty: string;
}

export interface SystemSettings {
  loa_logs_path?: string;
  next_daily_reset: string;
  next_weekly_reset: string;
  current_theme: string;
}

export interface BootstrapSnapshot {
  rosters: Roster[];
  characters: Character[];
  next_daily_reset: string;
}

// Todo Matrix Types
export interface TodoMatrixResponse {
  characters: TodoCharacter[];
  daily_tasks: TodoTask[];
  roster_tasks: TodoTask[];
  weekly_tasks: TodoTask[];
  raids: TodoRaid[];
  character_states?: Record<string, CharacterTaskState>;
  rested_entries?: Array<[number, string, number]>;
  todo_entries?: Array<[number, string, boolean]>;
}

export interface TodoCharacter {
  id: number;
  name: string;
  class: string;
  ilvl: number;
  display_order: string;
}

export interface TodoTask {
  id: string;
  name: string;
  category: string;
  reset_schedule: string;
  logic_type: string;
  max_rest_value?: number;
  character_states: CharacterTaskState[];
}

export interface CharacterTaskState {
  tracked: boolean;
  completed: boolean;
  rested_value?: number;
  ilvl_too_low: boolean;
}

export interface TodoRaid {
  id: string;
  raid_name: string;
  difficulty: string;
  gates: TodoRaidGate[];
  character_states: CharacterRaidState[];
}

export interface TodoRaidGate {
  gate: string;
  name: string;
}

export interface CharacterRaidState {
  tracked: boolean;
  gate_states: RaidGateState[];
  ilvl_too_low: boolean;
}

export interface RaidGateState {
  gate: string;
  cleared: boolean;
  clear_time?: string;
}

export interface CharacterTaskState {
  tracked: boolean;
  completed: boolean;
}

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

// Listen for sync events from backend
export function initializeSyncEvents() {
  console.log('Initializing sync event listeners...');
  
  // Listen for sync completion events and refresh UI when backend data changes
  listen('sync-finished', async (event) => {
    console.log('Sync finished event received:', event.payload);
    const currentRosterId = get(activeRosterId);
    if (currentRosterId) {
      await loadCharacters(currentRosterId);
      await loadTodoMatrix(currentRosterId);
    }
  });

  listen('encounter-sync-complete', async (event) => {
    console.log('Encounter sync completed:', event.payload);
    const currentRosterId = get(activeRosterId);
    if (currentRosterId) {
      await loadTodoMatrix(currentRosterId);
    }
    const payload = event.payload as any;
    if (payload?.synced_count > 0) {
      try {
        await invoke('trigger_gold_processing');
        dispatchEvent(new CustomEvent('raid-completed'));
      } catch (e) {
        console.error('Gold processing after encounter sync failed:', e);
      }
    }
  });

  listen('encounters-force-sync-complete', async (event) => {
    console.log('Forced encounters sync completed:', event.payload);
    const currentRosterId = get(activeRosterId);
    if (currentRosterId) {
      await loadTodoMatrix(currentRosterId);
    }
    const payload = event.payload as any;
    if (payload?.synced_count > 0) {
      try {
        await invoke('trigger_gold_processing');
        dispatchEvent(new CustomEvent('raid-completed'));
      } catch (e) {
        console.error('Gold processing after force sync failed:', e);
      }
    }
  });

  listen('encounters-auto-sync-complete', async (event) => {
    console.log('Auto encounters sync event received:', event.payload);
    const currentRosterId = get(activeRosterId);
    if (currentRosterId) {
      await loadTodoMatrix(currentRosterId);
    }
    const payload = event.payload as any;
    if (payload?.synced_count > 0) {
      try {
        await invoke('trigger_gold_processing');
        dispatchEvent(new CustomEvent('raid-completed'));
      } catch (e) {
        console.error('Gold processing after auto sync failed:', e);
      }
    }
  });

  // Listen for sync error events
  listen('sync-error', (event) => {
    console.error('Sync error event received:', event.payload);
    // Could show a toast notification here
  });
}

// Load data from Tauri backend
export async function loadRosters() {
  try {
    console.log('Loading rosters from backend...');
    const result = await invoke<Roster[]>('get_rosters');
    console.log('Rosters loaded successfully:', result);
    rosters.set(result);
    console.log('Rosters store updated');
    
    // Sync all rosters to ensure raid data is initialized
    for (const roster of result) {
      console.log(`STORE: Syncing roster ${roster.id}...`);
      try {
        await checkAndSyncRosterIfNeeded(roster.id);
      } catch (error) {
        console.error(`STORE: Failed to sync roster ${roster.id}:`, error);
      }
    }
    
    return result;
  } catch (error) {
    console.error('Failed to load rosters:', error);
    return [];
  }
}

export async function loadCharacters(rosterId?: string) {
  try {
    console.log('Loading characters from backend...', { rosterId });
    
    let result: Character[];
    if (rosterId) {
      result = await invoke<Character[]>('get_characters', { rosterId });
      
      // Sync character data including raids for this roster
      await checkAndSyncRosterIfNeeded(rosterId);
    } else {
      // For dashboard, load all characters from existing store
      result = get(characters);
    }
    
    console.log('Characters loaded successfully:', result);
    characters.set(result);
    console.log('Characters store updated, new count:', result.length);
  } catch (error) {
    console.error('Failed to load characters:', error);
  }
}

export async function loadGameClasses() {
  try {
    // Load game classes directly from frontend data file (project rules: no hardcoded data in backend)
    // Convert GAME_CLASSES Record to array format expected by store
    const classesArray = Object.values(GAME_CLASSES).map((cls: FrontendGameClass) => ({
      id: cls.id,
      display_name: cls.displayName,
      icon_id: cls.iconId
    }));
    gameClasses.set(classesArray);
    console.log('Game classes loaded from frontend data:', classesArray.length);
  } catch (error) {
    console.error('Failed to load game classes:', error);
  }
}

export async function loadGameTasks() {
  try {
    // Load game tasks directly from frontend data file (project rules: no hardcoded data in backend)
    // Convert GAME_TASKS Record to array format expected by store
    const tasksArray = Object.values(GAME_TASKS).map((task: FrontendGameTask) => ({
      id: task.id,
      name: task.name,
      category: task.category,
      reset_schedule: task.reset_schedule,
      logic_type: task.logic_type,
      max_rest_value: task.max_rest_value
    }));
    gameTasks.set(tasksArray);
    console.log('Game tasks loaded from frontend data:', tasksArray.length);
  } catch (error) {
    console.error('Failed to load game tasks:', error);
  }
}

export async function debugDatabase() {
  try {
    console.log('Testing database connection...');
    const result = await invoke<string>('test_database_connection');
    console.log('Debug result:', result);
    return result;
  } catch (error) {
    console.error('Debug database failed:', error);
    throw error;
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
  console.log('setActiveRoster called with:', rosterId);
  activeRosterId.set(rosterId);
  localStorage.setItem('activeRosterId', rosterId);
  await loadCharacters(rosterId); // Load characters for the selected roster
}

export function addCharacter(character: Character) {
  characters.update(current => [...current, character]);
}

export function removeCharacter(charId: number) {
  characters.update(current => current.filter(c => c.char_id !== charId));
}

export async function updateCharacter(charId: number, updates: Partial<CharacterSettings>) {
  console.log(`STORE: updateCharacter called - charId: ${charId}, updates:`, updates);
  
  try {
    // Call backend handler with request object
    await invoke('update_character_settings', {
      request: {
        characterId: charId,
        settings: updates
      }
    });
    console.log(`STORE: Backend call successful for charId: ${charId}`);
    
    // Update local store for immediate UI feedback
    characters.update(all => all.map(c => 
      c.char_id === charId ? { ...c, ...updates } : c
    ));
    console.log(`STORE: Local store updated for charId: ${charId}`);
  } catch (error) {
    console.error('STORE: Failed to save character settings:', error);
    throw error;
  }
}

// Sync roster configurations with frontend data
export async function syncRosterConfigs(rosterId: string) {
  console.log(`=== FRONTEND SYNC DEBUG ===`);
  console.log(`STORE: Starting sync for roster: ${rosterId}`);
  
  try {
    // Convert frontend data to match our new backend structure
    const tasks = Object.fromEntries(
      Object.entries(GAME_TASKS).map(([key, task]) => [
        key,
        {
          id: task.id,
          name: task.name,
          category: task.category,
          reset_schedule: task.reset_schedule,
          logic_type: task.logic_type,
          max_rest_value: task.max_rest_value
        }
      ])
    );
    
    const raids = RAIDS.map(raid => ({
      id: raid.id,
      name: raid.name,
      difficulty: raid.difficulty,
      gates: raid.gates.map(gate => ({
        gate: gate.gate,
        min_ilvl: gate.minIlvl,
        tradable_gold: gate.tradableGold,
        bound_gold: gate.boundGold,
        box_price: gate.boxPrice
      }))
    }));
    
    console.log(`STORE: Syncing ${Object.keys(tasks).length} tasks and ${raids.length} raids`);
    
    // Call the new backend sync function
    await invoke('sync_roster_data', {
      rosterId: rosterId,
      tasks,
      raids
    });
    
    console.log(`STORE: Sync completed successfully for roster: ${rosterId}`);
    console.log(`=== FRONTEND SYNC COMPLETED ===`);
  } catch (error) {
    console.error('STORE: Failed to sync roster configs:', error);
    throw error;
  }
}

// Test function to manually sync roster
export async function testSyncRoster(rosterId: string) {
  console.log(`STORE: Manual sync test for roster ${rosterId}`);
  
  try {
    const { GAME_TASKS } = await import('$lib/data/tasks');
    const { RAIDS } = await import('$lib/data/raids');
    
    console.log(`STORE: Loaded ${Object.values(GAME_TASKS).length} tasks and ${RAIDS.length} raids`);
    
    const result = await invoke('sync_roster_data', {
      rosterId: rosterId,
      tasks: Object.fromEntries(
        Object.entries(GAME_TASKS).map(([key, task]) => [
          key,
          {
            id: task.id,
            name: task.name,
            category: task.category,
            reset_schedule: task.reset_schedule,
            logic_type: task.logic_type,
            max_rest_value: task.max_rest_value
          }
        ])
      ),
      raids: RAIDS
    });
    
    console.log(`STORE: Manual sync completed for roster ${rosterId}, result:`, result);
    return result;
  } catch (error) {
    console.error(`STORE: Failed to manual sync roster ${rosterId}:`, error);
    throw error;
  }
}

// Check and sync roster if needed (for new rosters)
export async function checkAndSyncRosterIfNeeded(rosterId: string) {
  console.log(`STORE: Checking if roster ${rosterId} needs sync...`);
  
  try {
    const { GAME_TASKS } = await import('$lib/data/tasks');
    const { RAIDS } = await import('$lib/data/raids');
    
    console.log(`STORE: Loaded ${Object.values(GAME_TASKS).length} tasks and ${RAIDS.length} raids`);
    
    const result = await invoke('sync_roster_data', {
      rosterId: rosterId,
      tasks: Object.fromEntries(
        Object.entries(GAME_TASKS).map(([key, task]) => [
          key,
          {
            id: task.id,
            name: task.name,
            category: task.category,
            reset_schedule: task.reset_schedule,
            logic_type: task.logic_type,
            max_rest_value: task.max_rest_value
          }
        ])
      ),
      raids: RAIDS
    });
    
    console.log(`STORE: Check-and-sync completed for roster ${rosterId}, result:`, result);
    return result;
  } catch (error) {
    console.error(`STORE: Failed to check-and-sync roster ${rosterId}:`, error);
    throw error;
  }
}

// Update Todo Config with optimistic updates
export async function updateTodoConfig(charId: number, contentId: string, isTracked: boolean) {
  console.log(`STORE: Updating todo config - char: ${charId}, content: ${contentId}, tracked: ${isTracked}`);
  
  try {
    // Call new backend handler
    await invoke('update_task_status', {
      character_id: charId,
      task_id: contentId,
      tracked: isTracked,
      completed: false
    });
    
    console.log(`STORE: Todo config updated successfully`);
  } catch (error) {
    console.error(`STORE: Failed to update todo config:`, error);
    throw error;
  }
}

// Update Raid Config
export async function updateRaidConfig(charId: number, raidId: string, tracked: boolean) {
  console.log(`STORE: Updating raid config - char: ${charId}, raid: ${raidId}, tracked: ${tracked}`);
  
  try {
    // Call new backend handler
    await invoke('update_raid_config', {
      character_id: charId,
      raid_id: raidId,
      tracked: tracked
    });
    
    console.log(`STORE: Raid config updated successfully`);
  } catch (error) {
    console.error(`STORE: Failed to update raid config:`, error);
    throw error;
  }
}

// Mark task as completed
export async function markTaskCompleted(charId: number, contentId: string, completed: boolean) {
  console.log(`STORE: Marking task completed - char: ${charId}, content: ${contentId}, completed: ${completed}`);
  
  try {
    await invoke('mark_task_completed', {
      character_id: charId,
      content_id: contentId,
      completed: completed
    });
    
    console.log(`STORE: Task completion status updated successfully`);
  } catch (error) {
    console.error(`STORE: Failed to mark task completed:`, error);
    throw error;
  }
}

// Mark raid gate as completed
export async function markRaidGateCompleted(charId: number, raidId: string, gate: string, cleared: boolean) {
  console.log(`STORE: Marking raid gate completed - char: ${charId}, raid: ${raidId}, gate: ${gate}, cleared: ${cleared}`);
  
  try {
    await invoke('update_raid_gate_status', {
      character_id: charId,
      raid_id: raidId,
      gate: gate,
      cleared: cleared,
      clear_time: new Date().toISOString()
    });
    
    console.log(`STORE: Raid gate completion status updated successfully`);
    
    // Trigger automatic gold processing for new raid completions
    if (cleared) {
      try {
        console.log('Triggering automatic gold processing for raid completion...');
        const goldResult = await invoke('trigger_gold_processing');
        console.log('Automatic gold processing result:', goldResult);
        
        // Dispatch event to refresh dashboard
        dispatchEvent(new CustomEvent('raid-completed'));
      } catch (goldError) {
        console.error('Failed to trigger automatic gold processing:', goldError);
      }
    }
  } catch (error) {
    console.error(`STORE: Failed to mark raid gate completed:`, error);
    throw error;
  }
}

// Scrape roster from LostArk Bible
export async function scrapeRoster(rosterName: string) {
  console.log(`STORE: Scraping roster: ${rosterName}`);
  
  try {
    const result = await invoke('scrape_roster', {
      rosterName: rosterName
    });
    
    console.log(`STORE: Roster scraped successfully:`, result);
    
    // Reload rosters and characters after scraping
    await loadRosters();
    await loadCharacters(get(activeRosterId));
    
    console.log(`STORE: New roster ${rosterName} added and initialized successfully.`);
    
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
    console.log('System settings updated successfully');
  } catch (error) {
    console.error('Failed to update system settings:', error);
    throw error;
  }
}


// Load Todo Matrix data
export async function loadTodoMatrix(rosterId: string) {
  try {
    console.log('Loading todo matrix from backend...', { rosterId });
    
    const result = await invoke<TodoMatrixResponse>('get_todo_matrix', { 
      rosterId 
    });
    
    console.log('Todo matrix loaded successfully:', result);
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
    console.log(`Updating task status - char: ${characterId}, task: ${taskId}, tracked: ${tracked}, completed: ${completed}`);
    
    await invoke('update_task_status', {
      characterId,
      taskId,
      tracked,
      completed
    });
    
    console.log('Task status updated successfully');
    
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
    console.log(`Updating raid gate status - char: ${characterId}, raid: ${raidId}, gate: ${gate}, cleared: ${cleared}`);
    
    await invoke('update_raid_gate_status', {
      characterId,
      raidId,
      gate,
      cleared
    });
    
    console.log('Raid gate status updated successfully');
    
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
    console.log(`Updating roster task status - roster: ${rosterId}, task: ${taskId}, completed: ${completed}`);
    
    await invoke('update_roster_task_status', {
      rosterId,
      taskId,
      completed
    });
    
    console.log('Roster task status updated successfully');
    
    // Reload todo matrix to reflect changes
    await loadTodoMatrix(rosterId);
  } catch (error) {
    console.error('Failed to update roster task status:', error);
    throw error;
  }
}

// Initialize app on startup
export async function initializeApp() {
  console.log('Initializing LOA Tracker app...');
  
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
    const savedRosterId = localStorage.getItem('activeRosterId') || '';
    if (savedRosterId && (snapshot.rosters ?? []).some((roster) => roster.id === savedRosterId)) {
      activeRosterId.set(savedRosterId);
    } else if ((snapshot.rosters ?? []).length > 0) {
      activeRosterId.set(snapshot.rosters[0].id);
    }
    
    console.log('App initialization completed successfully');
  } catch (error) {
    console.error('Failed to initialize app:', error);
    throw error;
  }
}
