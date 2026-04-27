<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getTimeUntilAvailable, isTaskAvailable } from '../utils/availability';

  export let taskId: string;
  export let taskName: string;

  let interval: NodeJS.Timeout;
  let timeUntil = '';
  let isAvailable = false;

  function updateTime() {
    isAvailable = isTaskAvailable(taskId);
    if (!isAvailable) {
      timeUntil = getTimeUntilAvailable(taskId);
    }
  }

  onMount(() => {
    updateTime();
    // Update every minute
    interval = setInterval(updateTime, 60000);
  });

  onDestroy(() => {
    if (interval) clearInterval(interval);
  });
</script>

{#if isAvailable}
  <div class="available-indicator">
    <span class="available-text">Available</span>
  </div>
{:else if timeUntil}
  <div class="countdown-container">
    <div class="countdown-time">{timeUntil}</div>
    <div class="countdown-label">until {taskName}</div>
  </div>
{:else}
  <div class="unavailable-indicator">
    <span class="unavailable-text">Unavailable</span>
  </div>
{/if}

<style>
  .countdown-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 4px 8px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    min-width: 80px;
  }

  .countdown-time {
    font-size: 12px;
    font-weight: 600;
    color: #fbbf24;
    white-space: nowrap;
  }

  .countdown-label {
    font-size: 10px;
    color: #9ca3af;
    white-space: nowrap;
  }

  .available-indicator {
    padding: 4px 8px;
    background: rgba(34, 197, 94, 0.2);
    border: 1px solid rgba(34, 197, 94, 0.4);
    border-radius: 8px;
  }

  .available-text {
    font-size: 11px;
    color: #22c55e;
    font-weight: 500;
  }

  .unavailable-indicator {
    padding: 4px 8px;
    background: rgba(239, 68, 68, 0.2);
    border: 1px solid rgba(239, 68, 68, 0.4);
    border-radius: 8px;
  }

  .unavailable-text {
    font-size: 11px;
    color: #ef4444;
    font-weight: 500;
  }
</style>
