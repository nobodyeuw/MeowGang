<script lang="ts">
  import { onMount, tick } from 'svelte';
  // Temporarily disabled due to Supabase realtime message limits
  // import { hasMeowConnectConsent, isMeowConnectFeatureEnabled } from '$lib/services/meow-connect';
  // import type { MeowConnectSection } from '$lib/types/app-shell';

  type SettingsTab = 'roster' | 'todo' | 'raid' | 'system';

  interface GuideStep {
    title: string;
    body: string;
    tab: string;
    settingsTab?: SettingsTab;
    // meowConnectTab?: MeowConnectSection;
    target?: string;
    align?: 'left' | 'right';
    requiresMeowConnect?: boolean;
    waitForMeowConnectConsent?: boolean;
  }

  export let activeTab = 'dashboard';
  export let activeSettingsTab = 'roster';
  // export let activeMeowConnectTab: MeowConnectSection = 'together';
  export let characterCount = 0;
  export let appReady = false;
  export let switchTab: (tab: string) => void;
  export let setSettingsTab: (tab: SettingsTab) => void;
  // export let setMeowConnectTab: (tab: MeowConnectSection) => void;

  const storageKey = 'setupGuideDismissed';

  const allSteps: GuideStep[] = [
    {
      title: 'Set-Up Guide',
      body: 'This guide walks through the main app areas, first roster setup, tracking, raid gold, MeowConnect, system options, and updates. Skip it any time if you already know the flow.',
      tab: 'dashboard'
    },
    {
      title: 'Dashboard',
      body: 'The dashboard is the daily overview. It shows roster characters, tracked daily and weekly work, raid status, gold choices, and optional static or group tags.',
      tab: 'dashboard',
      target: '[data-guide="dashboard"]',
      align: 'right'
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
    //{
      //title: 'Enable MeowConnect',
     // body: 'In System, keep MeowConnect enabled if you want shared raid availability with friends. You can turn real-time updates on or off separately.',
      //tab: 'settings',
      //settingsTab: 'system',
      //target: '[data-guide="system-meowconnect"]'
    //},
    {
      title: 'Share Characters',
      body: 'Use the CONNECT toggle only for characters you want to share through MeowConnect. Characters left OFF stay local.',
      tab: 'settings',
      settingsTab: 'roster',
      target: '[data-guide="meow-connect-toggle"]',
      requiresMeowConnect: true
    },
    {
      title: 'Choose Tracking',
      body: 'In Tracking, choose which daily, weekly, and raid tasks should be visible in To Do and Dashboard. The All row checkbox changes that task for every character.',
      tab: 'settings',
      settingsTab: 'todo',
      target: '[data-guide="tracking-matrix"]'
    },
    {
      title: 'Rested And Lazy Tracking',
      body: 'Chaos and Guardian rows can store rested values. Lazy only counts those dailies when rested is high enough, which keeps low-priority characters quieter.',
      tab: 'settings',
      settingsTab: 'todo',
      target: '[data-guide="rested-input"]',
      align: 'right'
    },
    {
      title: 'Configure Raid Gold',
      body: 'In Raids, choose raid difficulty, Static/Friends, Take Gold, and Buy Box. Expand a raid if you need per-gate difficulty or box settings.',
      tab: 'settings',
      settingsTab: 'raid',
      target: '[data-guide="raid-matrix"]'
    },
    {
      title: 'Raid Bulk Controls',
      body: 'Use All Static and All Gold per raid row when multiple characters should receive the same reservation or gold setting. Buy Box stays individual.',
      tab: 'settings',
      settingsTab: 'raid',
      target: '[data-guide="raid-bulk-actions"]',
      align: 'right'
    },
    {
      title: 'To Do',
      body: 'The To Do tab is the action view. Check off daily tasks, weekly tasks, roster events, and raid gates as you complete them.',
      tab: 'todo',
      target: '[data-guide="todo"]',
      align: 'right'
    },
    {
      title: 'Marketplace',
      body: 'Marketplace tracks selected economy prices, supports favorites, manual overrides, category filters, sorting, and price history.',
      tab: 'marketplace',
      target: '[data-guide="marketplace"]',
      align: 'right'
    },
//    {
//      title: 'Accept MeowConnect',
//      body: 'Open MeowConnect and accept the sharing terms when you are ready. The guide waits here until MeowConnect consent is accepted.',
//      tab: 'meow-connect',
//      target: '[data-guide="meow-connect-consent"]',
//      requiresMeowConnect: true,
//      waitForMeowConnectConsent: true
//    },
//    {
//      title: 'MeowConnect',
//      body: 'Raid Together compares open raids between you and selected friends. After adding friends, it shows how many matching open raids you share.',
//      tab: 'meow-connect',
//      meowConnectTab: 'together',
//      target: '[data-guide="meow-connect"]',
//      requiresMeowConnect: true,
//      align: 'right'
//    },
//    {
//      title: 'Raid Together Details',
//      body: 'Select friends at the top, then expand a friend profile inside a raid card to see character details, available and cleared raids, reservations, and group assignment options.',
//      tab: 'meow-connect',
//      meowConnectTab: 'together',
//      target: '[data-guide="meow-connect-profile-details"]',
//      requiresMeowConnect: true,
//      align: 'right'
//    },
//    {
//      title: 'Sync Now',
//      body: 'After the initial roster setup, open MeowConnect Settings and press Sync now. The first upload can take a while because it prepares your connected characters, clears, logs, groups, and visibility data.',
//      tab: 'meow-connect',
//      meowConnectTab: 'settings',
//      target: '[data-guide="meow-connect-sync"]',
//      requiresMeowConnect: true
//    },
//    {
//      title: 'Add Friends',
//      body: 'Friends are added in MeowConnect Settings. Search for a whitelisted name, send a request, and accept incoming requests here.',
//      tab: 'meow-connect',
//      meowConnectTab: 'settings',
//      target: '[data-guide="meow-connect-friends"]',
//      requiresMeowConnect: true,
//      align: 'right'
//    },
//    {
//      title: 'Groups And Tags',
//      body: 'Groups let you organize static runs. Create a group, add an optional short tag, invite members, then assign characters to that group from Raid Together profile details.',
//      tab: 'meow-connect',
//      meowConnectTab: 'settings',
//      target: '[data-guide="meow-connect-groups"]',
//      requiresMeowConnect: true,
//      align: 'right'
//    },
    {
      title: 'General Options',
      body: 'General contains app-wide options like dashboard layout, static badge visibility, welcome behavior, header countdown, RAT To Do view, and themes.',
      tab: 'settings',
      settingsTab: 'system',
      target: '[data-guide="system-general"]'
    },
    {
      title: 'System Options',
      body: 'Startup controls when LOA Tracker shows itself and which companion apps it watches. File Paths connect encounters.db and optional executable paths.',
      tab: 'settings',
      settingsTab: 'system',
      target: '[data-guide="system-startup"]'
    },
    {
      title: 'File Paths',
      body: 'The encounters.db path powers automatic raid clear detection from LOA Logs. LostArk.exe and LOA Logs.exe paths are used for companion startup behavior.',
      tab: 'settings',
      settingsTab: 'system',
      target: '[data-guide="system-file-paths"]',
      align: 'right'
    },
    {
      title: 'Updates',
      body: 'The Updates tab shows app changelogs, known issues, planned features, and available updater actions.',
      tab: 'updates',
      target: '[data-guide="updates"]',
      align: 'right'
    },
    {
      title: 'Back To Dashboard',
      body: 'The dashboard now shows the information gathered from roster, tracking, raid, MeowConnect, and system setup.',
      tab: 'dashboard',
      target: '[data-guide="dashboard"]',
      align: 'right'
    }
  ];

  let isOpen = false;
  let currentStep = 0;
  let autoStarted = false;
  let hasMounted = false;
  // Temporarily disabled due to Supabase realtime message limits
  // let meowConnectEnabled = true;
  // let meowConnectConsentAccepted = false;
  let guidePlacementStyle = '';

  // Temporarily disabled due to Supabase realtime message limits
  // $: steps = allSteps.filter((step) => !step.requiresMeowConnect || meowConnectEnabled);
  $: steps = allSteps.filter((step) => !step.requiresMeowConnect);
  $: if (currentStep >= steps.length) currentStep = Math.max(0, steps.length - 1);
  $: current = steps[currentStep];
  $: canGoBack = currentStep > 0;
  $: isLastStep = currentStep === steps.length - 1;
  // $: isCurrentStepBlocked = Boolean(current?.waitForMeowConnectConsent && !meowConnectConsentAccepted);
  $: isCurrentStepBlocked = false;

  onMount(() => {
    hasMounted = true;
    // refreshMeowConnectGuideState();
    const dismissed = localStorage.getItem(storageKey) === 'true';
    if (!dismissed && appReady && characterCount === 0) {
      startGuide();
      autoStarted = true;
    }

    const startListener = () => startGuide();
    // Temporarily disabled due to Supabase realtime message limits
    // const meowConnectStateListener = () => refreshMeowConnectGuideState();
    window.addEventListener('setup-guide:start', startListener);
    // window.addEventListener('meow-connect-consent-changed', meowConnectStateListener);
    // window.addEventListener('meow-connect-feature-changed', meowConnectStateListener);

    return () => {
      window.removeEventListener('setup-guide:start', startListener);
      // Temporarily disabled due to Supabase realtime message limits
      // window.removeEventListener('meow-connect-consent-changed', meowConnectStateListener);
      // window.removeEventListener('meow-connect-feature-changed', meowConnectStateListener);
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
    if (!step) return;
    if (activeTab !== step.tab) {
      switchTab(step.tab);
    }

    if (step.settingsTab && activeSettingsTab !== step.settingsTab) {
      setSettingsTab(step.settingsTab);
    }

//    if (step.meowConnectTab && activeMeowConnectTab !== step.meowConnectTab) {
//      setMeowConnectTab(step.meowConnectTab);
//    }

    await tick();
    window.setTimeout(() => highlightTarget(step.target), 80);
  }

  // Temporarily disabled due to Supabase realtime message limits
  // function refreshMeowConnectGuideState() {
  //   meowConnectEnabled = isMeowConnectFeatureEnabled();
  //   meowConnectConsentAccepted = hasMeowConnectConsent();
  // }

  function highlightTarget(selector?: string) {
    clearHighlight();
    guidePlacementStyle = '';

    if (!selector) return;

    const target = document.querySelector(selector);
    if (!(target instanceof HTMLElement)) return;

    target.classList.add('setup-guide-target');
    target.scrollIntoView({ block: 'center', inline: 'center', behavior: 'smooth' });
    guidePlacementStyle = getGuidePlacementStyle(target);
  }

  function clearHighlight() {
    document.querySelectorAll('.setup-guide-target').forEach((element) => {
      element.classList.remove('setup-guide-target');
    });
  }

  // Keep the card away from the highlighted target so setup controls remain visible while explaining them.
  function getGuidePlacementStyle(target: HTMLElement): string {
    const rect = target.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const horizontal = rect.left + rect.width / 2 > viewportWidth / 2 ? 'left' : 'right';
    const vertical = rect.top + rect.height / 2 > viewportHeight / 2 ? 'top' : 'bottom';
    const edge = viewportWidth <= 768 ? '0.75rem' : '1.25rem';
    const topOffset = viewportWidth <= 768 ? '4.25rem' : '4.75rem';

    return [
      `--guide-left: ${horizontal === 'left' ? edge : 'auto'}`,
      `--guide-right: ${horizontal === 'right' ? edge : 'auto'}`,
      `--guide-top: ${vertical === 'top' ? topOffset : 'auto'}`,
      `--guide-bottom: ${vertical === 'bottom' ? edge : 'auto'}`
    ].join('; ');
  }

  function nextStep() {
    if (isCurrentStepBlocked) return;
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
  <section class:align-right={current.align === 'right'} class="setup-guide" style={guidePlacementStyle} aria-live="polite">
    <div class="guide-progress">
      <span>Step {currentStep + 1} of {steps.length}</span>
      <button type="button" class="text-button" on:click={finishGuide}>Skip guide</button>
    </div>

    <h2>{current.title}</h2>
    <p>{current.body}</p>

    <div class="guide-actions">
      <button type="button" class="secondary" on:click={previousStep} disabled={!canGoBack}>Back</button>
      <button type="button" class="primary" on:click={nextStep} disabled={isCurrentStepBlocked}>
        {isCurrentStepBlocked ? 'Waiting for consent' : isLastStep ? 'Finish' : 'Continue'}
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
    background: var(--app-color-subtle-scrim);
  }

  .setup-guide {
    position: fixed;
    left: var(--guide-left, 1.25rem);
    right: var(--guide-right, auto);
    top: var(--guide-top, auto);
    bottom: var(--guide-bottom, 1.25rem);
    z-index: 1003;
    width: min(420px, calc(100vw - 2.5rem));
    padding: 1rem;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 55%, var(--md-sys-color-outline));
    border-radius: 12px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    box-shadow: var(--app-shadow-md);
  }

  .setup-guide.align-right {
    left: var(--guide-left, auto);
    right: var(--guide-right, 1.25rem);
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
    outline: 3px solid var(--app-color-guide-highlight, #00e5ff) !important;
    outline-offset: 3px;
    box-shadow:
      0 0 0 5px color-mix(in srgb, var(--app-color-guide-highlight, #00e5ff) 22%, transparent),
      0 0 18px color-mix(in srgb, var(--app-color-guide-highlight, #00e5ff) 55%, transparent) !important;
  }

  @media (max-width: 768px) {
    .setup-guide,
    .setup-guide.align-right {
      left: var(--guide-left, 0.75rem);
      right: var(--guide-right, 0.75rem);
      top: var(--guide-top, auto);
      bottom: var(--guide-bottom, 0.75rem);
      width: auto;
    }
  }
</style>
