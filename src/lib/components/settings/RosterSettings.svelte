<script lang="ts">
  import { rosters, characters, activeRosterId, gameClasses, loadCharacters, updateCharacter, loadRosters, scrapeRoster } from '$lib/store';
  import type { Character } from '$lib/store';
  import { invoke } from '@tauri-apps/api/core';
  import { dndzone } from 'svelte-dnd-action';
  import { onMount } from 'svelte';
  import { flip } from 'svelte/animate';
  import { GAME_CLASSES } from '$lib/data/classes';

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

  // Load all characters for all rosters on component mount
  onMount(async () => {
    console.log('RosterSettings component mounted');
    await loadRosters();
    await loadAllCharacters();
  });

  async function loadAllCharacters() {
    try {
      console.log('Loading all characters from backend...');
      console.log('Calling get_characters with no parameters to get all characters...');
      const result: Character[] = await invoke('get_characters', {});
      console.log('All characters loaded successfully:', result);
      console.log('Number of characters loaded:', result?.length || 0);
      
      // Update the characters store
      characters.set(result || []);
      console.log('Characters store updated with all characters');
      console.log('Characters store now has:', $characters.length, 'characters');
      
      // Debug: Log characters grouped by roster
      const grouped: { [key: string]: Character[] } = {};
      $characters.forEach(char => {
        if (!grouped[char.roster_id]) {
          grouped[char.roster_id] = [];
        }
        grouped[char.roster_id].push(char);
      });
      console.log('Characters grouped by roster:', grouped);
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
    if (!renameRosterName.trim() || !renameRosterId) return;
    
    try {
      isLoading = true;
      
      // Update roster name in all characters for this roster
      console.log('Updating roster name for all characters in roster:', renameRosterId);
      const rosterCharacters = $characters.filter(c => c.roster_id === renameRosterId);
      console.log('Found', rosterCharacters.length, 'characters to update');
      
      for (const character of rosterCharacters) {
        await invoke('update_character_roster_name', {
          characterId: character.char_id,
          newRosterName: renameRosterName.trim()
        });
      }
      
      // Update rosters store immediately for live UI updates
      rosters.update(current => {
        return current.map(roster => {
          if (roster.id === renameRosterId) {
            return { ...roster, roster_name: renameRosterName.trim() };
          }
          return roster;
        });
      });
      
      // Update characters store for consistency
      characters.update(current => {
        return current.map(char => {
          if (char.roster_id === renameRosterId) {
            return { ...char, roster_name: renameRosterName.trim() };
          }
          return char;
        });
      });
      
      // Show success message
      successMessage = `Roster "${renameRosterName.trim()}" renamed successfully!`;
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
  $: {
    if (!isDragging) {
      const rosterChars = $characters
        .filter(c => c.roster_id === $activeRosterId)
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
    console.log('Auto-selecting first roster:', $rosters[0].id);
    activeRosterId.set($rosters[0].id);
    localStorage.setItem('activeRosterId', $rosters[0].id);
  }

  // Debug reactive statement to track changes
  $: console.log('Reactive update - rosters:', $rosters.length, 'activeRosterId:', $activeRosterId);

  async function addRoster() {
    if (newRosterName.trim()) {
      isLoading = true;
      const rosterName = newRosterName.trim();

      try {
        // Call scrapeRoster frontend function (includes complete initialization)
        const result = await scrapeRoster(rosterName);
        console.log('Roster scraped successfully:', result);

        // Reload rosters to resolve the newly created roster id
        const allRosters = await loadRosters();
        const newRoster = allRosters.find(roster => roster.roster_name === rosterName)
          || allRosters.find(roster => roster.roster_name.toLowerCase() === rosterName.toLowerCase());

        await loadAllCharacters();

        if (newRoster) {
          activeRosterId.set(newRoster.id);
          localStorage.setItem('activeRosterId', newRoster.id);
        } else {
          console.warn('Could not resolve new roster id after scrape; falling back to roster name', rosterName);
          activeRosterId.set(rosterName);
          localStorage.setItem('activeRosterId', rosterName);
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

  async function removeRoster(rosterId: string) {
    console.log('=== REMOVE ROSTER DEBUG ===');
    console.log('Roster ID to delete:', rosterId);
    
    // Create custom confirmation dialog
    const confirmed = await new Promise<boolean>((resolve) => {
      const dialog = document.createElement('div');
      dialog.style.cssText = `
        position: fixed;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        background: var(--md-sys-color-surface);
        border: 1px solid var(--md-sys-color-error);
        border-radius: 8px;
        padding: 20px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        z-index: 1000;
        min-width: 300px;
      `;
      
      dialog.innerHTML = `
        <h3 style="margin: 0 0 16px 0; color: var(--md-sys-color-error);">Delete Roster</h3>
        <p style="margin: 0 0 16px 0; color: var(--md-sys-color-on-surface);">
          Are you sure you want to delete this roster and all its characters? 
          This action cannot be undone and will permanently delete:
        </p>
        <ul style="margin: 0 0 16px 0; color: var(--md-sys-color-on-surface-variant); padding-left: 20px;">
          <li>All characters from this roster</li>
          <li>All tracking configurations</li>
          <li>All rested values</li>
          <li>All gold logs</li>
          <li>All completion status</li>
        </ul>
        <div style="display: flex; gap: 12px; justify-content: flex-end; margin-top: 20px;">
          <button id="cancel-btn" style="
            padding: 8px 16px;
            border: 1px solid var(--md-sys-color-outline);
            background: var(--md-sys-color-surface);
            color: var(--md-sys-color-on-surface);
            border-radius: 4px;
            cursor: pointer;
          ">Cancel</button>
          <button id="confirm-btn" style="
            padding: 8px 16px;
            border: none;
            background: var(--md-sys-color-error);
            color: var(--md-sys-color-on-error);
            border-radius: 4px;
            cursor: pointer;
          ">Delete</button>
        </div>
      `;
      
      document.body.appendChild(dialog);
      
      // Add event listeners
      const cleanup = () => {
        if (document.body.contains(dialog)) {
          document.body.removeChild(dialog);
        }
        document.removeEventListener('keydown', handleEscape);
      };
      
      const handleCancel = () => {
        console.log('User cancelled roster deletion');
        cleanup();
        resolve(false);
      };
      
      const handleConfirm = () => {
        console.log('User confirmed roster deletion');
        cleanup();
        resolve(true);
      };
      
      const handleEscape = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
          handleCancel();
          document.removeEventListener('keydown', handleEscape);
        }
      };
      
      // Add listeners
      document.getElementById('cancel-btn')?.addEventListener('click', handleCancel);
      document.getElementById('confirm-btn')?.addEventListener('click', handleConfirm);
      document.addEventListener('keydown', handleEscape);
    });
    
    console.log('User confirmed deletion:', confirmed);
    
    if (!confirmed) {
      console.log('Roster deletion cancelled by user');
      return;
    }
    
    try {
      isLoading = true;
      
      // Call backend to delete roster and all related data
      console.log('Calling backend delete_roster with roster_id:', rosterId);
      await invoke('delete_roster', { rosterId });
      
      // Use store functions for consistent updates
      const { removeRoster: storeRemoveRoster } = await import('$lib/store');
      storeRemoveRoster(rosterId);
      
      console.log('Roster deletion completed, updating stores');
      
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
    }
  }

  function handleRosterClick(rosterId: string) {
    console.log('Roster clicked:', rosterId);
    // Only set the active roster ID, don't load characters (we already have all characters)
    activeRosterId.set(rosterId);
    localStorage.setItem('activeRosterId', rosterId);
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
      console.log('Updating character order in database:', items);
      
      // Create simple character order updates with only required fields
      const characterUpdates = items.map((item, index) => ({
        char_id: parseInt(item.id),
        display_order: index
      }));
      
      console.log('Sending character updates:', characterUpdates);
      await invoke('update_character_order', { characters: characterUpdates });
    } catch (error) {
      console.error('Failed to update character order:', error);
    }
  }

  async function toggleGold(char: any) {
    const newStatus = !char.earns_gold;
    console.log(`FRONTEND: toggleGold called for ${char.char_name} (char_id: ${char.char_id}), new status: ${newStatus}`);
    
    // Check gold character limit for this roster
    const currentRosterCharacters = $characters.filter(c => c.roster_id === char.roster_id);
    const goldCharacterCount = currentRosterCharacters.filter(c => c.earns_gold).length;
    
    if (newStatus && goldCharacterCount >= 6) {
      console.log(`FRONTEND: Cannot enable gold for ${char.char_name} - limit of 6 gold characters per roster reached`);
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
      
      console.log(`FRONTEND: Successfully toggled gold for ${char.char_name} to ${newStatus}`);
    } catch (err) {
      console.error("FRONTEND: Toggle failed:", err);
    }
  }

  async function toggleHideFromDashboard(char: any) {
    const newStatus = !char.hide_from_dashboard;
    try {
      await updateCharacter(char.char_id, { hide_from_dashboard: newStatus });
      console.log(`FRONTEND: Set hide_from_dashboard=${newStatus} for ${char.char_name}`);
    } catch (err) {
      console.error("FRONTEND: Hide toggle failed:", err);
    }
  }

  async function updateCharacterEarnsGold(charId: number, earnsGold: boolean) {
    try {
      await invoke('update_character_earns_gold', { charId, earnsGold });
    } catch (error) {
      console.error('Failed to update character earns gold:', error);
    }
  }

  function getClassIcon(classId: string) {
    const gameClass = GAME_CLASSES[classId];
    return gameClass?.iconId || "0";
  }

  function getClassName(classId: string) {
    const gameClass = GAME_CLASSES[classId];
    return gameClass ? gameClass.displayName : classId;
  }

  function handleDragStart(event: DragEvent) {
    console.log('Drag started');
  }

  function handleDragEnd(event: DragEvent) {
    console.log('Drag ended');
  }
</script>

<div class="roster-settings">
  <div class="roster-section">
    <div class="settings-container">
  <!-- Success Message -->
  {#if showSuccessMessage}
    <div class="success-message {successMessage.includes('Failed') ? 'error' : successMessage.includes('Cannot enable gold') ? 'warning' : 'success'}">
      <span>{successMessage}</span>
    </div>
  {/if}

  <!-- Roster Management -->
      <button 
        class="add-button"
        on:click={() => { showAddRosterDialog = true; console.log('Add Roster clicked, isLoading:', isLoading); }}
        disabled={isLoading}
      >
        <span>+</span> Add Roster
      </button>
    </div>

    <div class="roster-list">
      {#each $rosters as roster}
        <div 
          class="roster-item"
          class:active={roster.id === $activeRosterId}
          on:click={() => handleRosterClick(roster.id)}
          on:keydown={(e) => e.key === 'Enter' && handleRosterClick(roster.id)}
          tabindex="0"
          role="button"
        >
          <div class="roster-info">
            <h4>{roster.roster_name}</h4>
            <p class="roster-id">ID: {roster.id}</p>
            <p class="character-count">
              {$characters.filter(c => c.roster_id === roster.id).length} characters
            </p>
          </div>
          <div class="roster-actions">
            <button 
              class="action-button secondary"
              on:click|stopPropagation={() => openRenameDialog(roster.id, roster.roster_name || '')}
              on:keydown|stopPropagation={(e) => e.key === 'Enter' && openRenameDialog(roster.id, roster.roster_name || '')}
            >
              Rename
            </button>
            <button 
              class="action-button danger"
              on:click|stopPropagation={() => removeRoster(roster.id)}
              on:keydown|stopPropagation={(e) => e.key === 'Enter' && removeRoster(roster.id)}
             >
              Remove
            </button>
          </div>
        </div>
      {/each}
      
      {#if $rosters.length === 0}
        <div class="empty-state">
          <p>No rosters configured yet.</p>
          <p>Click "Add Roster" to get started.</p>
        </div>
      {/if}
    </div>
  </div>

  <!-- Character Management -->
  {#if $activeRosterId && currentRoster}
    <div class="character-section">
      <div class="section-header">
        <h4>Characters in {currentRoster.roster_name}</h4>
      </div>
      <div class="character-list" 
        use:dndzone={dndOptions} 
        on:consider={handleDndConsider} 
        on:finalize={handleDndFinalize}>

        {#each dndItems as char (char.id)}
          <div class="character-item" animate:flip={{duration: 200}}>
            
            <div class="drag-handle" style="cursor: grab; padding: 10px; color: #555; display: flex; align-items: center;">
              <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="9" cy="5" r="1"/><circle cx="9" cy="12" r="1"/><circle cx="9" cy="19" r="1"/>
                <circle cx="15" cy="5" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="15" cy="19" r="1"/>
              </svg>
            </div>

            <div class="char-info">
              <img src="/images/classes/{getClassIcon(char.class_id)}.png" alt="" class="class-icon" />
              <div class="name-box">
                <span class="char-name">{char.char_name}</span>
                <span class="class-name">{getClassName(char.class_id)}</span>
              </div>
              <div class="stats">
                <div class="stat-item">iLvl: {char.item_level.toFixed(2)}</div>
                <div class="stat-item">CP: {char.combat_power.toFixed(2)}</div>
              </div>
            </div>

            <div class="actions">
              <button 
                class="toggle-btn gold" 
                class:active={char.earns_gold}
                on:click|stopPropagation={() => toggleGold(char)}
              >
                {char.earns_gold ? 'EARNS GOLD' : 'RAT'}
              </button>
              <button
                class="toggle-btn hide"
                class:active={char.hide_from_dashboard}
                on:click|stopPropagation={() => toggleHideFromDashboard(char)}
              >
                {char.hide_from_dashboard ? 'HIDDEN' : 'SHOW ON DASHBOARD'}
              </button>
            </div>
          </div>
        {:else}
          <div class="empty-state">
            <p>No characters in this roster yet.</p>
            <p>Add a roster to scrape character data.</p>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<!-- Rename Roster Dialog -->
{#if showRenameRosterDialog}
  <div class="dialog-overlay" role="button" tabindex="0" on:click={cancelRename} on:keydown={(e) => e.key === 'Enter' && cancelRename()} aria-label="Close dialog">
    <div class="dialog" on:click|stopPropagation role="dialog" tabindex="-1" on:keydown={(e) => e.key === 'Escape' && cancelRename()}>
      <div class="dialog-header">
        <h3>Rename Roster</h3>
        <button class="close-button" on:click={cancelRename}>×</button>
      </div>
      <div class="dialog-content">
        <div class="form-group">
          <label for="rename-roster-name">Roster Name</label>
          <input 
            id="rename-roster-name"
            type="text" 
            bind:value={renameRosterName}
            placeholder="Enter new roster name"
            on:keydown={(e) => e.key === 'Enter' && renameRoster()}
            disabled={isLoading}
          />
        </div>
        {#if isLoading}
          <div class="loading-indicator">
            <p>Renaming roster...</p>
          </div>
        {/if}
      </div>
      <div class="dialog-actions">
        <button class="button secondary" on:click={cancelRename} disabled={isLoading}>Cancel</button>
        <button class="button primary" on:click={renameRoster} disabled={isLoading || !renameRosterName.trim()}>
          {isLoading ? 'Renaming...' : 'Rename'}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Add Roster Dialog -->
{#if showAddRosterDialog}
  <div class="dialog-overlay" on:click={() => showAddRosterDialog = false} role="dialog" tabindex="-1" on:keydown={(e) => e.key === 'Escape' && (showAddRosterDialog = false)}>
    <div class="dialog" on:click|stopPropagation role="document" tabindex="-1" on:keydown={(e) => e.key === 'Escape' && (showAddRosterDialog = false)}>
      <div class="dialog-header">
        <h3>Add New Roster</h3>
        <button class="close-button" on:click={() => showAddRosterDialog = false}>×</button>
      </div>
      <div class="dialog-content">
        <div class="form-group">
          <label for="roster-name">Character Name</label>
          <input 
            id="roster-name"
            type="text" 
            bind:value={newRosterName}
            placeholder="Enter 1 character of yours (e.g. Vaanyar)"
            on:keydown={(e) => e.key === 'Enter' && addRoster()}
            disabled={isLoading}
          />
        </div>
        {#if isLoading}
          <div class="loading-indicator">
            <p>Scraping roster data...</p>
          </div>
        {/if}
      </div>
      <div class="dialog-actions">
        <button class="button secondary" on:click={() => showAddRosterDialog = false} disabled={isLoading}>Cancel</button>
        <button class="button primary" on:click={addRoster} disabled={isLoading || !newRosterName.trim()}>
          {isLoading ? 'Adding...' : 'Add Roster'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .roster-settings {
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--md-sys-color-outline);
    padding-bottom: 1rem;
    background: transparent;
    margin-top: 0;
  }

  .section-header h4 {
    margin: 0;
    color: var(--md-sys-color-primary);
    font-size: 1.3rem;
    font-weight: 600;
  }

  .character-count {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.9rem;
  }

  .add-button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.3s ease;
    box-shadow: 0 2px 8px rgba(103, 80, 164, 0.2);
  }

  .add-button:hover:not(:disabled) {
    background: var(--md-sys-color-primary-container);
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(103, 80, 164, 0.3);
  }

  .add-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  /* Roster List */
  .roster-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .roster-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--md-sys-color-surface);
    border: 2px solid var(--md-sys-color-outline);
    border-radius: 12px;
    padding: 1rem;
    transition: all 0.3s ease;
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }

  .roster-item:hover {
    border-color: var(--md-sys-color-primary);
    box-shadow: 0 4px 16px rgba(103, 80, 164, 0.2);
    transform: translateY(-2px);
  }

  .roster-item.active {
    border-color: var(--md-sys-color-primary);
    background: var(--md-sys-color-primary-container);
    box-shadow: 0 2px 8px rgba(103, 80, 164, 0.3);
  }

  .roster-info h4 {
    margin: 0 0 0.5rem 0;
    color: var(--md-sys-color-on-surface);
    font-size: 1.1rem;
    font-weight: 600;
  }

  .roster-info p {
    margin: 0.25rem 0;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.9rem;
  }

  .roster-actions {
    display: flex;
    gap: 0.5rem;
  }

  .action-button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.3s ease;
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface-variant);
    border: 1px solid var(--md-sys-color-outline);
  }

  .action-button.danger {
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
    border-color: var(--md-sys-color-error);
  }

  .action-button.danger:hover {
    background: var(--md-sys-color-error-container);
    border-color: var(--md-sys-color-error-container);
  }

  .action-button:hover {
    background: var(--md-sys-color-surface-container);
    border-color: var(--md-sys-color-primary);
    transform: translateY(-1px);
  }

  /* Character List - New Layout */
  .character-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .character-item {
    display: flex;
    align-items: center;
    background: var(--md-sys-color-surface);
    border: 2px solid var(--md-sys-color-outline);
    border-radius: 12px;
    padding: 1rem;
    gap: 1rem;
    transition: all 0.3s ease;
    cursor: grab;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }

  .character-item:hover {
    background: var(--md-sys-color-surface-variant);
    border-color: var(--md-sys-color-primary);
    transform: translateY(-2px);
    box-shadow: 0 4px 16px rgba(103, 80, 164, 0.2);
  }

  .character-item:active {
    cursor: grabbing;
  }

  .character-item .drag-handle:hover {
    color: var(--md-sys-color-primary) !important;
  }

  .char-info {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .class-icon {
    width: 32px;
    height: 32px;
    object-fit: contain;
    border-radius: 6px;
  }

  .name-box {
    display: flex;
    flex-direction: column;
    flex: 1;
  }

  .char-name {
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
    font-size: 1rem;
  }

  .class-name {
    font-size: 0.8rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .stats {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 80px;
  }

  .stat-item {
    font-family: monospace;
    color: var(--md-sys-color-on-surface-variant);
    font-weight: bold;
    text-align: center;
    font-size: 0.9rem;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    position: relative;
    z-index: 10;
  }

  .toggle-btn {
    padding: 0.5rem 0.75rem;
    border-radius: 8px;
    border: 1px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.2s;
    min-width: 90px;
    position: relative;
    z-index: 10;
    pointer-events: auto;
  }

  .toggle-btn.gold.active {
    border-color: var(--md-sys-color-tertiary);
    background: var(--md-sys-color-tertiary-container);
    color: var(--md-sys-color-on-tertiary-container);
  }

  .toggle-btn:hover {
    background: var(--md-sys-color-surface-container);
    transform: translateY(-1px);
  }

  .toggle-btn:active {
    transform: translateY(0);
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
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
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

  /* Dialog */
  .dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--md-sys-color-surface);
    border: 2px solid var(--md-sys-color-outline);
    border-radius: 16px;
    padding: 0;
    min-width: 400px;
    max-width: 90vw;
    max-height: 90vh;
    overflow: hidden;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  }

  .dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    border-bottom: 2px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface-variant);
  }

  .dialog-header h3 {
    margin: 0;
    color: var(--md-sys-color-primary);
    font-size: 1.3rem;
    font-weight: 600;
  }

  .close-button {
    background: none;
    border: none;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 1.5rem;
    cursor: pointer;
    padding: 0;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    transition: all 0.3s ease;
  }

  .close-button:hover {
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
  }

  .dialog-content {
    padding: 1.5rem;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.5rem;
    color: var(--md-sys-color-on-surface);
    font-weight: 600;
  }

  .form-group input {
    width: 100%;
    background: var(--md-sys-color-surface-variant);
    border: 2px solid var(--md-sys-color-outline);
    color: var(--md-sys-color-on-surface);
    padding: 0.75rem;
    border-radius: 8px;
    font-size: 1rem;
    transition: all 0.3s ease;
  }

  .form-group input:focus {
    outline: none;
    border-color: var(--md-sys-color-primary);
    box-shadow: 0 0 8px rgba(103, 80, 164, 0.2);
  }

  .loading-indicator {
    text-align: center;
    color: var(--md-sys-color-primary);
    margin-top: 1rem;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 1rem 1.5rem;
    border-top: 2px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface-variant);
  }

  .button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.3s ease;
  }

  .button.primary {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }

  .button.primary:hover:not(:disabled) {
    background: var(--md-sys-color-primary-container);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(103, 80, 164, 0.3);
  }

  .button.secondary {
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface-variant);
    border: 1px solid var(--md-sys-color-outline);
  }

  .button.secondary:hover:not(:disabled) {
    background: var(--md-sys-color-surface-container);
    border-color: var(--md-sys-color-primary);
  }

  .button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .empty-state {
    text-align: center;
    padding: 2rem;
    color: var(--md-sys-color-on-surface-variant);
    font-style: italic;
  }

  @media (max-width: 768px) {
    .section-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }
    
    .roster-item {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }
    
    .roster-actions {
      width: 100%;
      justify-content: flex-end;
    }
    
    .dialog {
      min-width: auto;
      margin: 1rem;
    }
    
  }
</style>
