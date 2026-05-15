<script lang="ts">
</script>

<div class="progression-planner">
  <div class="wip-container">
    <div class="wip-icon">🚧</div>
    <h2 class="wip-title">Progression Planner</h2>
    <p class="wip-subtitle">This feature is currently in development</p>
    
    <div class="wip-details">
      <div class="detail-item">
        <span class="detail-icon">📈</span>
        <span class="detail-text">Plan your character progression path</span>
      </div>
      <div class="detail-item">
        <span class="detail-icon">🎯</span>
        <span class="detail-text">Set item level and gear upgrade goals</span>
      </div>
      <div class="detail-item">
        <span class="detail-icon">💰</span>
        <span class="detail-text">Estimate gold and material costs</span>
      </div>
    </div>
    
    <div class="wip-badge">
      <span class="badge-dot"></span>
      In Progress
    </div>
  </div>
</div>

<style>
  .progression-planner {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: calc(100vh - 120px);
    padding: 2rem;
  }

  .wip-container {
    text-align: center;
    max-width: 480px;
    padding: 3rem 2rem;
    background: var(--md-sys-color-surface-variant);
    border-radius: 16px;
    border: 1px solid var(--md-sys-color-outline-variant);
  }

  .wip-icon {
    font-size: 3.5rem;
    margin-bottom: 1rem;
  }

  .wip-title {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
    margin: 0 0 0.5rem;
  }

  .wip-subtitle {
    font-size: 0.95rem;
    color: var(--md-sys-color-on-surface-variant);
    margin: 0 0 2rem;
  }

  .wip-details {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-bottom: 2rem;
    text-align: left;
  }

  .detail-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: var(--md-sys-color-surface);
    border-radius: 8px;
    border: 1px solid var(--md-sys-color-outline-variant);
  }

  .detail-icon {
    font-size: 1.25rem;
    flex-shrink: 0;
  }

  .detail-text {
    font-size: 0.875rem;
    color: var(--md-sys-color-on-surface);
  }

  .wip-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1.25rem;
    background: var(--md-sys-color-tertiary-container);
    color: var(--md-sys-color-on-tertiary-container);
    border-radius: 20px;
    font-size: 0.875rem;
    font-weight: 600;
  }

  .badge-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--md-sys-color-tertiary);
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  @media (max-width: 480px) {
    .wip-container {
      padding: 2rem 1.5rem;
    }

    .wip-title {
      font-size: 1.25rem;
    }
  }
</style>
