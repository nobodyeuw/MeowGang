<script lang="ts">
  import { Dashboard, Marketplace, RaidManagement, Settings, Todo, UpdateTab } from '$lib/components/features';
  // Temporarily disabled due to Supabase realtime message limits
  // import { MeowConnect } from '$lib/components/features';
  import type { AppTab } from '$lib/types/app-shell';
  // import type { MeowConnectSection } from '$lib/types/app-shell';

  export let activeTab: AppTab = 'dashboard';
  export let activeSettingsTab = 'roster';
  // Temporarily disabled due to Supabase realtime message limits
  // export let activeMeowConnectTab: MeowConnectSection = 'together';
  // export let meowConnectFeatureEnabled = true;
  export let raidManagementVisible = false;
  export let discordAuthUserId = '';
  export let discordAuthUser = '';
  export let highlightCharId: number | null = null;
  export let setHeaderContent: (content: string) => void;
  // Temporarily disabled due to Supabase realtime message limits
  // export let handlePendingRequestsChanged: (count: number) => void;
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
  {:else if activeTab === 'raid-management' && raidManagementVisible}
    <RaidManagement discordId={discordAuthUserId} discordName={discordAuthUser} accessGranted={raidManagementVisible} />
  {:else if activeTab === 'updates'}
    <UpdateTab />
  {/if}
  <!-- Temporarily disabled due to Supabase realtime message limits -->
  <!-- {:else if activeTab === 'meow-connect' && meowConnectFeatureEnabled}
    <MeowConnect
      activeSection={activeMeowConnectTab}
      on:pendingRequestsChanged={(event: CustomEvent<number>) => handlePendingRequestsChanged(event.detail)}
    />
  -->
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
