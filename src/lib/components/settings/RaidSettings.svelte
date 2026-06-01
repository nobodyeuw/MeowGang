<script lang="ts">
  // Raid Settings owns raid config persistence and optimistic state updates.
  // The matrix components render the visible rows, including mixed-gate difficulty state.
  import { onMount } from 'svelte';
  import { activeRosterId } from '$lib/store';
  import RosterButtonGroup from '$lib/components/common/RosterButtonGroup.svelte';
  import { markMeowConnectUnsyncedChanges } from '$lib/services/meow-connect';
  import RaidMatrixTable from '$lib/components/settings/raid-settings/RaidMatrixTable.svelte';
  import {
    buildRaidMatrixData,
    cloneRaidConfigs,
    getBulkToggleCharacters,
    getClassIcon,
    getMasterDifficulty,
    getRaidGoldValues,
    getRaidDefinition,
    getTotalBoxPrice,
    getTotalGold,
    hasMixedDifficulties,
    hasReachedGoldLimit,
    isMasterActive,
    isRaidAlreadyActive,
    isRaidRowEnabled,
    loadCollapseUntrackedRaidRows,
    saveCollapseUntrackedRaidRows
  } from '$lib/components/settings/raid-settings/helpers';
  import type {
    CharacterRaidConfig,
    RaidBulkToggleType,
    RaidConfig,
    RaidMatrixData
  } from '$lib/components/settings/raid-settings/types';
  import { loadRaidSettingsMatrix, updateRaidGateConfigCommand } from '$lib/services/raid-settings';

  // State
  let raidMatrix: RaidMatrixData[] = [];
  let isLoading = true;
  let error = '';
  let successMessage = '';
  let showSuccessMessage = false;
  let expandedRaids = new Set<string>();
  let lastLoadedRosterId: string = '';
  let collapseUntrackedRaidRows = loadCollapseUntrackedRaidRows();
  
  // Reactive checkbox states to force UI updates
  let checkboxUpdateTrigger = 0;

  // Additional reactive variables for the new structure
  $: matrixData = {
    character_states: raidMatrix.length > 0 ? raidMatrix[0].characters : []
  };
  $: visibleRaidMatrix = collapseUntrackedRaidRows ? raidMatrix.filter(isRaidRowEnabled) : raidMatrix;
  $: hasHiddenRaidRows = raidMatrix.some((raid) => !isRaidRowEnabled(raid));
  
  function setCollapseUntrackedRaidRows(value: boolean) {
    collapseUntrackedRaidRows = value;
    saveCollapseUntrackedRaidRows(value);
  }

  async function loadRaidConfiguration() {
    try {
      error = '';
      raidMatrix = [];
      isLoading = true;
      
      if (!$activeRosterId) {
        throw new Error('No active roster selected');
      }
      
      // Tracking state is loaded with raid config so untracked raids can be collapsed without extra UI calls.
      const { characters, raid_configs, trackingStates } = await loadRaidSettingsMatrix($activeRosterId);
      
      // Check if raid_configs is defined
      if (!raid_configs || !Array.isArray(raid_configs)) {
        console.error('ERROR: raid_configs is not an array:', raid_configs);
        throw new Error('Invalid raid_configs data from backend');
      }
      
      raidMatrix = buildRaidMatrixData(characters, raid_configs, trackingStates, expandedRaids);
      
    } catch (err) {
      error = `Failed to load raid configuration: ${err}`;
      console.error(error);
    } finally {
      isLoading = false;
    }
  }

  function hasMixedDifficultiesReactive(char: CharacterRaidConfig, raidId: string): boolean {
    checkboxUpdateTrigger;
    const latestChar = getRaidRowCharacter(raidId, char.char_id) ?? char;
    return hasMixedDifficulties(latestChar, raidId);
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
    const targetChar = raidMatrix
      .flatMap(rg => rg.characters)
      .find(c => c.char_id === charId);
    if (!targetChar) return;

    const targetGate = targetChar.raid_configs
      .find(r => r.content_id === contentId)?.gates
      .find(g => g.gate === gateName);
    if (!targetGate) return;

    const oldDifficulty = targetGate.difficulty;

    // Immutable update - only touch the specific raid row and gate
    raidMatrix = raidMatrix.map(rg => {
      if (rg.content_id !== contentId) return rg;
      return {
        ...rg,
        characters: rg.characters.map(c => {
          if (c.char_id !== charId) return c;
          const updatedRaidConfigs = c.raid_configs.map(cfg => {
            if (cfg.content_id !== contentId) return cfg;
            return {
              ...cfg,
              gates: cfg.gates
                ? cfg.gates.map(g =>
                    g.gate === gateName ? { ...g, difficulty: newDifficulty } : g
                  )
                : []
            };
          });
          return {
            ...c,
            raid_configs: updatedRaidConfigs,
            master_difficulty: getMasterDifficulty(rg, updatedRaidConfigs)
          };
        })
      };
    });
    checkboxUpdateTrigger++;

    try {
      await updateRaidGateConfigCommand({
        rosterId: $activeRosterId,
        charId,
        contentId,
        gate: gateName,
        difficulty: newDifficulty,
        takeGold: targetGate.take_gold,
        buyBox: targetGate.buy_box
      });

      // Solo cascades to all gates
      if (newDifficulty === 'Solo' || (oldDifficulty === 'Solo' && newDifficulty !== 'Solo')) {
        await changeMasterDifficulty(contentId, charId, newDifficulty);
      } else {
        updateMasterDifficulty(charId, contentId);
      }
      markMeowConnectUnsyncedChanges('Raid difficulty setting changed.');
    } catch (err) {
      console.error('Failed to update gate difficulty:', err);
      // Revert
      raidMatrix = raidMatrix.map(rg => ({
        ...rg,
        characters: rg.characters.map(c => {
          if (c.char_id !== charId) return c;
          const updatedRaidConfigs = c.raid_configs.map(cfg => {
            if (cfg.content_id !== contentId) return cfg;
            return {
              ...cfg,
              gates: cfg.gates
                ? cfg.gates.map(g =>
                    g.gate === gateName ? { ...g, difficulty: oldDifficulty } : g
                  )
                : []
            };
          });
          return {
            ...c,
            raid_configs: updatedRaidConfigs,
            master_difficulty: getMasterDifficulty(rg, updatedRaidConfigs)
          };
        })
      }));
      checkboxUpdateTrigger++;
    }
  }

  // Change difficulty for ALL gates in a raid (master row)
  async function changeMasterDifficulty(contentId: string, charId: number, newDifficulty: string) {
    // Find the target character's raid config - read only, do NOT mutate
    const targetChar = raidMatrix
      .flatMap(rg => rg.characters)
      .find(c => c.char_id === charId);
    if (!targetChar) return;

    const targetRaidConfig = targetChar.raid_configs.find(r => r.content_id === contentId);
    if (!targetRaidConfig || !targetRaidConfig.gates) return;

    // Produce a fully immutable update for the specific raid row only.
    raidMatrix = raidMatrix.map(rg => {
      if (rg.content_id !== contentId) return rg;
      return {
        ...rg,
        characters: rg.characters.map(c => {
          if (c.char_id !== charId) return c;
          return {
            ...c,
            master_difficulty: newDifficulty,
            raid_configs: c.raid_configs.map(cfg => {
              if (cfg.content_id !== contentId) return cfg;
              return {
                ...cfg,
                gates: cfg.gates
                  ? cfg.gates.map(g => ({ ...g, difficulty: newDifficulty }))
                  : []
              };
            })
          };
        })
      };
    });

    // Re-read the gates from the now-updated (immutable) raidMatrix for backend calls
    const updatedChar = raidMatrix
      .flatMap(rg => rg.characters)
      .find(c => c.char_id === charId);
    const updatedGates = updatedChar?.raid_configs
      .find(r => r.content_id === contentId)?.gates ?? [];

    // Sequential saves - no Promise.all to avoid DELETE+re-insert race
    try {
      for (const gate of updatedGates) {
        await updateRaidGateConfigCommand({
          rosterId: $activeRosterId,
          charId,
          contentId,
          gate: gate.gate,
          difficulty: newDifficulty,
          takeGold: gate.take_gold,
          buyBox: gate.buy_box
        });
      }
      markMeowConnectUnsyncedChanges('Raid master difficulty changed.');
    } catch (err) {
      console.error('Failed to update master difficulty:', err);
      error = `Failed to update difficulty: ${err}`;
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

    raidMatrix = raidMatrix.map(raidGroup => ({
      ...raidGroup,
      characters: raidGroup.characters.map(char => ({
        ...char,
        master_difficulty: char.char_id === charId && raidGroup.content_id === contentId
          ? getMasterDifficulty(raidGroup, char.raid_configs)
          : char.master_difficulty,
        raid_configs: char.raid_configs.map(config => ({
          ...config,
          gates: config.gates ? [...config.gates] : []
        }))
      }))
    }));
    checkboxUpdateTrigger++;
  }

  // Toggle buy box for a specific gate
  async function toggleGateBox(contentId: string, charId: number, gateName: string, currentValue: boolean) {
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
      
      // Update gate buy_box
      gateConfig.buy_box = !currentValue;
      
      // Save updated configuration
      await updateRaidGateConfigCommand({
        rosterId: $activeRosterId,
        charId,
        contentId,
        gate: gateName,
        difficulty: gateConfig.difficulty || '',
        takeGold: gateConfig.take_gold,
        buyBox: gateConfig.buy_box
      });
      markMeowConnectUnsyncedChanges('Raid gate setting changed.');
      
      dispatchEvent(new CustomEvent('raid-settings-updated'));
      
      // Update only this character's gold values without full reload
      const raidGroupIndex = raidMatrix.findIndex(r => r.content_id === contentId);
      if (raidGroupIndex !== -1) {
        const charIndex = raidMatrix[raidGroupIndex].characters.findIndex(c => c.char_id === charId);
        if (charIndex !== -1) {
          // Recalculate gold values for this character
          const raidGroup = raidMatrix[raidGroupIndex];
          const character = raidMatrix[raidGroupIndex].characters[charIndex];
          
          character.gold_values = getRaidGoldValues(character, raidGroup.content_id);
          
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
      markMeowConnectUnsyncedChanges('Raid difficulty setting changed.');
      
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
          
          character.gold_values = getRaidGoldValues(character, raidGroup.content_id);
          
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
        await updateRaidGateConfigCommand({
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
    markMeowConnectUnsyncedChanges('Raid gate difficulty settings changed.');
  }

  // Reactive version of isMasterActive that triggers on checkboxUpdateTrigger
  function isMasterActiveReactive(char: CharacterRaidConfig, raidId: string, difficulty: string, type: 'take_gold' | 'buy_box' | 'reserved_for_static'): boolean {
    // This forces reactivity when checkboxUpdateTrigger changes
    checkboxUpdateTrigger;
    return isMasterActive(char, raidId, difficulty, type);
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

  function refreshCharacterAcrossRaidRows(charId: number, updatedRaidConfigs: RaidConfig[]) {
    raidMatrix = raidMatrix.map(raidGroup => ({
      ...raidGroup,
      characters: raidGroup.characters.map((matrixChar) => {
        if (matrixChar.char_id !== charId) {
          return {
            ...matrixChar,
            raid_configs: cloneRaidConfigs(matrixChar.raid_configs)
          };
        }

        const raidConfigs = cloneRaidConfigs(updatedRaidConfigs);
        const refreshedChar = {
          ...matrixChar,
          raid_configs: raidConfigs,
          master_difficulty: getMasterDifficulty(raidGroup, raidConfigs)
        };

        return {
          ...refreshedChar,
          gold_values: getRaidGoldValues(refreshedChar, raidGroup.content_id)
        };
      })
    }));

    checkboxUpdateTrigger++;
  }

  function areAllEligibleRaidMastersActive(raid: RaidMatrixData, type: RaidBulkToggleType): boolean {
    const eligibleCharacters = getBulkToggleCharacters(raid, type);
    if (eligibleCharacters.length === 0) return false;
    return eligibleCharacters.every((char) => isMasterActiveReactive(char, raid.content_id, char.master_difficulty, type));
  }

  function getRaidRowCharacter(contentId: string, charId: number): CharacterRaidConfig | undefined {
    return raidMatrix
      .find((raidGroup) => raidGroup.content_id === contentId)
      ?.characters.find((char) => char.char_id === charId);
  }

  async function toggleAllRaidMasters(raid: RaidMatrixData, type: RaidBulkToggleType) {
    const targetValue = !areAllEligibleRaidMastersActive(raid, type);
    const eligibleCharacters = getBulkToggleCharacters(raid, type, targetValue);
    let changedCount = 0;

    for (const char of eligibleCharacters) {
      const latestChar = getRaidRowCharacter(raid.content_id, char.char_id);
      if (!latestChar) continue;

      const currentValue = isMasterActiveReactive(latestChar, raid.content_id, latestChar.master_difficulty, type);
      if (currentValue === targetValue) continue;
      await toggleMasterRaid(latestChar.char_id, raid.content_id, latestChar.master_difficulty, type, targetValue);
      changedCount++;
    }

    const skippedGoldLimit = type === 'take_gold' && targetValue
      ? raid.characters.filter((char) => !char.is_locked && char.earns_gold && hasReachedGoldLimit(char) && !isRaidAlreadyActive(char, raid.content_id)).length
      : 0;

    successMessage = skippedGoldLimit > 0
      ? `Updated ${changedCount} characters. ${skippedGoldLimit} skipped due to the 3 raid gold limit.`
      : `${targetValue ? 'Enabled' : 'Disabled'} ${type === 'take_gold' ? 'Take Gold' : 'Static/Friends'} for ${changedCount} characters.`;
    showSuccessMessage = true;
    setTimeout(() => {
      successMessage = '';
      showSuccessMessage = false;
    }, 3000);
  }

  // Handle click on disabled checkbox (show message only)
  function handleDisabledGoldClick(char: any) {
    // No message needed - disabled checkboxes are self-explanatory
  }

  // Toggle master raid settings - Optimized for Svelte 5
  async function toggleMasterRaid(charId: number, raidId: string, difficulty: string, type: 'take_gold' | 'buy_box' | 'reserved_for_static' | 'difficulty', forcedTargetValue?: boolean) {
    // Find character across all raid groups
    const characterRaids = raidMatrix
      .flatMap(raidGroup => raidGroup.characters)
      .filter(c => c.char_id === charId);
    
    const char = characterRaids[0];
    if (!char) return;

    // Find raid definition from raids.ts to get all gates
    const raidDef = getRaidDefinition(raidId, difficulty);
    if (!raidDef) return;

    // Determine target value (take status of first gate and negate it)
    const raidConfig = char.raid_configs.find((r: any) => r.content_id === raidId);
    const targetValue = forcedTargetValue ?? (raidConfig && raidConfig.gates && raidConfig.gates.length > 0 ? !raidConfig.gates[0][type] : true);

    // GOLD LIMIT CHECK for Master-Toggle
    if (type === 'take_gold' && targetValue === true) {
      const currentGoldRaidsCount = char.raid_configs.filter((r: any) =>
        r.gates && r.gates.some((gate: any) => gate.take_gold === true)
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
            buy_box: false,
            reserved_for_static: false
          };
          char.raid_configs.push(config);
        }
        
        let gateConfig = config.gates.find((g: any) => g.gate === gateDef.gate);
        if (!gateConfig) {
          gateConfig = {
            gate: gateDef.gate,
            difficulty: difficulty,
            take_gold: false,
            buy_box: false,
            reserved_for_static: false
          };
          config.gates.push(gateConfig);
        } else {
          gateConfig.difficulty = difficulty;
        }
      }

      refreshCharacterAcrossRaidRows(charId, char.raid_configs);

      // 2. Backend Updates
      try {
        for (const gateDef of raidDef.gates) {
          await updateRaidGateConfigCommand({
            rosterId: $activeRosterId,
            charId,
            contentId: raidId,
            gate: gateDef.gate,
            difficulty: difficulty,
            takeGold: undefined,
            buyBox: undefined
          });
        }
        markMeowConnectUnsyncedChanges('Raid difficulty setting changed.');
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
          buy_box: false,
          reserved_for_static: false
        };
        char.raid_configs.push(config);
      }
      
      let gateConfig = config.gates.find((g: any) => g.gate === gateDef.gate);
      if (!gateConfig) {
        gateConfig = {
          gate: gateDef.gate,
          difficulty: difficulty,
          take_gold: type === 'take_gold' ? targetValue : false,
          buy_box: type === 'buy_box' ? targetValue : false,
          reserved_for_static: type === 'reserved_for_static' ? targetValue : false
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
          char.gold_values = getRaidGoldValues(char, raidGroup.content_id);
        }
      }
    }

    refreshCharacterAcrossRaidRows(charId, char.raid_configs);

    // 2. Backend Updates (We send a request for each gate)
    try {
      for (const gateDef of raidDef.gates) {
        const savedGateConfig = config?.gates?.find((gate: any) => gate.gate === gateDef.gate);
        await updateRaidGateConfigCommand({
          rosterId: $activeRosterId,
          charId,
          contentId: raidId,
          gate: gateDef.gate,
          difficulty: savedGateConfig?.difficulty || difficulty,
          takeGold: type === 'take_gold' ? targetValue : undefined,
          buyBox: type === 'buy_box' ? targetValue : undefined,
          reservedForStatic: type === 'reserved_for_static' ? targetValue : undefined
        });
      }
      markMeowConnectUnsyncedChanges(
        type === 'reserved_for_static'
          ? 'Raid static reservation setting changed.'
          : 'Raid gold or box setting changed.'
      );
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
          buy_box: false,
          reserved_for_static: false
        };
        char.raid_configs.push(config);
      }
      
      gateConfig = {
        gate: gate,
        difficulty: difficulty,
        take_gold: false,
        buy_box: false,
        reserved_for_static: false
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
      await updateRaidGateConfigCommand({
        rosterId: $activeRosterId,
        charId,
        contentId: raidId,
        gate: gate,
        difficulty: gateConfig.difficulty || difficulty,
        takeGold: gateConfig.take_gold,
        buyBox: gateConfig.buy_box
      });
      markMeowConnectUnsyncedChanges('Raid gate setting changed.');
    } catch (err) {
      console.error("Error saving raid config:", err);
      error = `Failed to update gate setting: ${err}`;
      // On error: reload page or rollback state
    }

    // Force master row checkbox reactivity
    forceMasterRowUpdate();
  }

  // Load data when component mounts or roster changes
  $: if ($activeRosterId && $activeRosterId !== lastLoadedRosterId) {
    lastLoadedRosterId = $activeRosterId;
    loadRaidConfiguration();
  }

  onMount(() => {
    const handleTrackingConfigChanged = (event: Event) => {
      const detail = (event as CustomEvent<{ type?: string }>).detail;
      if (detail?.type === 'raid' && $activeRosterId) {
        void loadRaidConfiguration();
      }
    };

    window.addEventListener('tracking-config-changed', handleTrackingConfigChanged);
    return () => {
      window.removeEventListener('tracking-config-changed', handleTrackingConfigChanged);
    };
  });
</script>

<div class="raid-matrix-settings">
  <RosterButtonGroup />

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
    
    <RaidMatrixTable
      {raidMatrix}
      {visibleRaidMatrix}
      {hasHiddenRaidRows}
      {collapseUntrackedRaidRows}
      onSetCollapseUntrackedRaidRows={setCollapseUntrackedRaidRows}
      onToggleRaidExpansion={toggleRaidExpansion}
      onToggleAllRaidMasters={toggleAllRaidMasters}
      {areAllEligibleRaidMastersActive}
      {getClassIcon}
      {hasMixedDifficultiesReactive}
      onChangeMasterDifficulty={changeMasterDifficulty}
      {isMasterActiveReactive}
      onToggleMasterRaid={toggleMasterRaid}
      onDisabledGoldClick={handleDisabledGoldClick}
      onChangeGateDifficulty={changeGateDifficulty}
      onToggleRaidGate={toggleRaidGate}
    />
  {/if}
</div>

  <style>
  .raid-matrix-settings {
    display: flex;
    flex-direction: column;
    padding: 0;
    flex: 1 1 0;
    min-height: 0;
    height: 100%;
    overflow: hidden;
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
    box-shadow: var(--app-shadow-md);
    max-width: 400px;
  }

  .success-message.success {
    background: var(--app-color-success-gradient);
    color: white;
    border: 1px solid var(--md-sys-color-success);
  }

  .success-message.warning {
    background: var(--app-color-warning-gradient);
    color: white;
    border: 1px solid var(--md-sys-color-warning);
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

</style>



