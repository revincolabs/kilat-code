<script lang="ts">
  import { app } from "../lib/state.svelte";
</script>

<div class="toasts">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.level}">{t.message}</div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 36px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 100;
    max-width: 420px;
  }
  .toast {
    background: var(--bg-2);
    border: 1px solid var(--line-2);
    border-left: 3px solid var(--fg-2);
    border-radius: 8px;
    padding: 8px 12px;
    font-size: 12px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .toast.ok {
    border-left-color: var(--ok);
  }
  .toast.warn {
    border-left-color: var(--warn);
  }
  .toast.error {
    border-left-color: var(--err);
  }
</style>
