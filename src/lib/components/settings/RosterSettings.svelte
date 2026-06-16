<script lang="ts">
  import { rosters, characters, activeRosterId, loadCharacters, updateCharacter, loadRosters, scrapeRoster } from '$lib/store';
  import type { Character } from '$lib/store';
  import { onDestroy, onMount } from 'svelte';
  import { markMeowConnectUnsyncedChanges } from '$lib/services/meow-connect';
  import AddRosterDialog from '$lib/components/settings/roster-settings/AddRosterDialog.svelte';
  import DeleteRosterDialog from '$lib/components/settings/roster-settings/DeleteRosterDialog.svelte';
  import RenameRosterDialog from '$lib/components/settings/roster-settings/RenameRosterDialog.svelte';
  import RosterCharacterListSection from '$lib/components/settings/roster-settings/RosterCharacterListSection.svelte';
  import RosterManagementSection from '$lib/components/settings/roster-settings/RosterManagementSection.svelte';
  import SoftRemoveCharacterDialog from '$lib/components/settings/roster-settings/SoftRemoveCharacterDialog.svelte';
  import { setActiveRosterPreference } from '$lib/services/roster-preferences';
  import {
    getDailyUpdateBadge,
    type SyncMetadata
  } from '$lib/components/settings/roster-settings/helpers';
  import {
    deleteRosterCommand,
    loadAllCharactersCommand,
    loadRosterScrapeHistoryCommand,
    updateCharacterOrderCommand,
    updateRosterNameCommand,
    updateRosterOrderCommand
  } from '$lib/services/roster-settings';

  let showAddRosterDialog = false;
  let showRenameRosterDialog = false;
  let newRosterName = '';
  let renameRosterId = '';
  let renameRosterName = '';
  let isLoading = false;
  let successMessage = '';
  let showSuccessMessage = false;
  let dndItems: any[] = [];
  let isDragging = false;
  let rosterDndItems: any[] = [];
  let isRosterDragging = false;
  let scrapeStatusRosterId = '';
  let rosterScrapeHistory: SyncMetadata[] = [];
  let scrapeStatusLoading = false;
  let currentTime = Date.now();
  let countdownTimer: ReturnType<typeof setInterval> | null = null;
  let scrapeStatusRefreshTimer: ReturnType<typeof setInterval> | null = null;
  let deleteConfirmCharacter: Character | null = null;
  let deleteConfirmRemaining = 0;
  let deleteConfirmTimer: ReturnType<typeof setInterval> | null = null;
  let deleteRosterConfirm: { id: string; name: string } | null = null;

  // Load all characters for all rosters on component mount
  onMount(async () => {
    countdownTimer = setInterval(() => {
      currentTime = Date.now();
    }, 1000);
    scrapeStatusRefreshTimer = setInterval(() => {
      if ($activeRosterId) void loadRosterScrapeStatus($activeRosterId, false);
    }, 60000);
    await loadRosters();
    await loadAllCharacters();
  });

  onDestroy(() => {
    if (countdownTimer) clearInterval(countdownTimer);
    if (scrapeStatusRefreshTimer) clearInterval(scrapeStatusRefreshTimer);
    if (deleteConfirmTimer) clearInterval(deleteConfirmTimer);
  });

  async function loadAllCharacters() {
    try {
      const result = await loadAllCharactersCommand();
      
      const uniqueCharacters = Array.from(
        new Map((result || []).map(character => [character.char_id, character])).values()
      );

      // Update the characters store
      characters.set(uniqueCharacters);
    } catch (error) {
      console.error('Failed to load all characters:', error);
      console.error('Error details:', error);
    }
  }

  function openRenameDialog(rosterId: string, currentName: string) {
    renameRosterId = rosterId;
    renameRosterName = currentName || '';
    showRenameRosterDialog = true;
  }

  async function renameRoster() {
    if (isLoading || !renameRosterName.trim() || !renameRosterId) return;
    const targetRosterId = renameRosterId;
    const targetRosterName = renameRosterName.trim();
    
    try {
      isLoading = true;
      
      // Update every character row for this roster id in one backend call.
      await updateRosterNameCommand(targetRosterId, targetRosterName);
      
      // Update rosters store immediately for live UI updates
      rosters.update(current => {
        return current.map(roster => {
          if (roster.id === targetRosterId) {
            return { ...roster, roster_name: targetRosterName };
          }
          return roster;
        });
      });
      
      // Update characters store for consistency
      characters.update(current => {
        return current.map(char => {
          if (char.roster_id === targetRosterId) {
            return { ...char, roster_name: targetRosterName };
          }
          return char;
        });
      });
      
      // Show success message
      successMessage = `Roster "${targetRosterName}" renamed successfully!`;
      showSuccessMessage = true;
      
      // Hide success message after 3 seconds
      setTimeout(() => {
        showSuccessMessage = false;
        successMessage = '';
      }, 3000);
      
      // Close dialog
      showRenameRosterDialog = false;
      renameRosterId = '';
      renameRosterName = '';
    } catch (error) {
      console.error('Failed to rename roster:', error);
      successMessage = 'Failed to rename roster. Please try again.';
      showSuccessMessage = true;
      setTimeout(() => {
        showSuccessMessage = false;
        successMessage = '';
      }, 3000);
    } finally {
      isLoading = false;
    }
  }

  function cancelRename() {
    showRenameRosterDialog = false;
    renameRosterId = '';
    renameRosterName = '';
  }

  $: currentRoster = $rosters.find(r => r.id === $activeRosterId);
  $: if ($activeRosterId && $activeRosterId !== scrapeStatusRosterId) {
    scrapeStatusRosterId = $activeRosterId;
    void loadRosterScrapeStatus($activeRosterId);
  }
  $: dailyScrapeBadge = getDailyUpdateBadge(rosterScrapeHistory, currentTime, scrapeStatusLoading);

  $: {
    if (!isRosterDragging) {
      rosterDndItems = $rosters
        .map((roster, index) => ({
          ...roster,
          id: roster.id,
          roster_display_order: roster.roster_display_order ?? index
        }))
        .sort((a, b) =>
          (a.roster_display_order ?? 0) - (b.roster_display_order ?? 0)
          || a.roster_name.localeCompare(b.roster_name)
        );
    }
  }

  $: rosterDndOptions = {
    flipDurationMs: 200,
    dragHandleSelector: ".roster-drag-handle",
    items: rosterDndItems
  };

  $: {
    if (!isDragging) {
      const seenCharacterIds = new Set<number>();
      const rosterChars = $characters
        .filter(c => c.roster_id === $activeRosterId)
        .filter(c => {
          if (seenCharacterIds.has(c.char_id)) return false;
          seenCharacterIds.add(c.char_id);
          return true;
        })
        // Ensure we sort by stored display_order
        .sort((a, b) => (a.display_order || 0) - (b.display_order || 0));
      
      dndItems = rosterChars.map(c => ({...c, id: c.char_id.toString()}));
    }
  }

  $: dndOptions = {
    flipDurationMs: 200,
    dragHandleSelector: ".drag-handle",
    items: dndItems
  };


  // Auto-select first roster when rosters are loaded and no roster is active
  $: if ($rosters.length > 0 && !$activeRosterId) {
    activeRosterId.set($rosters[0].id);
    setActiveRosterPreference($rosters[0].id);
  }

  async function addRoster() {
    if (isLoading) return;
    if (newRosterName.trim()) {
      isLoading = true;
      const rosterName = newRosterName.trim();

      try {
        // Call scrapeRoster frontend function (includes complete initialization)
        await scrapeRoster(rosterName);

        // Reload rosters to resolve the newly created roster id
        const allRosters = await loadRosters();
        const newRoster = allRosters.find(roster => roster.roster_name === rosterName)
          || allRosters.find(roster => roster.roster_name.toLowerCase() === rosterName.toLowerCase());

        await loadAllCharacters();

        if (newRoster) {
          activeRosterId.set(newRoster.id);
          setActiveRosterPreference(newRoster.id);
          await loadRosterScrapeStatus(newRoster.id);
        } else {
          console.warn('Could not resolve new roster id after scrape; falling back to roster name', rosterName);
          activeRosterId.set(rosterName);
          setActiveRosterPreference(rosterName);
          await loadRosterScrapeStatus(rosterName);
        }

        newRosterName = '';
        showAddRosterDialog = false;
      } catch (error) {
        console.error('Failed to scrape roster:', error);
        alert('Failed to add roster: ' + error);
      } finally {
        isLoading = false;
      }
    }
  }

  function requestRemoveRoster(rosterId: string) {
    const roster = $rosters.find((entry) => entry.id === rosterId);
    deleteRosterConfirm = {
      id: rosterId,
      name: roster?.roster_name || rosterId
    };
  }

  function cancelRemoveRoster() {
    if (isLoading) return;
    deleteRosterConfirm = null;
  }

  async function confirmRemoveRoster() {
    if (!deleteRosterConfirm || isLoading) return;
    const rosterId = deleteRosterConfirm.id;

    try {
      isLoading = true;
      
      // Call backend to delete roster and all related data
      await deleteRosterCommand(rosterId);
      
      // Use store functions for consistent updates
      const { removeRoster: storeRemoveRoster } = await import('$lib/store');
      storeRemoveRoster(rosterId);
      
      // Show success message
      successMessage = 'Roster and all related data deleted successfully!';
      showSuccessMessage = true;
      
      // Hide success message after 3 seconds
      setTimeout(() => {
        showSuccessMessage = false;
      }, 3000);
      
    } catch (error) {
      console.error('Failed to delete roster:', error);
      alert('Failed to delete roster: ' + error);
    } finally {
      isLoading = false;
      deleteRosterConfirm = null;
    }
  }

  function handleRosterClick(rosterId: string) {
    // Only set the active roster ID, don't load characters (we already have all characters)
    activeRosterId.set(rosterId);
    setActiveRosterPreference(rosterId);
    void loadRosterScrapeStatus(rosterId);
  }

  async function loadRosterScrapeStatus(rosterId: string, showLoading = true) {
    if (!rosterId) {
      rosterScrapeHistory = [];
      return;
    }

    if (showLoading) scrapeStatusLoading = true;
    try {
      const history = await loadRosterScrapeHistoryCommand(rosterId);
      if ($activeRosterId === rosterId) {
        rosterScrapeHistory = history;
      }
    } catch (error) {
      console.error('Failed to load roster scrape status:', error);
      if ($activeRosterId === rosterId) {
        rosterScrapeHistory = [];
      }
    } finally {
      if (showLoading && $activeRosterId === rosterId) {
        scrapeStatusLoading = false;
      }
    }
  }

  function handleRosterDndConsider(event: CustomEvent<any>) {
    isRosterDragging = true;
    rosterDndItems = event.detail.items;
  }

  async function handleRosterDndFinalize(event: CustomEvent<any>) {
    const reorderedItems = event.detail.items;
    rosterDndItems = reorderedItems;

    rosters.update(current => {
      const orderByRosterId = new Map<string, number>(
        reorderedItems.map((item: any, index: number) => [String(item.id), index])
      );
      return [...current]
        .map(roster => ({
          ...roster,
          roster_display_order: orderByRosterId.get(roster.id) ?? roster.roster_display_order ?? 0
        }))
        .sort((a, b) => (a.roster_display_order ?? 0) - (b.roster_display_order ?? 0));
    });

    try {
      await updateRosterOrderCommand(
        reorderedItems.map((item: any, index: number) => ({
          roster_id: item.id,
          display_order: index
        }))
      );
    } catch (error) {
      console.error('Failed to update roster order:', error);
      await loadRosters();
    } finally {
      isRosterDragging = false;
    }
  }

  function handleDndConsider(event: CustomEvent<any>) {
    isDragging = true; // Block store updates
    const { items } = event.detail;
    dndItems = items;
  }

  async function handleDndFinalize(event: CustomEvent<any>) {
    const { items: reorderedItems } = event.detail;
    dndItems = reorderedItems;
    
    // Update store and database
    characters.update(current => {
      const updatedCharacters = [...current];
      reorderedItems.forEach((newChar: any, index: number) => {
        const charToUpdate = updatedCharacters.find(c => c.char_id === parseInt(newChar.id));
        if (charToUpdate) {
          charToUpdate.display_order = index;
        }
      });
      return updatedCharacters;
    });

    await updateCharacterDisplayOrder(reorderedItems);
    
    // Release lock so store can sync again if needed
    isDragging = false; 
  }

  async function updateCharacterDisplayOrder(items: any[]) {
    try {
      // Create simple character order updates with only required fields
      const characterUpdates = items.map((item, index) => ({
        char_id: parseInt(item.id),
        display_order: index
      }));
      
      await updateCharacterOrderCommand(characterUpdates);
      markMeowConnectUnsyncedChanges('Character display order changed.');
    } catch (error) {
      console.error('Failed to update character order:', error);
    }
  }

  async function toggleGold(char: any) {
    const newStatus = !char.earns_gold;
    
    // Check gold character limit for this roster
    const currentRosterCharacters = $characters.filter(c => c.roster_id === char.roster_id);
    const goldCharacterCount = currentRosterCharacters.filter(c => c.earns_gold).length;
    
    if (newStatus && goldCharacterCount >= 6) {
      // Show warning message
      successMessage = `Cannot enable gold for ${char.char_name}. Maximum 6 gold characters per roster allowed.`;
      showSuccessMessage = true;
      
      // Hide error message after 3 seconds
      setTimeout(() => {
        successMessage = '';
        showSuccessMessage = false;
      }, 3000);
      return;
    }
    
    try {
      // Use the new updateCharacter function from store
      await updateCharacter(char.char_id, { earns_gold: newStatus });
      markMeowConnectUnsyncedChanges(`${char.char_name} gold earner setting changed.`);
    } catch (err) {
      console.error("FRONTEND: Toggle failed:", err);
    }
  }

  async function toggleHideFromDashboard(char: any) {
    const newStatus = !char.hide_from_dashboard;
    try {
      await updateCharacter(char.char_id, { hide_from_dashboard: newStatus });
      markMeowConnectUnsyncedChanges(`${char.char_name} dashboard visibility changed.`);
    } catch (err) {
      console.error("FRONTEND: Hide toggle failed:", err);
    }
  }

  // Temporarily disabled due to Supabase realtime message limits
  // async function toggleMeowConnect(char: any) {
  //   const newStatus = !char.meow_connect_enabled;
  //   try {
  //     await updateCharacter(char.char_id, { meow_connect_enabled: newStatus });
  //     markMeowConnectUnsyncedChanges(`${char.char_name} MeowConnect sharing changed.`);
  //   } catch (err) {
  //     console.error("FRONTEND: MeowConnect toggle failed:", err);
  //   }
  // }

  function requestSoftRemoveCharacter(char: Character) {
    deleteConfirmCharacter = char;
    deleteConfirmRemaining = 5;
    if (deleteConfirmTimer) clearInterval(deleteConfirmTimer);
    deleteConfirmTimer = setInterval(() => {
      deleteConfirmRemaining = Math.max(deleteConfirmRemaining - 1, 0);
      if (deleteConfirmRemaining === 0 && deleteConfirmTimer) {
        clearInterval(deleteConfirmTimer);
        deleteConfirmTimer = null;
      }
    }, 1000);
  }

  function cancelSoftRemoveCharacter() {
    deleteConfirmCharacter = null;
    deleteConfirmRemaining = 0;
    if (deleteConfirmTimer) {
      clearInterval(deleteConfirmTimer);
      deleteConfirmTimer = null;
    }
  }

  async function softRemoveCharacter() {
    if (!deleteConfirmCharacter || deleteConfirmRemaining > 0) return;
    const char = deleteConfirmCharacter;

    try {
      await updateCharacter(char.char_id, {
        removed_from_roster: true,
        hide_from_dashboard: true,
        meow_connect_enabled: false
      });
      markMeowConnectUnsyncedChanges(`${char.char_name} removed from roster view.`);
      successMessage = `${char.char_name} removed from roster view.`;
      showSuccessMessage = true;
      setTimeout(() => {
        successMessage = '';
        showSuccessMessage = false;
      }, 3000);
      cancelSoftRemoveCharacter();
    } catch (err) {
      console.error("FRONTEND: Remove character failed:", err);
      successMessage = `Failed to remove ${char.char_name}.`;
      showSuccessMessage = true;
    }
  }

