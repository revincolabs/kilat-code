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
    display: flex;
    gap: 10px;
    background: var(--bg-2);
    border: 1px solid var(--line-2);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    padding: 10px 14px 10px 12px;
    font-size: 12.5px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .toast::before {
    content: "";
    width: 7px;
    height: 7px;
    margin-top: 0.45em;
    flex: none;
    border-radius: 50%;
    background: var(--fg-2);
  }
  .toast.ok::before {
    background: var(--ok);
  }
  .toast.warn::before {
    background: var(--warn);
  }
  .toast.error::before {
    background: var(--err);
  }
</style>
