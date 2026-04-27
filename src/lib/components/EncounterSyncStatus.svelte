<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EncounterSyncService, type SyncResult } from '../services/encounter-sync';
  
  let isSyncing = false;
  let lastSyncResult: SyncResult | null = null;
  let error: string | null = null;
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    // Set up event listeners for automatic sync events
    unlisten = await EncounterSyncService.listenToSyncEvents((event, payload) => {
      console.log('Encounter sync event:', event, payload);
      
      switch (event) {
        case 'encounter-sync-start':
          isSyncing = true;
          error = null;
          break;
        case 'encounter-sync-progress':
          // Could update progress UI here if needed
          break;
        case 'encounter-sync-complete':
        case 'encounters-force-sync-complete':
          isSyncing = false;
          lastSyncResult = payload;
          break;
      }
    });

    // Auto-start sync on component mount
    await startAutoSync();
  });

  onDestroy(() => {
    if (unlisten) {
      unlisten();
    }
  });

  async function startAutoSync() {
    try {
      isSyncing = true;
      error = null;
      lastSyncResult = await EncounterSyncService.syncEncountersToCompletions();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unknown error occurred';
      isSyncing = false;
    }
  }

  $: syncSuccess = lastSyncResult && lastSyncResult.synced_count > 0;
  $: hasErrors = lastSyncResult && lastSyncResult.errors.length > 0;
  $: lastSyncTime = lastSyncResult ? new Date().toLocaleTimeString() : null;
</script>

<div class="encounter-sync-status p-3 bg-white rounded-lg shadow-sm border border-gray-200">
  <div class="flex items-center justify-between mb-2">
    <h3 class="text-sm font-semibold text-gray-700">LOA Logs Sync</h3>
    <div class="flex items-center gap-2">
      {#if isSyncing}
        <div class="flex items-center gap-1">
          <div class="w-2 h-2 bg-blue-500 rounded-full animate-pulse"></div>
          <span class="text-xs text-blue-600">Syncing...</span>
        </div>
      {:else if syncSuccess}
        <div class="flex items-center gap-1">
          <div class="w-2 h-2 bg-green-500 rounded-full"></div>
          <span class="text-xs text-green-600">Synced</span>
        </div>
      {:else if hasErrors}
        <div class="flex items-center gap-1">
          <div class="w-2 h-2 bg-red-500 rounded-full"></div>
          <span class="text-xs text-red-600">Error</span>
        </div>
      {:else}
        <div class="flex items-center gap-1">
          <div class="w-2 h-2 bg-gray-400 rounded-full"></div>
          <span class="text-xs text-gray-600">Ready</span>
        </div>
      {/if}
    </div>
  </div>

  <!-- Error Display -->
  {#if error}
    <div class="mb-2 p-2 bg-red-50 border border-red-200 text-red-700 rounded text-xs">
      {error}
    </div>
  {/if}

  <!-- Sync Results Summary -->
  {#if lastSyncResult}
    <div class="text-xs space-y-1">
      <div class="flex justify-between">
        <span class="text-gray-600">Last sync:</span>
        <span class="text-gray-800">{lastSyncTime}</span>
      </div>
      
      <div class="grid grid-cols-2 gap-2">
        <div class="flex justify-between">
          <span class="text-gray-600">Synced:</span>
          <span class="{syncSuccess ? 'text-green-600 font-medium' : 'text-gray-600'}">{lastSyncResult.synced_count}</span>
        </div>
        <div class="flex justify-between">
          <span class="text-gray-600">Skipped:</span>
          <span class="text-gray-600">{lastSyncResult.skipped_count}</span>
        </div>
      </div>
      
      <div class="flex justify-between">
        <span class="text-gray-600">Duration:</span>
        <span class="text-gray-600">{lastSyncResult.duration_ms}ms</span>
      </div>
      
      {#if hasErrors}
        <div class="pt-1 border-t border-gray-200">
          <div class="flex justify-between">
            <span class="text-red-600">Errors:</span>
            <span class="text-red-600">{lastSyncResult.errors.length}</span>
          </div>
          {#if lastSyncResult.errors.length > 0}
            <div class="mt-1 text-red-600">
              {#each lastSyncResult.errors.slice(0, 2) as error}
                <div class="truncate" title={error}>{error}</div>
              {/each}
              {#if lastSyncResult.errors.length > 2}
                <div class="text-xs text-red-500">+{lastSyncResult.errors.length - 2} more...</div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .animate-pulse {
    animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }
  
  @keyframes pulse {
    0%, 100% {
      opacity: 1;
    }
    50% {
      opacity: .5;
    }
  }
</style>
