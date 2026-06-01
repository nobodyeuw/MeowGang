<script lang="ts">
  import ToggleSwitch from '$lib/components/common/ToggleSwitch.svelte';

  export let meowConnectEnabled = true;
  export let meowConnectRealtimeEnabled = true;
  export let onToggleMeowConnectEnabled: () => void;
  export let onToggleMeowConnectRealtime: () => void;
</script>

<div class="settings-section" data-guide="system-meowconnect">
  <div class="section-header">
    <div class="section-icon">
      <img src="/images/meowconnect_tab.png" alt="" class="section-image-icon" />
    </div>
    <div>
      <h3>MeowConnect</h3>
      <p>Control shared availability and live cloud updates</p>
    </div>
  </div>

  <div class="settings-grid">
    <div class="setting-card toggle-card">
      <div class="setting-header">
        <div class="setting-icon meowconnect-icon">
          <img src="/images/meowconnect_tab.png" alt="" />
        </div>
        <div class="toggle-content">
          <h4>MeowConnect</h4>
          <p>Enable or disable the MeowConnect feature</p>
        </div>
        <ToggleSwitch checked={meowConnectEnabled} ariaLabel="Enable MeowConnect" on:change={onToggleMeowConnectEnabled} />
      </div>
    </div>

    <div class="setting-card toggle-card" class:disabled-card={!meowConnectEnabled}>
      <div class="setting-header">
        <div class="setting-icon meowconnect-icon">
          <img src="/images/meowconnect_tab.png" alt="" />
        </div>
        <div class="toggle-content">
          <h4>Real-time MeowConnect</h4>
          <p>Enable real-time raid completion syncing with the cloud database</p>
        </div>
        <ToggleSwitch
          checked={meowConnectRealtimeEnabled}
          disabled={!meowConnectEnabled}
          ariaLabel="Enable real-time MeowConnect"
          on:change={onToggleMeowConnectRealtime}
        />
      </div>
    </div>
  </div>
</div>

<style>
  .disabled-card {
    opacity: 0.62;
  }
</style>
