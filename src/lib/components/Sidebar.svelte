<script lang="ts">
  export let activeTab: string;
  export let switchTab: (tab: string) => void;
  export let isOpen: boolean = false;
  export let discordAuthUser: string = '';
</script>

<nav class="sidebar" class:isOpen={isOpen}>
  <div class="nav-items">
    <button 
      class="nav-item"
      class:active={activeTab === 'dashboard'}
      on:click={() => switchTab('dashboard')}
    >
      <span class="nav-icon">📊</span>
      <span class="nav-text">Dashboard</span>
    </button>
    
    <button 
      class="nav-item"
      class:active={activeTab === 'todo'}
      on:click={() => switchTab('todo')}
    >
      <span class="nav-icon">✅</span>
      <span class="nav-text">To Do's</span>
    </button>
    
    <button 
      class="nav-item"
      class:active={activeTab === 'progression'}
      on:click={() => switchTab('progression')}
    >
      <span class="nav-icon">📈</span>
      <span class="nav-text">Progression Planner</span>
    </button>

    <div class="nav-spacer"></div>

    <button 
      class="nav-item"
      class:active={activeTab === 'settings'}
      on:click={() => switchTab('settings')}
    >
      <span class="nav-icon">⚙️</span>
      <span class="nav-text">Settings</span>
    </button>
  </div>

  <div class="sidebar-footer">
    <div class="sidebar-divider"></div>
    <button 
      class="nav-item update-item"
      class:active={activeTab === 'updates'}
      on:click={() => switchTab('updates')}
    >
      <span class="nav-icon">🔄</span>
      <span class="nav-text">Updates</span>
    </button>
    {#if discordAuthUser}
      <div class="sidebar-user">
        <span class="user-label">Welcome</span>
        <span class="user-name">{discordAuthUser}</span>
      </div>
    {/if}
  </div>

</nav>

<style>
  .sidebar {
    position: fixed;
    top: 0;
    left: 0;
    width: 280px;
    height: 100vh;
    background: var(--md-sys-color-surface);
    z-index: 1002;
    transform: translateX(-100%);
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 5px 0 15px rgba(0,0,0,0.5);
    border-right: 1px solid var(--md-sys-color-outline);
    display: flex;
    flex-direction: column;
  }

  .nav-items {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .nav-spacer {
    height: 1rem;
  }

  .sidebar-footer {
    padding: 0 0 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .sidebar-divider {
    height: 1px;
    background: var(--md-sys-color-outline);
    opacity: 0.35;
    margin: 0 1.5rem;
  }

  .sidebar-user {
    margin: 0 1.5rem;
    padding: 0.48rem 0.6rem;
    border: 1px solid rgba(255, 140, 0, 0.25);
    border-radius: 6px;
    background: color-mix(in srgb, var(--md-sys-color-surface-variant) 70%, transparent);
    color: var(--md-sys-color-on-surface-variant);
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
    text-align: center;
  }

  .user-label {
    font-size: 0.58rem;
    font-weight: 800;
    line-height: 1;
    text-transform: uppercase;
    color: var(--md-sys-color-on-surface-variant);
    opacity: 0.75;
  }

  .user-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-sys-color-on-surface);
    font-size: 0.76rem;
    font-weight: 800;
    line-height: 1.15;
  }

  .sidebar.isOpen {
    transform: translateX(0);
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 1rem;
    width: 100%;
    padding: 1rem 1.5rem;
    background: none;
    border: none;
    border-radius: 0;
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
    font-size: 1rem;
    color: var(--md-sys-color-on-surface-variant);
    position: relative;
  }

  .nav-item:hover {
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
  }

  .nav-item.active {
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }

  .nav-item.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 4px;
    background: var(--md-sys-color-primary);
  }

  .nav-icon {
    font-size: 1.25rem;
    width: 24px;
    text-align: center;
  }

  .nav-text {
    font-weight: 500;
  }

  @media (max-width: 768px) {
    .sidebar {
      width: 260px;
    }

    .nav-item {
      padding: 0.875rem 1.25rem;
      font-size: 0.9rem;
    }

    .nav-icon {
      font-size: 1.125rem;
      width: 20px;
    }
  }

  @media (max-width: 480px) {
    .sidebar {
      width: 240px;
    }

    .nav-item {
      padding: 0.75rem 1rem;
      font-size: 0.875rem;
    }

    .nav-icon {
      font-size: 1rem;
      width: 18px;
    }
  }
</style>
