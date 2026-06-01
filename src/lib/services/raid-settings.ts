import { invoke } from '@tauri-apps/api/core';

export interface RaidGateConfigUpdate {
  rosterId: string;
  charId: number;
  contentId: string;
  gate: string;
  difficulty: string;
  takeGold?: boolean;
  buyBox?: boolean;
  reservedForStatic?: boolean;
}

export interface RaidMatrixCommandData {
  characters: any[];
  raid_configs: any[];
  trackingStates: any[];
}

// Tauri command boundary for Settings > Raids.
export async function loadRaidSettingsMatrix(rosterId: string): Promise<RaidMatrixCommandData> {
  const [raidMatrix, trackingMatrix] = await Promise.all([
    invoke<{ characters: any[]; raid_configs: any[] }>('get_raid_matrix_data', { rosterId }),
    invoke<any>('get_tracking_config_matrix', {
      rosterId,
      tasks: [],
      raids: []
    })
  ]);

  return {
    characters: raidMatrix.characters,
    raid_configs: raidMatrix.raid_configs,
    trackingStates: trackingMatrix?.character_states || []
  };
}

export function updateRaidGateConfigCommand(update: RaidGateConfigUpdate): Promise<void> {
  return invoke('update_raid_gate_config', update as unknown as Record<string, unknown>);
}
