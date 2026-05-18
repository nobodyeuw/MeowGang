<script lang="ts">
  import { onMount, tick } from 'svelte';

  type SettingsTab = 'roster' | 'todo' | 'raid' | 'system';

  interface GuideStep {
    title: string;
    body: string;
    tab: string;
    settingsTab?: SettingsTab;
    target?: string;
    align?: 'left' | 'right';
  }

  export let activeTab = 'dashboard';
  export let activeSettingsTab = 'roster';
  export let characterCount = 0;
  export let appReady = false;
  export let switchTab: (tab: string) => void;
  export let setSettingsTab: (tab: SettingsTab) => void;

  const storageKey = 'setupGuideDismissed';

  const steps: GuideStep[] = [
    {
      title: 'Set-Up Guide',
      body: 'This guide walks through the first roster, tracking, raid, and system setup. Skip it any time if you already know the flow.',
      tab: 'dashboard'
    },
    {
      title: 'Add Your Roster',
      body: 'Open Settings > Roster and add one of your characters. The app uses that character to scrape the roster.',
      tab: 'settings',
      settingsTab: 'roster',
      target: '[data-guide="add-roster"]'
    },
    {
      title: 'Pick Gold Earners',
      body: 'Change the characters that should earn raid gold from RAT to EARNS GOLD. Gold earners are used for weekly gold planning.',
      tab: 'settings',
      settingsTab: 'roster',
      target: '[data-guide="gold-toggle"]'
    },
    {
      title: 'Order Characters',
      body: 'Drag characters into your preferred order. Dashboard and setup matrices follow this roster order.',
      tab: 'settings',
      settingsTab: 'roster',
      target: '[data-guide="character-drag"]'
    },
    {
      title: 'Choose Tracking',
      body: 'In Tracking, uncheck tasks or raids you do not want to track. The row toggle changes that task for all characters. Rested Chaos/Guardian values are app values, so 20 in-game rested equals 10 here.',
      tab: 'settings',
      settingsTab: 'todo',
      target: '[data-guide="tracking-matrix"]'
    },
    {
      title: 'Configure Raid Gold',
      body: 'In Raids, choose which raids your gold earners take gold from and set the difficulty. Expand a raid if you need per-gate box settings.',
      tab: 'settings',
      settingsTab: 'raid',
      target: '[data-guide="raid-matrix"]'
    },
    {
      title: 'System Options',
      body: 'Check startup options and paths. For a complete tracking experience, install LOA Logs by Snoww from github.com/snoww/loa-logs/releases/latest.',
      tab: 'settings',
      settingsTab: 'system',
      target: '[data-guide="system-startup"]'
    },
    {
      title: 'Back To Dashboard',
      body: 'The dashboard now shows the information gathered from roster, tracking, raid, and system setup.',
      tab: 'dashboard',
      target: '[data-guide="dashboard"]',
      align: 'right'
    }
  ];

  let isOpen = false;
  let currentStep = 0;
  let autoStarted = false;
  let hasMounted = false;

  $: current = steps[currentStep];
  $: canGoBack = currentStep > 0;
  $: isLastStep = currentStep === steps.length - 1;

  onMount(() => {
    hasMounted = true;
    const dismissed = localStorage.getItem(storageKey) === 'true';
    if (!dismissed && appReady && characterCount === 0) {
      startGuide();
      autoStarted = true;
    }

    const startListener = () => startGuide();
    window.addEventListener('setup-guide:start', startListener);

    return () => {
      window.removeEventListener('setup-guide:start', startListener);
      clearHighlight();
    };
  });

  $: if (hasMounted && appReady && !autoStarted && characterCount === 0 && localStorage.getItem(storageKey) !== 'true') {
    autoStarted = true;
    startGuide();
  }

  $: if (isOpen && current) {
    navigateForStep(current);
  }

  async function startGuide() {
    currentStep = 0;
    isOpen = true;
    await navigateForStep(steps[0]);
  }

  async function navigateForStep(step: GuideStep) {
    if (activeTab !== step.tab) {
      switchTab(step.tab);
    }

    if (step.settingsTab && activeSettingsTab !== step.settingsTab) {
      setSettingsTab(step.settingsTab);
    }

    await tick();
    window.setTimeout(() => highlightTarget(step.target), 80);
  }

  function highlightTarget(selector?: string) {
    clearHighlight();

    if (!selector) return;

    const target = document.querySelector(selector);
    if (!(target instanceof HTMLElement)) return;

    target.classList.add('setup-guide-target');
    target.scrollIntoView({ block: 'center', inline: 'center', behavior: 'smooth' });
  }

  function clearHighlight() {
    document.querySelectorAll('.setup-guide-target').forEach((element) => {
      element.classList.remove('setup-guide-target');
    });
  }

  function nextStep() {
    if (isLastStep) {
      finishGuide();
      return;
    }

    currentStep += 1;
  }

  function previousStep() {
    if (!canGoBack) return;
    currentStep -= 1;
  }

  function finishGuide() {
    localStorage.setItem(storageKey, 'true');
    isOpen = false;
    currentStep = 0;
    clearHighlight();
  }
</script>

{#if isOpen}
  <div class="guide-scrim" aria-hidden="true"></div>
  <section class:align-right={current.align === 'right'} class="setup-guide" aria-live="polite">
    <div class="guide-progress">
      <span>Step {currentStep + 1} of {steps.length}</span>
      <button type="button" class="text-button" on:click={finishGuide}>Skip guide</button>
    </div>

    <h2>{current.title}</h2>
    <p>{current.body}</p>

    <div class="guide-actions">
      <button type="button" class="secondary" on:click={previousStep} disabled={!canGoBack}>Back</button>
      <button type="button" class="primary" on:click={nextStep}>
        {isLastStep ? 'Finish' : 'Continue'}
      </button>
    </div>
  </section>
{/if}

<style>
  .guide-scrim {
    position: fixed;
    inset: 0;
    z-index: 999;
    pointer-events: none;
    background: rgba(0, 0, 0, 0.22);
  }

  .setup-guide {
    position: fixed;
    left: 1.25rem;
    bottom: 1.25rem;
    z-index: 1003;
    width: min(420px, calc(100vw - 2.5rem));
    padding: 1rem;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 55%, var(--md-sys-color-outline));
    border-radius: 12px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.45);
  }

  .setup-guide.align-right {
    left: auto;
    right: 1.25rem;
  }

  .guide-progress {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.78rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  h2 {
    margin: 0 0 0.5rem;
    font-size: 1.1rem;
    color: var(--md-sys-color-primary);
  }

  p {
    margin: 0;
    line-height: 1.45;
    color: var(--md-sys-color-on-surface-variant);
  }

  .guide-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  button {
    border: 0;
    border-radius: 8px;
    padding: 0.6rem 0.9rem;
    cursor: pointer;
    font-weight: 700;
  }

  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .primary {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }

  .secondary {
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
    border: 1px solid var(--md-sys-color-outline);
  }

  .text-button {
    padding: 0;
    background: transparent;
    color: var(--md-sys-color-primary);
  }

  :global(.setup-guide-target) {
    position: relative;
    z-index: 1001 !important;
    outline: 3px solid var(--md-sys-color-primary) !important;
    outline-offset: 4px;
    box-shadow: 0 0 0 8px rgba(255, 107, 53, 0.22), 0 0 24px rgba(255, 107, 53, 0.55) !important;
  }

  @media (max-width: 768px) {
    .setup-guide,
    .setup-guide.align-right {
      left: 0.75rem;
      right: 0.75rem;
      bottom: 0.75rem;
      width: auto;
    }
  }
</style>
