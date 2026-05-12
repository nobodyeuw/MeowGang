<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { activeRosterId, rosters } from '$lib/store';
  import { RAIDS } from '$lib/data/raids';
  import { GAME_CLASSES } from '$lib/data/classes';

  // Interfaces
  interface RaidGate {
    gate: string;
    minIlvl: number;
    tradableGold: number;
    boundGold: number;
    boxPrice: number;
  }

  interface Raid {
    id: string;
    name: string;
    difficulty: string;
    gates: RaidGate[];
  }

  interface RaidGroup {
    content_id: string;
    raid_name: string;
    difficulties: Map<string, Raid>;
    gates: Map<string, Map<string, RaidGate>>;
  }

  interface RaidConfig {
    content_id: string;
    gates: any[];
    take_gold: boolean;
    buy_box: boolean;
  }

  interface CharacterRaidConfig {
    char_id: number;
    char_name: string;
    item_level: number;
    combat_power: number;
    class_id: string;
    earns_gold: boolean;
    raid_configs: RaidConfig[];
    available_difficulties: string[];
    master_difficulty: string;
    is_locked: boolean;
    gold_values: {
      totalGold: number;
      tradableGold: number;
      boundGold: number;
      boxPrice: number;
    };
  }

  interface RaidMatrixData extends RaidGroup {
    characters: CharacterRaidConfig[];
    is_expanded: boolean;
    unique_key?: string;
  }

  // State
  let raidMatrix: RaidMatrixData[] = [];
  let isLoading = true;
  let error = '';
  let successMessage = '';
  let showSuccessMessage = false;
  let expandedRaids = new Set<string>();
  
  // Reactive checkbox states to force UI updates
  let checkboxUpdateTrigger = 0;

  // Additional reactive variables for the new structure
  $: matrixData = {
    character_states: raidMatrix.length > 0 ? raidMatrix[0].characters : []
  };
  
  let raidsToDisplay = RAIDS;

  // Group raids by content_id and process data
  function processRaidData() {
    const raidGroups = new Map<string, RaidGroup>();
    
    // Group raids by content_id
    RAIDS.forEach(raid => {
      if (!raidGroups.has(raid.id)) {
        raidGroups.set(raid.id, {
          content_id: raid.id,
          raid_name: raid.name,
          difficulties: new Map(),
          gates: new Map()
        });
      }
      
      const group = raidGroups.get(raid.id)!;
      group.difficulties.set(raid.difficulty, raid);
      
      // Group gates by gate name
      raid.gates.forEach(gate => {
        if (!group.gates.has(gate.gate)) {
          group.gates.set(gate.gate, new Map());
        }
        group.gates.get(gate.gate)!.set(raid.difficulty, gate);
      });
    });
    
    return Array.from(raidGroups.values()).sort((a, b) => {
      // Sort by raid_name to match tracking order
      if (a.raid_name < b.raid_name) return -1;
      if (a.raid_name > b.raid_name) return 1;
      return 0;
    });
  }

  // Load raid configuration from database
  async function loadRaidConfiguration() {
    try {
      isLoading = true;
      
      if (!$activeRosterId) {
        throw new Error('No active roster selected');
      }
      
      // Get characters and their raid configurations
      const result = await invoke('get_raid_matrix_data', { 
        rosterId: $activeRosterId 
      });
      
      const { characters, raid_configs } = result as { characters: any[], raid_configs: any[] };
      
      // Check if raid_configs is defined
      if (!raid_configs || !Array.isArray(raid_configs)) {
        console.error('ERROR: raid_configs is not an array:', raid_configs);
        throw new Error('Invalid raid_configs data from backend');
      }
      
      const raidGroups = processRaidData();
      
      // Transform raids and sort by min_ilvl, grouping by base name
    const raidsMap = new Map<string, Raid[]>();
    [...RAIDS].forEach(raid => {
      const baseName = raid.name;
      if (!raidsMap.has(baseName)) {
        raidsMap.set(baseName, []);
      }
      raidsMap.get(baseName)!.push(raid);
    });
    
    // Sort raids within each group by minIlvl and then sort groups by lowest minIlvl
    const sortedRaidGroups = Array.from(raidsMap.entries()).map(([baseName, raids]) => {
      const sortedRaids = raids.sort((a: Raid, b: Raid) => {
        const aMinIlvl = a.gates[0]?.minIlvl || 0;
        const bMinIlvl = b.gates[0]?.minIlvl || 0;
        return aMinIlvl - bMinIlvl;
      });
      return { baseName, raids: sortedRaids };
    }).sort((a, b) => {
      const aMinIlvl = a.raids[0]?.gates[0]?.minIlvl || 0;
      const bMinIlvl = b.raids[0]?.gates[0]?.minIlvl || 0;
      return aMinIlvl - bMinIlvl;
    });

    // Build raid groups - ONE ROW PER RAID with all difficulties
    const allRaidGroups: RaidGroup[] = sortedRaidGroups.map(({ raids, baseName }) => {
      const group: RaidGroup = {
        content_id: raids[0].id, // Use the first raid's ID
        raid_name: baseName,
        difficulties: new Map(),
        gates: new Map()
      };
      
      // Add ALL difficulties to this single group
      raids.forEach((raid: Raid) => {
        group.difficulties.set(raid.difficulty, raid);
        raid.gates.forEach((gate: RaidGate) => {
          if (!group.gates.has(gate.gate)) {
            group.gates.set(gate.gate, new Map());
          }
          group.gates.get(gate.gate)!.set(raid.difficulty, gate);
        });
      });
      
      return group;
    });

      // Build matrix - one row per raid with all difficulties
      raidMatrix = allRaidGroups.map(raidGroup => {
        // Add unique key for Svelte each block
        (raidGroup as any).unique_key = raidGroup.content_id;
        
        const characterConfigs = characters.map((char: any) => {
          const charRaidConfig: any = raid_configs.find((config: any) => config.char_id === char.char_id);
          
          if (!charRaidConfig) {
            return {
              char_id: char.char_id,
              char_name: char.char_name,
              item_level: char.item_level,
              combat_power: char.combat_power || 0,
              class_id: char.class_id,
              earns_gold: char.earns_gold,
              raid_configs: [],
              available_difficulties: [],
              master_difficulty: '',
              is_locked: true,
              gold_values: { totalGold: 0, tradableGold: 0, boundGold: 0, boxPrice: 0 }
            };
          }
          
          // Get available difficulties based on item level
          const availableDifficulties = getAvailableDifficulties(raidGroup, char.item_level);
          
          // Get current difficulty state
          const masterDifficulty = getMasterDifficulty(raidGroup, charRaidConfig.raid_configs);
          
          // Calculate gold values for this character and raid
          const getGoldValues = () => {
            // Get the current difficulty for this character from the first gate
            const raidConfig = charRaidConfig.raid_configs.find((r: any) => r.content_id === raidGroup.content_id);
            const currentDifficulty = raidConfig?.gates?.[0]?.difficulty || 'Solo';
            const currentRaidData = raidGroup.difficulties.get(currentDifficulty);
            
            if (!currentRaidData) return { totalGold: 0, tradableGold: 0, boundGold: 0, boxPrice: 0 };
            
            const totalGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.tradableGold || 0) + (gate.boundGold || 0), 0) || 0;
            const tradableGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.tradableGold || 0), 0) || 0;
            const boundGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.boundGold || 0), 0) || 0;
            const boxPrice = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.boxPrice || 0), 0) || 0;
            
            return { totalGold, tradableGold, boundGold, boxPrice };
          };
          
          const goldValues = getGoldValues();
          
          return {
            char_id: char.char_id,
            char_name: char.char_name,
            item_level: char.item_level,
            combat_power: char.combat_power || 0,
            class_id: char.class_id,
            earns_gold: char.earns_gold,
            raid_configs: charRaidConfig.raid_configs,
            available_difficulties: availableDifficulties,
            master_difficulty: masterDifficulty,
            is_locked: availableDifficulties.length === 0,
            gold_values: goldValues
          };
        });
        
        return {
          ...raidGroup,
          characters: characterConfigs,
          is_expanded: expandedRaids.has(raidGroup.content_id)
        };
      });
      
    } catch (err) {
      error = `Failed to load raid configuration: ${err}`;
      console.error(error);
    } finally {
      isLoading = false;
    }
  }

  // Get available difficulties for a character
  function getAvailableDifficulties(raidGroup: RaidGroup, itemLevel: number): string[] {
    const difficulties: string[] = [];
    
    raidGroup.difficulties.forEach((raid, difficulty) => {
      const canDoRaid = raid.gates.some(gate => gate.minIlvl <= itemLevel);
      if (canDoRaid) {
        difficulties.push(difficulty);
      }
    });
    
    // Sort by difficulty priority
    const priority: Record<string, number> = { 'Hard': 3, 'Normal': 2, 'Solo': 1, 'Nightmare': 4 };
    return difficulties.sort((a, b) => (priority[b] || 0) - (priority[a] || 0));
  }

  // Get master difficulty for a raid configuration
  function getMasterDifficulty(raidGroup: RaidGroup, raidConfigs: RaidConfig[]): string {
    const raidConfig = raidConfigs.find((config: any) => config.content_id === raidGroup.content_id);
    
    if (!raidConfig || raidConfig.gates.length === 0) {
      return '';
    }
    
    const difficulties = raidConfig.gates.map((gate: any) => gate.difficulty);
    const uniqueDifficulties = [...new Set(difficulties)];
    
    if (uniqueDifficulties.length === 1) {
      return uniqueDifficulties[0];
    }
    
    // If any gate is Solo, all must be Solo
    if (uniqueDifficulties.includes('Solo')) {
      return 'Solo';
    }
    
    return 'Mixed';
  }

  // Check if character has mixed difficulties for a specific raid
  function hasMixedDifficulties(char: any, raidId: string): boolean {
    const raidConfig = char.raid_configs.find((r: any) => r.content_id === raidId);
    if (!raidConfig || !raidConfig.gates || raidConfig.gates.length === 0) return false;
    
    const difficulties = raidConfig.gates.map((gate: any) => gate.difficulty);
    const uniqueDifficulties = [...new Set(difficulties)];
    
    // If Solo is included, it's not mixed (Solo overrides everything)
    if (uniqueDifficulties.includes('Solo')) return false;
    
    return uniqueDifficulties.length > 1;
  }

  // Get total gold for a raid at specific difficulty
  function getRaidGoldValues(char: any, raidId: string) {
    const raidConfig = char.raid_configs.find(r => r.content_id === raidId);
    if (!raidConfig || !raidConfig.gates || raidConfig.gates.length === 0) {
      return { totalGold: 0, tradableGold: 0, boundGold: 0, boxPrice: 0 };
    }
    
    // Calculate gold based on actual gate difficulties
    let totalGold = 0;
    let tradableGold = 0;
    let boundGold = 0;
    let boxPrice = 0;
    
    raidConfig.gates.forEach(gate => {
      const gateData = getGateData(raidId, gate.difficulty || 'Solo', gate.gate);
      if (gateData) {
        // Always show potential gold values (not dependent on take_gold status)
        totalGold += gateData.tradableGold + gateData.boundGold;
        tradableGold += gateData.tradableGold;
        boundGold += gateData.boundGold;
        // Box price only if buy_box is active
        if (gate.buy_box) {
          boxPrice += gateData.boxPrice;
        }
      }
    });
    
    return { totalGold, tradableGold, boundGold, boxPrice };
  }

  // Get potential box price for all gates (always show full amount)
  function getPotentialBoxPrice(char: any, raidId: string): number {
    const raidConfig = char.raid_configs.find(r => r.content_id === raidId);
    if (!raidConfig || !raidConfig.gates || raidConfig.gates.length === 0) {
      return 0;
    }
    
    let boxPrice = 0;
    raidConfig.gates.forEach(gate => {
      const gateData = getGateData(raidId, gate.difficulty || 'Solo', gate.gate);
      if (gateData) {
        boxPrice += gateData.boxPrice;
      }
    });
    
    return boxPrice;
  }

  // Get potential gold amount for all gates (always show full amount)
  function getPotentialGoldAmount(char: any, raidId: string): number {
    const raidConfig = char.raid_configs.find(r => r.content_id === raidId);
    if (!raidConfig || !raidConfig.gates || raidConfig.gates.length === 0) {
      return 0;
    }
    
    let goldAmount = 0;
    raidConfig.gates.forEach(gate => {
      const gateData = getGateData(raidId, gate.difficulty || 'Solo', gate.gate);
      if (gateData) {
        goldAmount += gateData.tradableGold + gateData.boundGold;
      }
    });
    
    return goldAmount;
  }

  // Toggle raid expansion
  function toggleRaidExpansion(contentId: string) {
    if (expandedRaids.has(contentId)) {
      expandedRaids.delete(contentId);
    } else {
      expandedRaids.add(contentId);
    }
    
    // Update matrix state
    raidMatrix = raidMatrix.map(raid => ({
      ...raid,
      is_expanded: expandedRaids.has(raid.content_id)
    }));
  }

  // Change difficulty for a specific gate
  async function changeGateDifficulty(contentId: string, charId: number, gateName: string, newDifficulty: string) {
    console.log('=== CHANGE GATE DIFFICULTY DEBUG ===');
    console.log('contentId:', contentId);
    console.log('charId:', charId);
    console.log('gateName:', gateName);
    console.log('newDifficulty:', newDifficulty);
    
    // Find character across all raid groups
    const characterRaids = raidMatrix
      .flatMap(raidGroup => raidGroup.characters)
      .filter(c => c.char_id === charId);
    
    const char = characterRaids[0];
    if (!char) return;

    // Find raid configuration
    const raidConfig = char.raid_configs.find(r => r.content_id === contentId);
    if (!raidConfig || !raidConfig.gates) {
      console.log('ERROR: Raid config not found');
      return;
    }

    // Find specific gate config
    const gateConfig = raidConfig.gates.find(g => g.gate === gateName);
    if (!gateConfig) {
      console.log('ERROR: Gate config not found:', gateName);
      return;
    }

    // Store old difficulty for reactivity
    const oldDifficulty = gateConfig.difficulty;
    
    // Update gate difficulty in local state
    gateConfig.difficulty = newDifficulty;

    // Force Svelte 5 reactivity
    raidMatrix = raidMatrix.map(raidGroup => ({
      ...raidGroup,
      characters: raidGroup.characters.map(char => ({
        ...char,
        raid_configs: char.raid_configs.map(config => ({
          ...config,
          gates: config.gates ? [...config.gates] : []
        }))
      }))
    }));

    // Backend call
    try {
      await invoke('update_raid_gate_config', {
        rosterId: $activeRosterId,
        charId,
        contentId,
        gate: gateName,
        difficulty: newDifficulty,
        takeGold: gateConfig.take_gold,
        buyBox: gateConfig.buy_box
      });
      
      // Check if we need to update all gates due to Solo logic
      const raidConfig = char.raid_configs.find(r => r.content_id === contentId);
      if (raidConfig && raidConfig.gates) {
        const hasSoloGates = raidConfig.gates.some(gate => gate.difficulty === 'Solo');
        
        // If Solo is selected, ALL gates must be Solo
        if (newDifficulty === 'Solo') {
          await changeMasterDifficulty(contentId, charId, 'Solo');
        }
        // If we're changing FROM Solo to something else, ALL gates must change
        else if (hasSoloGates && oldDifficulty === 'Solo' && newDifficulty !== 'Solo') {
          await changeMasterDifficulty(contentId, charId, newDifficulty);
        }
        // Normal difficulty change - update master difficulty if all gates now have the same difficulty
        else {
          updateMasterDifficulty(charId, contentId);
        }
      }
      
    } catch (err) {
      console.error('Failed to update gate difficulty:', err);
      // Revert on error
      gateConfig.difficulty = oldDifficulty;
      
      // Force reactivity again
      raidMatrix = raidMatrix.map(raidGroup => ({
        ...raidGroup,
        characters: raidGroup.characters.map(char => ({
          ...char,
          raid_configs: char.raid_configs.map(config => ({
            ...config,
            gates: config.gates ? [...config.gates] : []
          }))
        }))
      }));
    }
  }

  // Change difficulty for ALL gates in a raid (master row)
  async function changeMasterDifficulty(contentId: string, charId: number, newDifficulty: string) {
    console.log('=== CHANGE MASTER DIFFICULTY DEBUG ===');
    console.log('contentId:', contentId);
    console.log('charId:', charId);
    console.log('newDifficulty:', newDifficulty);
    
    // Find character across all raid groups
    const characterRaids = raidMatrix
      .flatMap(raidGroup => raidGroup.characters)
      .filter(c => c.char_id === charId);
    
    const char = characterRaids[0];
    if (!char) return;

    // Find raid configuration
    const raidConfig = char.raid_configs.find(r => r.content_id === contentId);
    if (!raidConfig || !raidConfig.gates) {
      console.log('ERROR: Raid config not found');
      return;
    }

    // Update ALL gates to the new difficulty
    raidConfig.gates.forEach(gate => {
      gate.difficulty = newDifficulty;
    });

    // Force Svelte 5 reactivity
    raidMatrix = raidMatrix.map(raidGroup => ({
      ...raidGroup,
      characters: raidGroup.characters.map(char => ({
        ...char,
        raid_configs: char.raid_configs.map(config => ({
          ...config,
          gates: config.gates ? [...config.gates] : []
        }))
      }))
    }));

    // Backend calls for all gates
    try {
      const promises = raidConfig.gates.map(gate =>
        invoke('update_raid_gate_config', {
          rosterId: $activeRosterId,
          charId,
          contentId,
          gate: gate.gate,
          difficulty: newDifficulty,
          takeGold: gate.take_gold,
          buyBox: gate.buy_box
        })
      );
      
      await Promise.all(promises);
      
      // Update master difficulty
      updateMasterDifficulty(charId, contentId);
      
    } catch (err) {
      console.error('Failed to update master difficulty:', err);
    }
  }

  // Update master difficulty for a character's raid
  function updateMasterDifficulty(charId: number, contentId: string) {
    const characterRaids = raidMatrix
      .flatMap(raidGroup => raidGroup.characters)
      .filter(c => c.char_id === charId);
    
    const char = characterRaids[0];
    if (!char) return;

    const raidConfig = char.raid_configs.find(r => r.content_id === contentId);
    if (!raidConfig || !raidConfig.gates) return;

    // Find the raid group to update master_difficulty
    const raidGroup = raidMatrix.find(r => r.content_id === contentId);
    if (!raidGroup) return;

    const charIndex = raidGroup.characters.findIndex(c => c.char_id === charId);
    if (charIndex === -1) return;

    // Update master_difficulty based on gate difficulties
    raidGroup.characters[charIndex].master_difficulty = getMasterDifficulty(raidGroup, char.raid_configs);

    // Force reactivity
    raidMatrix = raidMatrix.map(raidGroup => ({
      ...raidGroup,
      characters: raidGroup.characters.map(char => ({
        ...char,
        raid_configs: char.raid_configs.map(config => ({
          ...config,
          gates: config.gates ? [...config.gates] : []
        }))
      }))
    }));
  }

  // Toggle buy box for a specific gate
  async function toggleGateBox(contentId: string, charId: number, gateName: string, currentValue: boolean) {
    console.log('=== TOGGLE GATE BOX DEBUG ===');
    console.log('contentId:', contentId);
    console.log('charId:', charId);
    console.log('gateName:', gateName);
    console.log('currentValue:', currentValue);
    
    try {
      const raidGroup = raidMatrix.find(r => r.content_id === contentId);
      if (!raidGroup) {
        console.error('ERROR: Raid group not found:', contentId);
        return;
      }
      
      // Get current raid config for this character
      const charRaidConfig = raidGroup.characters.find(c => c.char_id === charId);
      if (!charRaidConfig) {
        console.error('ERROR: Character raid config not found:', charId);
        return;
      }
      
      const raidConfig = charRaidConfig.raid_configs.find(r => r.content_id === contentId);
      if (!raidConfig) {
        console.error('ERROR: Raid config not found:', contentId);
        return;
      }
      
      // Find the specific gate
      const gateConfig = raidConfig.gates.find(g => g.gate === gateName);
      if (!gateConfig) {
        console.error('ERROR: Gate config not found:', gateName);
        return;
      }
      
      console.log('Current gate buy_box:', gateConfig.buy_box);
      
      // Update gate buy_box
      gateConfig.buy_box = !currentValue;
      console.log('Updated gate buy_box to:', gateConfig.buy_box);
      
      // Save updated configuration
      console.log('Calling update_raid_gate_config...');
      await invoke('update_raid_gate_config', {
        rosterId: $activeRosterId,
        charId,
        contentId,
        gate: gateName,
        difficulty: gateConfig.difficulty,
        takeGold: gateConfig.take_gold,
        buyBox: gateConfig.buy_box
      });
      console.log('update_raid_gate_config completed');
      
      // Trigger gold processing to update dashboard immediately
      try {
        console.log('Triggering gold processing after buy box change...');
        const goldResult = await invoke('trigger_gold_processing');
        console.log('Gold processing result:', goldResult);
        
        // Dispatch event to refresh dashboard
        console.log('DEBUG: Dispatching raid-settings-updated event');
        dispatchEvent(new CustomEvent('raid-settings-updated'));
        console.log('DEBUG: Event dispatched successfully');
      } catch (goldError) {
        console.error('Failed to trigger gold processing:', goldError);
      }
      
      // Update only this character's gold values without full reload
      const raidGroupIndex = raidMatrix.findIndex(r => r.content_id === contentId);
      if (raidGroupIndex !== -1) {
        const charIndex = raidMatrix[raidGroupIndex].characters.findIndex(c => c.char_id === charId);
        if (charIndex !== -1) {
          // Recalculate gold values for this character
          const raidGroup = raidMatrix[raidGroupIndex];
          const character = raidMatrix[raidGroupIndex].characters[charIndex];
          
          const getGoldValues = () => {
            const currentDifficulty = character.raid_configs.find((r: any) => r.content_id === raidGroup.content_id)?.difficulty || 'Solo';
            const currentRaidData = raidGroup.difficulties.get(currentDifficulty);
            if (!currentRaidData) return { totalGold: 0, tradableGold: 0, boundGold: 0, boxPrice: 0 };
            
            const totalGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.tradableGold || 0) + (gate.boundGold || 0), 0) || 0;
            const tradableGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.tradableGold || 0), 0) || 0;
            const boundGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.boundGold || 0), 0) || 0;
            const boxPrice = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.boxPrice || 0), 0) || 0;
            
            return { totalGold, tradableGold, boundGold, boxPrice };
          };
          
          character.gold_values = getGoldValues();
          
          // Trigger reactivity
          // Force Svelte 5 reactivity with deeper assignment
    raidMatrix = raidMatrix.map(raidGroup => ({
      ...raidGroup,
      characters: raidGroup.characters.map(char => ({
        ...char,
        raid_configs: char.raid_configs.map(config => ({
          ...config,
          gates: config.gates ? [...config.gates] : []
        }))
      }))
    }));
        }
      }
      
      console.log('Gate box toggled without full reload');
    } catch (err) {
      console.error('ERROR in toggleGateBox:', err);
      error = `Failed to update gate box: ${err}`;
      console.error(error);
    }
  }

  // Change difficulty for a character's raid
  async function changeDifficulty(contentId: string, charId: number, newDifficulty: string) {
    try {
      const raidGroup = raidMatrix.find(r => r.content_id === contentId);
      if (!raidGroup) return;
      
      // If Solo is selected, apply to all gates
      if (newDifficulty === 'Solo') {
        await updateAllGatesDifficulty(contentId, charId, 'Solo');
      } else {
        // For Normal/Hard, we need to handle individual gate updates
        // For now, update all gates to new difficulty
        await updateAllGatesDifficulty(contentId, charId, newDifficulty);
      }
      
      // Update gold values and master_difficulty for the specific character
      const raidGroupIndex = raidMatrix.findIndex(r => r.content_id === contentId);
      if (raidGroupIndex !== -1) {
        const raidGroup = raidMatrix[raidGroupIndex];
        const characterIndex = raidGroup.characters.findIndex(c => c.char_id === charId);
        
        if (characterIndex !== -1) {
          const character = raidGroup.characters[characterIndex];
          
          // Update master_difficulty
          const raidConfig = character.raid_configs.find(r => r.content_id === contentId);
          if (raidConfig) {
            character.master_difficulty = getMasterDifficulty(raidGroup, character.raid_configs);
          }
          
          // Recalculate gold values for this character
          const getGoldValues = () => {
            const currentDifficulty = raidConfig?.gates?.[0]?.difficulty || 'Solo';
            const currentRaidData = raidGroup.difficulties.get(currentDifficulty);
            if (!currentRaidData) return { totalGold: 0, tradableGold: 0, boundGold: 0, boxPrice: 0 };
            
            const totalGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.tradableGold || 0) + (gate.boundGold || 0), 0) || 0;
            const tradableGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.tradableGold || 0), 0) || 0;
            const boundGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.boundGold || 0), 0) || 0;
            const boxPrice = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.boxPrice || 0), 0) || 0;
            
            return { totalGold, tradableGold, boundGold, boxPrice };
          };
          
          character.gold_values = getGoldValues();
          
          // Trigger reactivity
          // Force Svelte 5 reactivity with deeper assignment
    raidMatrix = raidMatrix.map(raidGroup => ({
      ...raidGroup,
      characters: raidGroup.characters.map(char => ({
        ...char,
        raid_configs: char.raid_configs.map(config => ({
          ...config,
          gates: config.gates ? [...config.gates] : []
        }))
      }))
    }));
        }
      }
    } catch (err) {
      error = `Failed to update difficulty: ${err}`;
      console.error(error);
    }
  }

  // Update all gates difficulty
  async function updateAllGatesDifficulty(contentId: string, charId: number, difficulty: string) {
    const raidGroup = raidMatrix.find(r => r.content_id === contentId);
    if (!raidGroup) return;
    
    const raidData = raidGroup.difficulties.get(difficulty);
    if (!raidData) return;
    
    // Update each gate
    for (const gateName of raidGroup.gates.keys()) {
      const gateData = raidData.gates.find(g => g.gate === gateName);
      if (gateData) {
        await invoke('update_raid_gate_config', {
          rosterId: $activeRosterId,
          charId,
          contentId,
          gate: gateName,
          difficulty,
          takeGold: false,
          buyBox: false
        });
      }
    }
  }

  
  // Helper function to get gate data from RAIDS
  function getGateData(raidId: string, difficulty: string, gateName: string) {
    const raid = RAIDS.find(r => r.id === raidId && r.difficulty === difficulty);
    return raid?.gates.find(g => g.gate === gateName) || null;
  }

  // Helper function to check if master row is active (all gates have the same status)
  function isMasterActive(char: any, raidId: string, difficulty: string, type: 'take_gold' | 'buy_box'): boolean {
    if (!char || !char.raid_configs) return false;
    
    const raidConfig = char.raid_configs.find((r: any) => r.content_id === raidId);
    if (!raidConfig || !raidConfig.gates || raidConfig.gates.length === 0) return false;
    
    // Active if ALL gates have the same status
    return raidConfig.gates.every((gate: any) => gate[type] === true);
  }

  // Reactive version of isMasterActive that triggers on checkboxUpdateTrigger
  function isMasterActiveReactive(char: any, raidId: string, difficulty: string, type: 'take_gold' | 'buy_box'): boolean {
    // This forces reactivity when checkboxUpdateTrigger changes
    checkboxUpdateTrigger;
    return isMasterActive(char, raidId, difficulty, type);
  }

  // Helper function to get master raid state from gates
  function getMasterRaidState(char: any, raidId: string, type: 'take_gold' | 'buy_box'): boolean {
    if (!char || !char.raid_configs) return false;
    
    const config = char.raid_configs.find((r: any) => r.content_id === raidId);
    
    if (!config) return false;
    
    // First try gates array (preferred method)
    if (config.gates && config.gates.length > 0) {
      return config.gates[0][type] || false;
    }
    
    // Fallback to raid config properties
    return config[type] || false;
  }

  // Force reactivity helper for master row toggles
  function forceMasterRowUpdate() {
    // This function forces Svelte to re-evaluate all master row checkbox states
    raidMatrix = raidMatrix.map(raidGroup => ({
      ...raidGroup,
      characters: raidGroup.characters.map(char => ({
        ...char,
        raid_configs: char.raid_configs.map(config => ({
          ...config,
          gates: config.gates ? [...config.gates] : []
        }))
      }))
    }));
    
    // Force checkbox UI update
    checkboxUpdateTrigger++;
  }

  // Check if character has reached gold limit (for disabling checkboxes)
  function hasReachedGoldLimit(char: any): boolean {
    if (!char.earns_gold) return false; // Non-gold earners have no limit
    
    const currentGoldRaidsCount = char.raid_configs.filter(r => 
      r.gates && r.gates.some(gate => gate.take_gold === true)
    ).length;
    
    return currentGoldRaidsCount >= 3;
  }

  // Check if this specific raid is already active (for enabling/disabling)
  function isRaidAlreadyActive(char: any, raidId: string): boolean {
    const raidConfig = char.raid_configs.find(r => r.content_id === raidId);
    return raidConfig && raidConfig.gates && 
      raidConfig.gates.some(gate => gate.take_gold === true);
  }

  // Handle click on disabled checkbox (show message only)
  function handleDisabledGoldClick(char: any) {
    // No message needed - disabled checkboxes are self-explanatory
  }

  // Toggle master raid settings - Optimized for Svelte 5
  async function toggleMasterRaid(charId: number, raidId: string, difficulty: string, type: 'take_gold' | 'buy_box' | 'difficulty') {
    // Find character across all raid groups
    const characterRaids = raidMatrix
      .flatMap(raidGroup => raidGroup.characters)
      .filter(c => c.char_id === charId);
    
    const char = characterRaids[0];
    if (!char) return;

    // Find raid definition from raids.ts to get all gates
    const raidDef = RAIDS.find(r => r.id === raidId && r.difficulty === difficulty);
    if (!raidDef) return;

    // Determine target value (take status of first gate and negate it)
    const raidConfig = char.raid_configs.find(r => r.content_id === raidId);
    const targetValue = raidConfig && raidConfig.gates && raidConfig.gates.length > 0 ? !raidConfig.gates[0][type] : true;

    // GOLD LIMIT CHECK for Master-Toggle
    if (type === 'take_gold' && targetValue === true) {
      const currentGoldRaidsCount = char.raid_configs.filter(r => 
        r.gates && r.gates.some(gate => gate.take_gold === true)
      ).length;
      if (char.earns_gold && currentGoldRaidsCount >= 3) {
        successMessage = `Limit reached! ${char.char_name} can only get gold from 3 raids.`;
        showSuccessMessage = true;
        
        // Hide message after 3 seconds
        setTimeout(() => {
          successMessage = '';
          showSuccessMessage = false;
        }, 3000);
        
        // Don't proceed with the toggle - checkbox stays false
        return;
      }
    }

    // DIFFICULTY CHANGE - Update all gates to new difficulty
    if (type === 'difficulty') {
      // 1. Local update for ALL gates of this raid
      for (const gateDef of raidDef.gates) {
        let config = char.raid_configs.find(r => r.content_id === raidId);
        if (!config) {
          config = { 
            content_id: raidId, 
            gates: [], 
            take_gold: false, 
            buy_box: false 
          };
          char.raid_configs.push(config);
        }
        
        let gateConfig = config.gates.find((g: any) => g.gate === gateDef.gate);
        if (!gateConfig) {
          gateConfig = {
            gate: gateDef.gate,
            difficulty: difficulty,
            take_gold: false,
            buy_box: false
          };
          config.gates.push(gateConfig);
        } else {
          gateConfig.difficulty = difficulty;
        }
      }

      // Force Svelte 5 reactivity with deeper assignment
    raidMatrix = raidMatrix.map(raidGroup => ({
      ...raidGroup,
      characters: raidGroup.characters.map(char => ({
        ...char,
        raid_configs: char.raid_configs.map(config => ({
          ...config,
          gates: config.gates ? [...config.gates] : []
        }))
      }))
    }));

      // 2. Backend Updates
      try {
        for (const gateDef of raidDef.gates) {
          await invoke('update_raid_gate_config', {
            rosterId: $activeRosterId,
            charId,
            contentId: raidId,
            gate: gateDef.gate,
            difficulty: difficulty,
            takeGold: undefined,
            buyBox: undefined
          });
        }
      } catch (err) {
        console.error("Error saving difficulty:", err);
        error = `Failed to update difficulty: ${err}`;
      }
      return;
    }

    // GOLD/BOX TOGGLE - Update all gates with new gold/box value
    // 1. Local update for ALL gates of this raid
    for (const gateDef of raidDef.gates) {
      let config = char.raid_configs.find(r => r.content_id === raidId);
      if (!config) {
        config = { 
          content_id: raidId, 
          gates: [], 
          take_gold: false, 
          buy_box: false 
        };
        char.raid_configs.push(config);
      }
      
      let gateConfig = config.gates.find((g: any) => g.gate === gateDef.gate);
      if (!gateConfig) {
        gateConfig = {
          gate: gateDef.gate,
          difficulty: difficulty,
          take_gold: targetValue,
          buy_box: targetValue
        };
        config.gates.push(gateConfig);
      } else {
        gateConfig[type] = targetValue;
      }
    }

    // 2. Update raid config properties for master row display
    const config = char.raid_configs.find(r => r.content_id === raidId);
    if (config) {
      config[type] = targetValue;
      
      // 3. Recalculate gold values if take_gold changed
      if (type === 'take_gold') {
        const raidGroup = raidMatrix.find(r => r.content_id === raidId);
        if (raidGroup) {
          const getGoldValues = () => {
            const currentDifficulty = config.gates?.[0]?.difficulty || 'Solo';
            const currentRaidData = raidGroup.difficulties.get(currentDifficulty);
            if (!currentRaidData) return { totalGold: 0, tradableGold: 0, boundGold: 0, boxPrice: 0 };
            
            const totalGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.tradableGold || 0) + (gate.boundGold || 0), 0) || 0;
            const tradableGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.tradableGold || 0), 0) || 0;
            const boundGold = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.boundGold || 0), 0) || 0;
            const boxPrice = currentRaidData?.gates?.reduce((sum: number, gate: any) => sum + (gate.boxPrice || 0), 0) || 0;
            
            return { totalGold, tradableGold, boundGold, boxPrice };
          };
          
          // Update gold values for this character across all raid groups
          raidMatrix.forEach(raidGroup => {
            const charIndex = raidGroup.characters.findIndex(c => c.char_id === charId);
            if (charIndex !== -1) {
              raidGroup.characters[charIndex].gold_values = getGoldValues();
            }
          });
        }
      }
    }

    // Force Svelte 5 reactivity with deeper assignment
    raidMatrix = raidMatrix.map(raidGroup => ({
      ...raidGroup,
      characters: raidGroup.characters.map(char => ({
        ...char,
        raid_configs: char.raid_configs.map(config => ({
          ...config,
          gates: config.gates ? [...config.gates] : []
        }))
      }))
    }));

    // 2. Backend Updates (We send a request for each gate)
    try {
      for (const gateDef of raidDef.gates) {
        await invoke('update_raid_gate_config', {
          rosterId: $activeRosterId,
          charId,
          contentId: raidId,
          gate: gateDef.gate,
          difficulty: difficulty,
          takeGold: type === 'take_gold' ? targetValue : undefined,
          buyBox: type === 'buy_box' ? targetValue : undefined
        });
      }
    } catch (err) {
      console.error("Error saving master raid config:", err);
      error = `Failed to update master raid setting: ${err}`;
    }

    // Force master row checkbox reactivity
    forceMasterRowUpdate();
  }

  
  // Toggle individual gate settings - Optimized for Svelte 5
  async function toggleRaidGate(charId: number, raidId: string, difficulty: string, gate: string, type: 'take_gold' | 'buy_box') {
    // Find character across all raid groups
    const characterRaids = raidMatrix
      .flatMap(raidGroup => raidGroup.characters)
      .filter(c => c.char_id === charId);
    
    const char = characterRaids[0];
    if (!char) return;

    // 1. GOLD LIMIT CHECK
    if (type === 'take_gold') {
      // Count how many raids already have "take_gold" active for this character
      const currentGoldRaidsCount = char.raid_configs.filter(r => 
        r.gates && r.gates.some(gate => gate.take_gold === true)
      ).length;
      
      // Check if this specific gate is currently active
      const raidConfig = char.raid_configs.find(r => r.content_id === raidId);
      const currentGateConfig = raidConfig?.gates?.find(g => g.gate === gate);
      const isCurrentlyActive = currentGateConfig?.take_gold || false;

      // If gold character AND already has 3 raids AND we want to activate a NEW one
      if (char.earns_gold && currentGoldRaidsCount >= 3 && !isCurrentlyActive) {
        successMessage = `Limit reached: ${char.char_name} can only get gold from 3 raids.`;
        showSuccessMessage = true;
        
        // Hide message after 3 seconds
        setTimeout(() => {
          successMessage = '';
          showSuccessMessage = false;
        }, 3000);
        
        // Don't proceed with the toggle - checkbox stays false
        return;
      }
    }

    // 2. LOCAL UPDATE (Optimistic UI)
    let config = char.raid_configs.find(r => r.content_id === raidId);
    let gateConfig = config?.gates.find((g: any) => g.gate === gate);
    
    if (!config || !gateConfig) {
      // If no entry exists yet (initialization)
      if (!config) {
        config = { 
          content_id: raidId, 
          gates: [], 
          take_gold: false, 
          buy_box: false 
        };
        char.raid_configs.push(config);
      }
      
      gateConfig = {
        gate: gate,
        difficulty: difficulty,
        take_gold: false,
        buy_box: false
      };
      config.gates.push(gateConfig);
    }

    // Toggle value
    if (type === 'take_gold') gateConfig.take_gold = !gateConfig.take_gold;
    if (type === 'buy_box') gateConfig.buy_box = !gateConfig.buy_box;

    // Force Svelte 5 reactivity with deeper assignment
    raidMatrix = raidMatrix.map(raidGroup => ({
      ...raidGroup,
      characters: raidGroup.characters.map(char => ({
        ...char,
        raid_configs: char.raid_configs.map(config => ({
          ...config,
          gates: config.gates ? [...config.gates] : []
        }))
      }))
    }));

    // Force master row checkbox reactivity
    forceMasterRowUpdate();

    // 3. BACKEND CALL
    try {
      await invoke('update_raid_gate_config', {
        rosterId: $activeRosterId,
        charId,
        contentId: raidId,
        gate: gate,
        difficulty: gateConfig.difficulty,
        takeGold: gateConfig.take_gold,
        buyBox: gateConfig.buy_box
      });
    } catch (err) {
      console.error("Error saving raid config:", err);
      error = `Failed to update gate setting: ${err}`;
      // On error: reload page or rollback state
    }

    // Force master row checkbox reactivity
    forceMasterRowUpdate();
  }

  // Get total gold for a raid at specific difficulty
  function getTotalGold(raidGroup: RaidGroup, difficulty: string): number {
    const raid = raidGroup.difficulties.get(difficulty);
    if (!raid) return 0;
    
    return raid.gates.reduce((total, gate) => total + gate.tradableGold + gate.boundGold, 0);
  }

  // Get total box price for a raid at specific difficulty
  function getTotalBoxPrice(raidGroup: RaidGroup, difficulty: string): number {
    const raid = raidGroup.difficulties.get(difficulty);
    if (!raid) return 0;
    
    return raid.gates.reduce((total, gate) => total + gate.boxPrice, 0);
  }

  // Get class icon
  function getClassIcon(classId: string): string {
    return GAME_CLASSES[classId]?.iconId || '0';
  }

  // Load data when component mounts or roster changes
  $: if ($activeRosterId) {
    loadRaidConfiguration();
  }
