<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  type SegmentedOption = {
    value: string;
    label: string;
    disabled?: boolean;
  };

  export let options: SegmentedOption[] = [];
  export let value = '';
  export let ariaLabel = 'Selection';
  export let density: 'default' | 'compact' = 'default';

  const dispatch = createEventDispatcher<{ change: string }>();

  function selectOption(option: SegmentedOption) {
    if (option.disabled || option.value === value) return;
    dispatch('change', option.value);
  }
</script>

<div class="ui-segmented" class:compact={density === 'compact'} role="group" aria-label={ariaLabel}>
  {#each options as option}
    <button
      type="button"
      class="ui-segmented-button"
      class:active={option.value === value}
      disabled={option.disabled}
      aria-pressed={option.value === value}
      on:click={() => selectOption(option)}
    >
      {option.label}
    </button>
  {/each}
</div>