</script>

<div class="roster-settings">
  <!-- Success Message -->
  {#if showSuccessMessage}
    <div class="success-message {successMessage.includes('Failed') ? 'error' : successMessage.includes('Cannot enable gold') ? 'warning' : 'success'}">
      <span>{successMessage}</span>
    </div>
  {/if}

  <RosterManagementSection
    rosters={$rosters}
    characters={$characters}
    activeRosterId={$activeRosterId}
    {rosterDndItems}
    {rosterDndOptions}
    {isLoading}
    onAddRoster={() => { showAddRosterDialog = true; }}
    onRosterClick={handleRosterClick}
    onRosterDndConsider={handleRosterDndConsider}
    onRosterDndFinalize={handleRosterDndFinalize}
    onRenameRoster={openRenameDialog}
    onRemoveRoster={requestRemoveRoster}
  />

  {#if $activeRosterId && currentRoster}
    <RosterCharacterListSection
      rosterName={currentRoster.roster_name}
      dailyUpdateBadge={dailyScrapeBadge}
      {dndItems}
      {dndOptions}
      onDndConsider={handleDndConsider}
      onDndFinalize={handleDndFinalize}
      onToggleGold={toggleGold}
      onToggleHideFromDashboard={toggleHideFromDashboard}
      showMeowConnect={false}
      onRequestSoftRemove={requestSoftRemoveCharacter}
    />
  {/if}
</div>

{#if deleteConfirmCharacter}
  <SoftRemoveCharacterDialog
    character={deleteConfirmCharacter}
    remaining={deleteConfirmRemaining}
    onCancel={cancelSoftRemoveCharacter}
    onConfirm={softRemoveCharacter}
  />
{/if}

{#if deleteRosterConfirm}
  <DeleteRosterDialog
    rosterName={deleteRosterConfirm.name}
    {isLoading}
    onCancel={cancelRemoveRoster}
    onConfirm={confirmRemoveRoster}
  />
{/if}

<!-- Rename Roster Dialog -->
{#if showRenameRosterDialog}
  <RenameRosterDialog
    bind:rosterName={renameRosterName}
    {isLoading}
    onCancel={cancelRename}
    onRename={renameRoster}
  />
{/if}

<!-- Add Roster Dialog -->
{#if showAddRosterDialog}
  <AddRosterDialog
    bind:rosterName={newRosterName}
    {isLoading}
    onClose={() => { showAddRosterDialog = false; }}
    onAdd={addRoster}
  />
{/if}

<style>
  .roster-settings {
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  /* Success Messages */
  .success-message {
    position: fixed;
    top: 80px;
    right: 20px;
    padding: 1rem 1.5rem;
    border-radius: 8px;
    font-weight: 600;
    z-index: 9999;
    animation: slideIn 0.3s ease-out;
    box-shadow: var(--app-shadow-md);
  }

  .success-message.success {
    background: var(--app-color-success-gradient);
    color: white;
    border: 1px solid var(--md-sys-color-success);
  }

  .success-message.error {
    background: var(--app-color-error-gradient);
    color: white;
    border: 1px solid var(--md-sys-color-error);
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
