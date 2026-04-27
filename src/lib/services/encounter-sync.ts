import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';

export interface SyncResult {
  synced_count: number;
  skipped_count: number;
  errors: string[];
  duration_ms: number;
}

export interface EncounterPreview {
  id: number;
  current_boss: string;
  local_player: string;
  difficulty: string;
  fight_start: number;
  cleared: boolean;
}

export interface EncounterMappingResult {
  content_id: string;
  gate: number;
  boss_names: string[];
}

export class EncounterSyncService {
  /**
   * Sync encounters from encounters.db to userlogs.db
   */
  static async syncEncountersToCompletions(): Promise<SyncResult> {
    return await invoke('sync_encounters_to_completions');
  }

  /**
   * Get preview of encounters from encounters.db
   */
  static async getEncountersPreview(limit?: number): Promise<EncounterPreview[]> {
    return await invoke('get_encounters_preview', { 
      limit 
    });
  }

  /**
   * Test boss mapping functionality
   */
  static async testBossMapping(bossName: string): Promise<EncounterMappingResult | null> {
    return await invoke('test_boss_mapping', { 
      bossName 
    });
  }

  /**
   * Force sync encounters immediately
   */
  static async forceEncountersSync(encountersDbPath: string): Promise<SyncResult> {
    return await invoke('force_encounters_sync', { 
      encountersDbPath 
    });
  }

  /**
   * Listen for sync events
   */
  static async listenToSyncEvents(callback: (event: string, payload: any) => void): Promise<() => void> {
    const unlistenPromises = await Promise.all([
      listen('encounter-sync-complete', (event) => callback('encounter-sync-complete', event?.payload || {})),
      listen('encounter-sync-progress', (event) => callback('encounter-sync-progress', event?.payload || {})),
      listen('encounter-sync-complete', (event) => callback('encounter-sync-complete', event?.payload || {})),
      listen('encounters-force-sync-complete', (event) => callback('encounters-force-sync-complete', event?.payload || {})),
      listen('encounters-auto-sync-complete', (event) => callback('encounters-auto-sync-complete', event?.payload || {})),
      listen('encounters-auto-sync-error', (event) => callback('encounters-auto-sync-error', event?.payload || {})),
      listen('encounter-sync-error', (event) => callback('encounter-sync-error', event?.payload || {}))
    ]);
    
    return () => {
      unlistenPromises.forEach(unlisten => unlisten());
    };
  }
}
