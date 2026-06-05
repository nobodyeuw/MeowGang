<script lang="ts">
  import { Dashboard, Marketplace, MeowConnect, Settings, Todo, UpdateTab } from '$lib/components/features';
  import type { AppTab, MeowConnectSection } from '$lib/types/app-shell';

  export let activeTab: AppTab = 'dashboard';
  export let activeSettingsTab = 'roster';
  export let activeMeowConnectTab: MeowConnectSection = 'together';
  export let meowConnectFeatureEnabled = true;
  export let highlightCharId: number | null = null;
  export let setHeaderContent: (content: string) => void;
  export let handlePendingRequestsChanged: (count: number) => void;
</script>

<main class="content" class:outer-scroll={activeTab !== 'todo' && activeTab !== 'settings'}>
  {#if activeTab === 'dashboard'}
    <Dashboard {setHeaderContent} />
  {:else if activeTab === 'todo'}
    <Todo highlightCharId={highlightCharId} />
  {:else if activeTab === 'settings'}
    <Settings activeSettingsTab={activeSettingsTab} on:tabChange={(e: CustomEvent<string>) => activeSettingsTab = e.detail} />
  {:else if activeTab === 'marketplace'}
    <Marketplace />
  {:else if activeTab === 'meow-connect' && meowConnectFeatureEnabled}
    <MeowConnect
      activeSection={activeMeowConnectTab}
      on:pendingRequestsChanged={(event: CustomEvent<number>) => handlePendingRequestsChanged(event.detail)}
    />
  {:else if activeTab === 'updates'}
    <UpdateTab />
  {/if}
</main>

<style>
  .content {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--md-sys-color-background);
  }

  .content.outer-scroll {
    overflow-y: auto;
  }

  @media (max-width: 768px) {
    .content {
      padding: 0;
    }
  }
</style>
