<script lang="ts">
  const plannedFeatures = [
    {
      title: "Gold Costs Analysis",
      description: "Gold costs per % damage increase overview",
      icon: "💰",
      status: "planned"
    }
  ];
</script>

<div class="planned-features">
  <div class="features-header">
    <span class="header-icon">🚀</span>
    <span class="header-text">Planned Features</span>
  </div>
  
  <div class="features-list">
    {#each plannedFeatures as feature}
      <div class="feature-item">
        <div class="feature-icon">{feature.icon}</div>
        <div class="feature-content">
          <div class="feature-title">{feature.title}</div>
          <div class="feature-description">{feature.description}</div>
        </div>
        <div class="feature-status planned">Planned</div>
      </div>
    {/each}
  </div>
  
  <div class="features-footer">
    <small>More features coming soon!</small>
  </div>
</div>

<style>
  .planned-features {
    padding: 1rem;
    border-top: 1px solid var(--md-sys-color-outline-variant);
    margin-top: auto;
  }

  .features-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
  }

  .header-icon {
    font-size: 1rem;
  }

  .header-text {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface-variant);
  }

  .features-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .feature-item {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.5rem;
    background: var(--md-sys-color-surface-variant);
    border-radius: 6px;
    border: 1px solid var(--md-sys-color-outline-variant);
  }

  .feature-icon {
    font-size: 0.875rem;
    flex-shrink: 0;
    margin-top: 0.125rem;
  }

  .feature-content {
    flex: 1;
    min-width: 0;
  }

  .feature-title {
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
    margin-bottom: 0.25rem;
  }

  .feature-description {
    font-size: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
    line-height: 1.3;
  }

  .feature-status {
    font-size: 0.625rem;
    font-weight: 600;
    padding: 0.125rem 0.375rem;
    border-radius: 4px;
    text-transform: uppercase;
    flex-shrink: 0;
  }

  .feature-status.planned {
    background: var(--md-sys-color-tertiary-container);
    color: var(--md-sys-color-on-tertiary-container);
  }

  .features-footer {
    margin-top: 0.75rem;
    text-align: center;
  }

  .features-footer small {
    font-size: 0.7rem;
    color: var(--md-sys-color-on-surface-variant);
    opacity: 0.7;
  }

  @media (max-width: 768px) {
    .planned-features {
      padding: 0.75rem;
    }

    .feature-title {
      font-size: 0.75rem;
    }

    .feature-description {
      font-size: 0.7rem;
    }
  }
</style>