</script>

<div class="raid-matrix-settings">
  <!-- Roster Selector -->
  <div class="roster-selector">
    <label for="roster-select" class="roster-label">Active Roster:</label>
    <select 
      id="roster-select"
      bind:value={$activeRosterId}
      class="roster-dropdown"
    >
      {#each $rosters as roster}
        <option value={roster.id}>{roster.roster_name}</option>
      {/each}
    </select>
  </div>

  {#if isLoading}
    <div class="loading">
      <div class="loading-spinner"></div>
      <p>Loading raid configuration...</p>
    </div>
  {:else if error}
    <div class="error">
      <p>{error}</p>
      <button on:click={loadRaidConfiguration}>Retry</button>
    </div>
  {:else if raidMatrix.length === 0}
    <div class="no-data">
      <p>No raid data available</p>
    </div>
  {:else}
    <!-- Success Message Overlay -->
    {#if showSuccessMessage}
      <div class="success-message {successMessage.includes('Cannot enable gold') ? 'warning' : 'success'}">
        <span>{successMessage}</span>
      </div>
    {/if}
    
    <div class="matrix-container">
      <div class="matrix-wrapper">
        <table class="raid-matrix">
          <thead>
            <tr class="header-row">
              <th class="sticky-col first-col">Raid \ Character</th>
              {#each raidMatrix[0]?.characters || [] as char}
                <th class="char-header sticky-col">
                  <div class="char-info">
                    <img src={`/images/classes/${getClassIcon(char.class_id)}.png`} alt="" class="class-icon" />
                    <div class="char-name-section">
                      <span class="char-name">{char.char_name}</span>
                      {#if char.earns_gold}
                        <img src="/images/gold.png" alt="Gold Earner" class="gold-earner-icon" />
                      {/if}
                    </div>
                    <div class="char-stats">
                      <span class="char-ilvl">iLvl: {Math.floor(char.item_level)}</span>
                      <span class="char-cp">CP: {Math.floor(char.combat_power)}</span>
                    </div>
                  </div>
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each raidMatrix as raid (raid.unique_key || raid.content_id)}
              <!-- Master Row -->
              <tr class="master-row">
                <td class="raid-name-cell sticky-col first-col">
                  <div class="raid-master-info">
                    <button 
                      class="expand-button"
                      class:expanded={raid.is_expanded}
                      on:click={() => toggleRaidExpansion(raid.content_id)}
                    >
                      <span class="expand-icon">{raid.is_expanded ? '▼' : '▶'}</span>
                    </button>
                    <span class="raid-name">{raid.raid_name}</span>
                  </div>
                </td>
                
                {#each raid.characters as char}
                  <td class="difficulty-cell">
                    {#if char.is_locked}
                      <div class="locked-indicator">
                        <span class="lock-icon">🔒</span>
                        <span class="lock-text">iLvl too low</span>
                      </div>
                    {:else}
                      <div class="cell-content">
                        <!-- Difficulty Toggles (Top) -->
                        <div class="difficulty-selector">
                          {#each char.available_difficulties as difficulty}
                            {@const isActive = char.master_difficulty === difficulty}
                            <button 
                              class="difficulty-btn"
                              class:active={isActive}
                              class:solo={difficulty === 'Solo'}
                              class:normal={difficulty === 'Normal'}
                              class:hard={difficulty === 'Hard'}
                              on:click={() => changeMasterDifficulty(raid.content_id, char.char_id, difficulty)}
                            >
                              {difficulty}
                            </button>
                          {/each}
                          
                          {#if char.master_difficulty === 'Mixed'}
                            <div class="mixed-indicator">Mixed</div>
                          {/if}
                        </div>
                        
                        <!-- Gold/Box Options (Bottom) -->
                        <div class="options-row">
                          <!-- Take Gold (only for gold earners) -->
                          {#if char.earns_gold}
                            {@const raidGoldValues = getRaidGoldValues(char, raid.content_id)}
                            {@const masterTakeGold = isMasterActiveReactive(char, raid.content_id, char.master_difficulty, 'take_gold')}
                            {@const masterBuyBox = isMasterActiveReactive(char, raid.content_id, char.master_difficulty, 'buy_box')}
                            {@const potentialBoxPrice = getPotentialBoxPrice(char, raid.content_id)}
                            {@const potentialGoldAmount = getPotentialGoldAmount(char, raid.content_id)}
                            {@const hasGoldLimit = hasReachedGoldLimit(char)}
                            {@const isRaidActive = isRaidAlreadyActive(char, raid.content_id)}
                            {@const shouldDisableGold = hasGoldLimit && !isRaidActive}
                            
                            <label class="option-toggle gold-option" title={`Tradable: ${raidGoldValues.tradableGold}g | Bound: ${raidGoldValues.boundGold}g (Total: ${raidGoldValues.totalGold}g)`}>
                              <input 
                                type="checkbox" 
                                checked={masterTakeGold}
                                disabled={shouldDisableGold}
                                on:change={(e) => {
                                  if (shouldDisableGold) {
                                    // Show message for disabled checkbox
                                    handleDisabledGoldClick(char);
                                    return;
                                  }
                                  
                                  // Normal toggle
                                  toggleMasterRaid(char.char_id, raid.content_id, char.master_difficulty, 'take_gold');
                                }}
                              />
                              <span class="option-label">Take Gold ({potentialGoldAmount}g)</span>
                            </label>
                            <label class="option-toggle">
                              <input 
                                type="checkbox" 
                                checked={masterBuyBox}
                                on:click={() => toggleMasterRaid(char.char_id, raid.content_id, char.master_difficulty, 'buy_box')}
                              />
                              <span class="option-label">Buy Box ({potentialBoxPrice}g)</span>
                            </label>
                          {/if}
                          
                          <!-- Buy Box Option for non-gold earners -->
                          {#if !char.earns_gold}
                            {@const nonGoldMasterBuyBox = isMasterActiveReactive(char, raid.content_id, char.master_difficulty, 'buy_box')}
                            {@const nonGoldPotentialBoxPrice = getPotentialBoxPrice(char, raid.content_id)}
                            <label class="option-toggle">
                              <input 
                                type="checkbox" 
                                checked={nonGoldMasterBuyBox}
                                on:click={() => toggleMasterRaid(char.char_id, raid.content_id, char.master_difficulty, 'buy_box')}
                              />
                              <span class="option-label">Buy Box ({nonGoldPotentialBoxPrice}g)</span>
                            </label>
                          {/if}
                        </div>
                      </div>
                    {/if}
                  </td>
                {/each}
              </tr>
              
              <!-- Child Rows (Gate Details) -->
              {#if raid.is_expanded}
                {#each Array.from(raid.gates.keys()).sort((a, b) => {
                  const gateA = raid.gates.get(a);
                  const gateB = raid.gates.get(b);
                  if (!gateA || !gateB) return 0;
                  if (gateA && !gateB) return -1;
                  if (!gateA && gateB) return 1;
                  return 0;
                }) as gateName}
                  <tr class="gate-row">
                    <td class="gate-name-cell sticky-col first-col">
                      <div class="gate-info">
                        <span class="gate-name">{gateName}</span>
                      </div>
                    </td>
                    
                    {#each raid.characters as char}
                      <td class="gate-controls-cell">
                        {#if !char.is_locked}
                          {@const currentGateBuyBox = char.raid_configs.find(r => r.content_id === raid.content_id)?.gates.find(g => g.gate === gateName)?.buy_box || false}
                          {@const gateData = getGateData(raid.content_id, char.raid_configs.find(r => r.content_id === raid.content_id)?.gates.find(g => g.gate === gateName)?.difficulty || 'Solo', gateName)}
                          {@const gateBoxPrice = gateData?.boxPrice || 0}
                          {@const gateTradableGold = gateData?.tradableGold || 0}
                          {@const gateBoundGold = gateData?.boundGold || 0}
                          {@const gateTotalGold = gateTradableGold + gateBoundGold}
                          <div class="cell-content">
                            <!-- Difficulty Toggles (Top) -->
                            <div class="difficulty-selector">
                              {#each char.available_difficulties as difficulty}
                                {@const currentGateDifficulty = char.raid_configs.find(r => r.content_id === raid.content_id)?.gates.find(g => g.gate === gateName)?.difficulty || ''}
                                <button 
                                  class="difficulty-btn"
                                  class:active={difficulty === currentGateDifficulty}
                                  class:solo={difficulty === 'Solo'}
                                  class:normal={difficulty === 'Normal'}
                                  class:hard={difficulty === 'Hard'}
                                  on:click={() => changeGateDifficulty(raid.content_id, char.char_id, gateName, difficulty)}
                                >
                                  {difficulty}
                                </button>
                              {/each}
                            </div>
                            
                            <!-- Buy Box Option (Bottom) -->
                            <div class="options-row">
                              <label class="option-toggle">
                                <input 
                                  type="checkbox" 
                                  checked={currentGateBuyBox}
                                  on:click={() => toggleRaidGate(char.char_id, raid.content_id, char.raid_configs.find(r => r.content_id === raid.content_id)?.gates.find(g => g.gate === gateName)?.difficulty || 'Solo', gateName, 'buy_box')}
                                />
                                <span class="option-label">Buy Box ({gateBoxPrice}g)</span>
                              </label>
                            </div>
                          </div>
                        {:else}
                          <!-- Locked character - no content -->
                        {/if}
                      </td>
                    {/each}
                  </tr>
                {/each}
              {/if}
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</div>

  <style>
  .raid-matrix-settings {
    display: flex;
    flex-direction: column;
    padding: 0;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .settings-header {
    padding: 16px;
    background: var(--md-sys-color-surface);
    border-bottom: 1px solid var(--md-sys-color-outline);
  }

  .settings-header h2 {
    margin: 0 0 8px 0;
    color: var(--md-sys-color-on-surface);
    font-size: 20px;
    font-weight: 600;
  }

  .settings-description {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 14px;
    line-height: 1.4;
  }

  .loading, .error, .no-data {
    text-align: center;
    padding: 40px;
    color: var(--md-sys-color-on-surface);
  }

  .error {
    color: var(--md-sys-color-error);
  }

  .error button {
    margin-top: 10px;
    padding: 8px 16px;
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--md-sys-color-surface-variant);
    border-top: 3px solid var(--md-sys-color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto 16px;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .matrix-container {
    display: flex;
    flex: 1 1 auto;
    min-height: 0;
    background: var(--md-sys-color-surface);
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid var(--md-sys-color-outline);
    height: 100%;
    max-height: none;
  }

  .matrix-wrapper {
    min-height: 0;
    overflow-x: auto;
    overflow-y: auto;
    height: 100%;
    position: relative;
  }

  .raid-matrix {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
    min-width: 800px;
  }

  .header-row th {
    background: var(--md-sys-color-surface-variant);
    padding: 12px 8px;
    text-align: center;
    border-bottom: 2px solid var(--md-sys-color-outline);
    font-weight: 600;
    color: var(--md-sys-color-on-surface-variant);
  }

  .char-header {
    min-width: 150px;
    border-left: 1px solid var(--md-sys-color-outline);
  }

  .char-info {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .class-icon {
    width: 32px;
    height: 32px;
    border-radius: 50%;
  }

  .char-name {
    font-weight: 600;
    font-size: 12px;
    color: var(--md-sys-color-on-surface);
  }

  .char-name-section {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .gold-earner-icon {
    width: 14px;
    height: 14px;
    object-fit: contain;
  }

  .char-stats {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 10px;
    color: var(--md-sys-color-on-surface-variant);
  }

  .char-cp {
    font-weight: 500;
  }

  .char-ilvl {
    color: var(--md-sys-color-tertiary);
  }

  .gold-earner-badge {
    background: var(--md-sys-color-tertiary);
    color: var(--md-sys-color-on-tertiary);
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 9px;
  }

  .sticky-col {
    position: sticky;
    left: 0;
    z-index: 10;
    background: var(--md-sys-color-surface);
    box-shadow: 2px 0 0 0 var(--md-sys-color-outline);
  }

  .first-col {
    z-index: 20;
    background: var(--md-sys-color-surface-variant);
    box-shadow: 2px 0 0 0 var(--md-sys-color-outline);
  }

  .master-row {
    background: var(--md-sys-color-surface-container);
  }

  .raid-name-cell {
    padding: 12px 8px;
    border-bottom: 1px solid var(--md-sys-color-outline);
    font-weight: 500;
    min-width: 200px;
  }

  .raid-master-info {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .expand-button {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 12px;
    color: var(--md-sys-color-on-surface);
    padding: 4px;
    border-radius: 4px;
    transition: all 0.2s ease;
  }

  .expand-button:hover {
    background: var(--md-sys-color-surface-container-highest);
  }

  .raid-name {
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
  }

  .difficulty-cell {
    padding: 8px;
    text-align: center;
    border-bottom: 1px solid var(--md-sys-color-outline);
    border-left: 1px solid var(--md-sys-color-outline);
    min-width: 120px;
  }

  .locked-indicator {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    color: var(--md-sys-color-on-surface-variant);
  }

  .lock-icon {
    font-size: 16px;
  }

  .lock-text {
    font-size: 11px;
    font-style: italic;
  }

  .difficulty-selector {
    display: flex;
    gap: 4px;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
  }

  .difficulty-btn {
    padding: 4px 8px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 4px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  .difficulty-btn:hover {
    background: var(--md-sys-color-surface-container-highest);
    transform: translateY(-1px);
  }

  .difficulty-btn.active {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border-color: #ff6b35;
    box-shadow: 0 0 8px #ff6b35,
                0 0 16px #ff6b35,
                inset 0 0 4px #ff6b35;
    transform: translateY(-1px);
  }

  .difficulty-btn.solo {
    background: var(--md-sys-color-secondary);
    color: var(--md-sys-color-on-secondary);
    border-color: var(--md-sys-color-secondary);
  }

  .difficulty-btn.solo.active {
    background: var(--md-sys-color-secondary);
    color: var(--md-sys-color-on-secondary);
    border-color: #ff6b35;
    box-shadow: 0 0 8px #ff6b35,
                0 0 16px #ff6b35,
                inset 0 0 4px #ff6b35;
  }

  .difficulty-btn.normal {
    background: var(--md-sys-color-secondary);
    color: var(--md-sys-color-on-secondary);
    border-color: var(--md-sys-color-secondary);
  }

  .difficulty-btn.normal.active {
    background: var(--md-sys-color-secondary);
    color: var(--md-sys-color-on-secondary);
    border-color: #ff6b35;
    box-shadow: 0 0 8px #ff6b35,
                0 0 16px #ff6b35,
                inset 0 0 4px #ff6b35;
  }

  .difficulty-btn.hard {
    background: var(--md-sys-color-secondary);
    color: var(--md-sys-color-on-secondary);
    border-color: var(--md-sys-color-secondary);
  }

  .difficulty-btn.hard.active {
    background: var(--md-sys-color-secondary);
    color: var(--md-sys-color-on-secondary);
    border-color: #ff6b35;
    box-shadow: 0 0 8px #ff6b35,
                0 0 16px #ff6b35,
                inset 0 0 4px #ff6b35;
  }

  .mixed-indicator {
    padding: 4px 8px;
    background: var(--md-sys-color-warning);
    color: var(--md-sys-color-on-warning);
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
  }

  .gate-row {
    background: var(--md-sys-color-surface);
  }

  .gate-name-cell {
    padding: 8px 8px 8px 32px;
    border-bottom: 1px solid var(--md-sys-color-outline);
    font-size: 12px;
    min-width: 200px;
  }

  .gate-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .gate-name {
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
  }

  .gate-rewards {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .reward-info {
    display: flex;
    gap: 8px;
    font-size: 10px;
    color: var(--md-sys-color-on-surface-variant);
  }

  .gold-amount {
    color: var(--md-sys-color-tertiary);
  }

  .box-price {
    color: var(--md-sys-color-secondary);
  }

  .gate-controls-cell {
    padding: 6px;
    text-align: center;
    border-bottom: 1px solid var(--md-sys-color-outline);
    border-left: 1px solid var(--md-sys-color-outline);
    min-width: 120px;
  }

  .cell-content {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: center;
  }

  .options-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: center;
  }

  .option-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--md-sys-color-on-surface);
    transition: all 0.2s ease;
  }

  .option-toggle input[type="checkbox"] {
    width: 14px;
    height: 14px;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .option-toggle input[type="checkbox"]:checked {
    background: var(--md-sys-color-primary);
    border-color: #ff6b35;
    box-shadow: 0 0 6px #ff6b35,
                0 0 12px #ff6b35;
  }

  .option-toggle input[type="checkbox"]:hover {
    transform: scale(1.1);
  }

  .option-toggle input[type="checkbox"]:checked:hover {
    transform: scale(1.15);
    box-shadow: 0 0 8px #ff6b35,
                0 0 16px #ff6b35;
  }

  /* Neon Pulse Animations */
  @keyframes rainbow-border {
    0% { background-position: 0% 50%; }
    50% { background-position: 100% 50%; }
    100% { background-position: 0% 50%; }
  }

  @keyframes neon-pulse {
    0%, 100% {
      box-shadow: 0 0 8px rgba(var(--md-sys-color-primary-rgb), 0.6),
                  0 0 16px rgba(var(--md-sys-color-primary-rgb), 0.3),
                  inset 0 0 4px rgba(var(--md-sys-color-primary-rgb), 0.2);
    }
    50% {
      box-shadow: 0 0 12px rgba(var(--md-sys-color-primary-rgb), 0.8),
                  0 0 24px rgba(var(--md-sys-color-primary-rgb), 0.4),
                  inset 0 0 6px rgba(var(--md-sys-color-primary-rgb), 0.3);
    }
  }

  @keyframes neon-pulse-tertiary {
    0%, 100% {
      box-shadow: 0 0 8px rgba(var(--md-sys-color-tertiary-rgb), 0.6),
                  0 0 16px rgba(var(--md-sys-color-tertiary-rgb), 0.3),
                  inset 0 0 4px rgba(var(--md-sys-color-tertiary-rgb), 0.2);
    }
    50% {
      box-shadow: 0 0 12px rgba(var(--md-sys-color-tertiary-rgb), 0.8),
                  0 0 24px rgba(var(--md-sys-color-tertiary-rgb), 0.4),
                  inset 0 0 6px rgba(var(--md-sys-color-tertiary-rgb), 0.3);
    }
  }

  @keyframes neon-pulse-secondary {
    0%, 100% {
      box-shadow: 0 0 8px rgba(var(--md-sys-color-secondary-rgb), 0.6),
                  0 0 16px rgba(var(--md-sys-color-secondary-rgb), 0.3),
                  inset 0 0 4px rgba(var(--md-sys-color-secondary-rgb), 0.2);
    }
    50% {
      box-shadow: 0 0 12px rgba(var(--md-sys-color-secondary-rgb), 0.8),
                  0 0 24px rgba(var(--md-sys-color-secondary-rgb), 0.4),
                  inset 0 0 6px rgba(var(--md-sys-color-secondary-rgb), 0.3);
    }
  }

  @keyframes neon-pulse-error {
    0%, 100% {
      box-shadow: 0 0 8px rgba(var(--md-sys-color-error-rgb), 0.6),
                  0 0 16px rgba(var(--md-sys-color-error-rgb), 0.3),
                  inset 0 0 4px rgba(var(--md-sys-color-error-rgb), 0.2);
    }
    50% {
      box-shadow: 0 0 12px rgba(var(--md-sys-color-error-rgb), 0.8),
                  0 0 24px rgba(var(--md-sys-color-error-rgb), 0.4),
                  inset 0 0 6px rgba(var(--md-sys-color-error-rgb), 0.3);
    }
  }

  @keyframes checkbox-pulse {
    0%, 100% {
      box-shadow: 0 0 6px rgba(var(--md-sys-color-primary-rgb), 0.6),
                  0 0 12px rgba(var(--md-sys-color-primary-rgb), 0.3);
    }
    50% {
      box-shadow: 0 0 10px rgba(var(--md-sys-color-primary-rgb), 0.8),
                  0 0 20px rgba(var(--md-sys-color-primary-rgb), 0.4);
    }
  }

  .gate-controls {
    display: flex;
    gap: 8px;
    justify-content: center;
    align-items: center;
  }

  .gate-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    font-size: 12px;
  }

  .gate-toggle input[type="checkbox"] {
    width: 14px;
    height: 14px;
    accent-color: var(--md-sys-color-primary);
  }

  .toggle-label {
    color: var(--md-sys-color-on-surface);
  }

  /* Scrollbar styling */
  .matrix-wrapper::-webkit-scrollbar {
    width: 8px;
    height: 8px;
  }

  .matrix-wrapper::-webkit-scrollbar-track {
    background: var(--md-sys-color-surface-variant);
  }

  .matrix-wrapper::-webkit-scrollbar-thumb {
    background: var(--md-sys-color-on-surface-variant);
    border-radius: 4px;
  }

  .matrix-wrapper::-webkit-scrollbar-thumb:hover {
    background: var(--md-sys-color-on-surface);
  }

  @media (max-width: 768px) {
    .raid-matrix-settings {
      padding: 10px;
    }
    
    .char-header {
      min-width: 120px;
    }
    
    .raid-name-cell {
      min-width: 150px;
    }
    
    .difficulty-cell {
      min-width: 100px;
    }
  }

  /* Success Messages */
  .success-message {
    position: absolute;
    top: 10px;
    right: 20px;
    padding: 1rem 1.5rem;
    border-radius: 8px;
    font-weight: 600;
    z-index: 100;
    animation: slideIn 0.3s ease-out;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    max-width: 400px;
  }

  .success-message.success {
    background: linear-gradient(135deg, #4caf50 0%, #45a049 100%);
    color: white;
    border: 1px solid #4caf50;
  }

  .success-message.error {
    background: linear-gradient(135deg, #f44336 0%, #d32f2f 100%);
    color: white;
    border: 1px solid #f44336;
  }

  .success-message.warning {
    background: linear-gradient(135deg, #ff9800 0%, #ff6b35 100%);
    color: white;
    border: 1px solid #ff9800;
  }

  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  /* Roster Selector Styles - matching TrackingSettings */
  .roster-selector {
    padding: 16px;
    background: var(--md-sys-color-surface);
    border-bottom: 1px solid var(--md-sys-color-outline);
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .roster-label {
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
    font-size: 14px;
  }

  .roster-dropdown {
    padding: 8px 12px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font-size: 14px;
    min-width: 200px;
    cursor: pointer;
  }

  .roster-dropdown:focus {
    outline: 2px solid var(--md-sys-color-primary);
    outline-offset: 2px;
  }
</style>
