<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  const appWindow = getCurrentWindow();

  async function runWindowCommand(action: () => Promise<void>, label: string) {
    try {
      await action();
    } catch (error) {
      console.warn(`Failed to ${label}:`, error);
    }
  }

  async function minimizeWindow() {
    await runWindowCommand(() => appWindow.minimize(), 'minimize window');
  }

  async function toggleMaximizeWindow() {
    await runWindowCommand(() => appWindow.toggleMaximize(), 'toggle window maximize state');
  }

  async function closeWindow() {
    await runWindowCommand(() => appWindow.close(), 'close window');
  }
</script>

<div class="window-controls" aria-label="Window controls">
  <button type="button" class="window-control minimize" aria-label="Minimize window" on:click={minimizeWindow}>
    <span></span>
  </button>
  <button type="button" class="window-control maximize" aria-label="Maximize window" on:click={toggleMaximizeWindow}>
    <span></span>
  </button>
  <button type="button" class="window-control close" aria-label="Close window" on:click={closeWindow}>
    <span></span>
  </button>
</div>

<style>
  .window-controls {
    display: flex;
    align-items: stretch;
    align-self: stretch;
    margin: -0.42rem -1.25rem -0.42rem 0;
  }

  .window-control {
    display: grid;
    place-items: center;
    width: 46px;
    min-height: 100%;
    border: none;
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    transition:
      background-color 0.15s ease,
      color 0.15s ease;
  }

  .window-control:hover {
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
  }

  .window-control.close:hover {
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
  }

  .window-control span {
    position: relative;
    display: block;
    width: 13px;
    height: 13px;
  }

  .window-control.minimize span::before {
    content: '';
    position: absolute;
    left: 1px;
    right: 1px;
    bottom: 2px;
    height: 1.5px;
    background: currentColor;
  }

  .window-control.maximize span::before {
    content: '';
    position: absolute;
    inset: 1px;
    border: 1.5px solid currentColor;
    border-radius: 1px;
  }

  .window-control.close span::before,
  .window-control.close span::after {
    content: '';
    position: absolute;
    top: 6px;
    left: 1px;
    right: 1px;
    height: 1.5px;
    background: currentColor;
  }

  .window-control.close span::before {
    transform: rotate(45deg);
  }

  .window-control.close span::after {
    transform: rotate(-45deg);
  }

  @media (max-width: 768px) {
    .window-controls {
      margin: -0.38rem -0.9rem -0.38rem 0;
    }

    .window-control {
      width: 42px;
    }
  }
</style>
