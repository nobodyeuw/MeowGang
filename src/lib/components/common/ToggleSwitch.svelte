<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let checked = false;
  export let disabled = false;
  export let ariaLabel = 'Toggle setting';

  const dispatch = createEventDispatcher<{ change: boolean }>();

  function handleChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    dispatch('change', input.checked);
  }
</script>

<label class="toggle-switch-control" aria-label={ariaLabel}>
  <input type="checkbox" {checked} {disabled} on:change={handleChange} />
  <span class="ui-switch" class:active={checked} aria-hidden="true"></span>
</label>

<style>
  .toggle-switch-control {
    display: inline-flex;
    align-items: center;
    flex: 0 0 auto;
    cursor: pointer;
  }

  .toggle-switch-control:has(input:disabled) {
    cursor: not-allowed;
    opacity: 0.55;
  }

  input {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    border: 0;
    clip: rect(0 0 0 0);
    overflow: hidden;
    white-space: nowrap;
  }
</style>
