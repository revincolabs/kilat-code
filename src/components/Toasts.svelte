<script lang="ts">
  import { app } from "../lib/state.svelte";
</script>

<div class="toasts" role="status" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.level}">{t.message}</div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 14px;
    bottom: 38px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    z-index: 100;
    max-width: 420px;
  }
  .toast {
    background: var(--bg-2);
    border: 1px solid var(--line-2);
    border-radius: var(--radius);
    box-shadow:
      inset 2px 0 0 var(--fg-2),
      var(--shadow-pop);
    padding: 9px 12px 9px 14px;
    font-size: 12.5px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .toast.ok {
    box-shadow:
      inset 2px 0 0 var(--ok),
      var(--shadow-pop);
  }
  .toast.warn {
    box-shadow:
      inset 2px 0 0 var(--warn),
      var(--shadow-pop);
  }
  .toast.error {
    box-shadow:
      inset 2px 0 0 var(--err),
      var(--shadow-pop);
  }
</style>
