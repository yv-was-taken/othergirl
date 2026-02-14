<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let open = false;

  const dispatch = createEventDispatcher<{
    vote: { keep: boolean };
    close: undefined;
  }>();
</script>

{#if open}
  <div class="fixed inset-0 z-30 flex items-center justify-center bg-black/60 p-4">
    <div class="surface w-full max-w-md space-y-4 p-5">
      <h3 class="text-lg font-semibold">Keep this chat?</h3>
      <p class="text-sm text-slate-300">If both people choose keep, it will be saved in history.</p>
      <div class="flex gap-2">
        <button class="btn-primary flex-1" on:click={() => dispatch('vote', { keep: true })}>Keep</button>
        <button class="btn-secondary flex-1" on:click={() => dispatch('vote', { keep: false })}>Skip</button>
      </div>
      <button class="btn-secondary w-full" on:click={() => dispatch('close')}>Close</button>
    </div>
  </div>
{/if}
