<script lang="ts">
  import { onMount } from 'svelte';
  import { iconAsset } from '$lib/assets';
  import { rosters } from '$lib/store';
  import { isRosterTaskTracked } from '$lib/components/dashboard/helpers';
  import { loadDashboardSnapshot } from '$lib/services/dashboard';
  import { updateTodoRosterTaskStatus } from '$lib/services/todo';
  import { getGameDayStart } from '$lib/utils/availability';

  type DueRoster = {
    id: string;
    name: string;
  };

  const shipIcon = iconAsset('ship.png');
  let dueRosters: DueRoster[] = [];
  let loading = false;
  let refreshKey = '';
  let chooserOpen = false;
  $: dueRosterNames = dueRosters.map((roster) => roster.name).join(', ');
  $: reminderTitle = dueRosters.length > 0
    ? `Buy 300 Sea Coins in your Stronghold for: ${dueRosterNames}`
    : 'Buy 300 Sea Coins in your Stronghold';

  $: rosterKey = $rosters.map((roster) => roster.id).join('|');
  $: if ($rosters.length === 0 && dueRosters.length > 0) {
    dueRosters = [];
  }
  $: if (rosterKey && rosterKey !== refreshKey) {
    refreshKey = rosterKey;
    refreshReminderState();
  }

  onMount(() => {
    document.addEventListener('click', handleOutsideClick);
    window.addEventListener('todo-task-status-changed', refreshReminderState);
    window.addEventListener('roster-event-progress-updated', refreshReminderState);
    window.addEventListener('tracking-config-changed', refreshReminderState);
    refreshReminderState();

    return () => {
      document.removeEventListener('click', handleOutsideClick);
      window.removeEventListener('todo-task-status-changed', refreshReminderState);
      window.removeEventListener('roster-event-progress-updated', refreshReminderState);
      window.removeEventListener('tracking-config-changed', refreshReminderState);
    };
  });

  function handleOutsideClick(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    if (target?.closest('.sea-coin-reminder-wrap')) return;
    chooserOpen = false;
  }

  function isSeaCoinCompletedToday(completions: any[]): boolean {
    const currentDailyReset = getGameDayStart().getTime();
    return completions.some(
      (completion) =>
        completion.content_id === 'sea_coin' &&
        Number(completion.is_completed) === 1 &&
        Number(completion.timestamp || 0) >= currentDailyReset
    );
  }

  async function refreshReminderState() {
    if (loading) return;
    loading = true;

    try {
      const nextDueRosters: DueRoster[] = [];

      for (const roster of $rosters) {
        const snapshot = await loadDashboardSnapshot(roster.id);
        if (!isRosterTaskTracked(snapshot, 'ship_shop')) continue;

        const completions = snapshot.roster_completion_status || Object.values(snapshot.completion_by_character || {}).flat();
        if (!isSeaCoinCompletedToday(completions)) {
          nextDueRosters.push({ id: roster.id, name: roster.roster_name });
        }
      }

      dueRosters = nextDueRosters;
    } catch (error) {
      console.warn('Failed to refresh Sea Coin reminder state:', error);
    } finally {
      loading = false;
    }
  }

  function handleReminderClick(event: MouseEvent) {
    event.stopPropagation();

    if (dueRosters.length <= 1) {
      markSeaCoinsBought(dueRosters[0]);
      return;
    }

    chooserOpen = !chooserOpen;
  }

  async function markSeaCoinsBought(roster: DueRoster) {
    chooserOpen = false;
    dueRosters = dueRosters.filter((dueRoster) => dueRoster.id !== roster.id);

    try {
      await updateTodoRosterTaskStatus(roster.id, 'sea_coin', true);
      window.dispatchEvent(new CustomEvent('todo-task-status-changed', {
        detail: { rosterId: roster.id, taskId: 'sea_coin', completed: true }
      }));
    } catch (error) {
      console.warn('Failed to mark Sea Coin reminder completed:', error);
      await refreshReminderState();
    }
  }
</script>

{#if dueRosters.length > 0}
  <div class="sea-coin-reminder-wrap no-window-drag">
    <button
      type="button"
      class="sea-coin-reminder"
      title={reminderTitle}
      aria-label={reminderTitle}
      aria-expanded={chooserOpen}
      on:click={handleReminderClick}
    >
      <img src={shipIcon} alt="" />
      {#if dueRosters.length > 1}
        <span>{dueRosters.length}</span>
      {/if}
    </button>

    {#if chooserOpen && dueRosters.length > 1}
      <div class="sea-coin-chooser" role="menu" aria-label="Choose roster for Sea Coin reminder">
        <strong>Sea Coins bought for:</strong>
        {#each dueRosters as roster}
          <button type="button" role="menuitem" on:click|stopPropagation={() => markSeaCoinsBought(roster)}>
            {roster.name}
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .sea-coin-reminder-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .sea-coin-reminder {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 52%, var(--md-sys-color-outline));
    border-radius: 8px;
    background: color-mix(in srgb, var(--md-sys-color-primary) 9%, var(--md-sys-color-surface-container));
    box-shadow: var(--app-color-accent-glow-sm, none);
    cursor: pointer;
    animation: ship-wobble 2.4s ease-in-out infinite;
  }

  .sea-coin-reminder:hover {
    background: var(--app-color-hover-surface);
    border-color: var(--md-sys-color-primary);
  }

  .sea-coin-reminder img {
    width: 22px;
    height: 22px;
    object-fit: contain;
    pointer-events: none;
  }

  .sea-coin-reminder span {
    position: absolute;
    right: -5px;
    top: -5px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    font-size: 0.65rem;
    font-weight: 600;
    line-height: 16px;
  }

  .sea-coin-chooser {
    position: absolute;
    top: calc(100% + 0.5rem);
    left: 50%;
    z-index: 1500;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    width: max-content;
    min-width: 180px;
    max-width: min(260px, 80vw);
    padding: 0.65rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
    box-shadow: var(--app-shadow-md);
    transform: translateX(-50%);
  }

  .sea-coin-chooser strong {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface-variant);
  }

  .sea-coin-chooser button {
    padding: 0.4rem 0.55rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface-container);
    color: var(--md-sys-color-on-surface);
    font-size: 0.82rem;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }

  .sea-coin-chooser button:hover {
    border-color: var(--md-sys-color-primary);
    background: var(--app-color-hover-surface);
  }

  @keyframes ship-wobble {
    0%, 100% { transform: rotate(0deg) translateY(0); }
    20% { transform: rotate(-5deg) translateY(-1px); }
    40% { transform: rotate(4deg) translateY(0); }
    60% { transform: rotate(-3deg) translateY(1px); }
    80% { transform: rotate(2deg) translateY(0); }
  }
</style>
