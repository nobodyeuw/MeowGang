<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getTimeUntilAvailable, isTaskAvailable } from '$lib/utils/availability';

  export let taskId: string;
  export let taskName: string;

  let interval: ReturnType<typeof setInterval>;
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
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 10%, transparent);
    border-radius: 8px;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-on-surface) 20%, transparent);
    min-width: 80px;
  }

  .countdown-time {
    font-size: 12px;
    font-weight: 600;
    color: var(--md-sys-color-warning);
    white-space: nowrap;
  }

  .countdown-label {
    font-size: 10px;
    color: var(--md-sys-color-on-surface-variant);
    white-space: nowrap;
  }

  .available-indicator {
    padding: 4px 8px;
    background: color-mix(in srgb, var(--md-sys-color-success) 20%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-success) 40%, transparent);
    border-radius: 8px;
  }

  .available-text {
    font-size: 11px;
    color: var(--md-sys-color-success);
    font-weight: 500;
  }

  .unavailable-indicator {
    padding: 4px 8px;
    background: color-mix(in srgb, var(--md-sys-color-error) 20%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-error) 40%, transparent);
    border-radius: 8px;
  }

  .unavailable-text {
    font-size: 11px;
    color: var(--md-sys-color-error);
    font-weight: 500;
  }
</style>
