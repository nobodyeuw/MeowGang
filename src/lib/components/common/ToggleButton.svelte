<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let pressed = false;
  export let disabled = false;
  export let onLabel = '✓';
  export let offLabel = '○';
  export let title = '';
  export let ariaLabel = 'Toggle';
  export let compact = false;

  const dispatch = createEventDispatcher<{ change: boolean }>();

  function toggle() {
    if (disabled) return;
    dispatch('change', !pressed);
  }
</script>

<button
  type="button"
  class="ui-toggle-button"
  class:active={pressed}
  class:compact
  {disabled}
  {title}
  aria-label={ariaLabel}
  aria-pressed={pressed}
  on:click={toggle}
  {...$$restProps}
>
  <slot>
    <span class="toggle-state-icon">{pressed ? onLabel : offLabel}</span>
  </slot>
</button>

<style>
  .toggle-state-icon {
    font-size: 0.95rem;
    font-weight: 800;
    line-height: 1;
  }
</style>
