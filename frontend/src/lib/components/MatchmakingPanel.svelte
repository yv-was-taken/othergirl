<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let categories: { id: string; name: string }[] = [];
  export let languages: { id: string; name: string }[] = [];

  export let selectedCategory = '';
  export let selectedLanguage = '';

  export let queued = false;
  export let queuePosition: number | null = null;
  export let queueWaitSeconds: number | null = null;

  const dispatch = createEventDispatcher<{
    join: undefined;
    leave: undefined;
  }>();
</script>

<div class="surface space-y-4 p-4">
  <h2 class="text-lg font-semibold">Matchmaking</h2>

  <div>
    <label for="category-select" class="mb-1 block text-xs uppercase tracking-wide text-slate-400">Category</label>
    <select id="category-select" class="input" bind:value={selectedCategory}>
      {#if categories.length === 0}
        <option value="">No categories</option>
      {:else}
        {#each categories as category}
          <option value={category.id}>{category.name}</option>
        {/each}
      {/if}
    </select>
  </div>

  <div>
    <label for="language-select" class="mb-1 block text-xs uppercase tracking-wide text-slate-400">Language</label>
    <select id="language-select" class="input" bind:value={selectedLanguage}>
      {#if languages.length === 0}
        <option value="">No languages</option>
      {:else}
        {#each languages as language}
          <option value={language.id}>{language.name}</option>
        {/each}
      {/if}
    </select>
  </div>

  {#if queued}
    <div class="rounded-xl bg-white/5 p-3 text-sm text-slate-300">
      Queue position: {queuePosition ?? '-'}
      <br />
      Estimated wait: {queueWaitSeconds ?? '-'}s
    </div>
    <button class="btn-secondary w-full" on:click={() => dispatch('leave')}>Leave queue</button>
  {:else}
    <button class="btn-primary w-full" on:click={() => dispatch('join')}>Find chat</button>
  {/if}
</div>
