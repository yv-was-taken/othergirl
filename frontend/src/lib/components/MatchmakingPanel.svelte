<script lang="ts">
  let {
    categories = [],
    languages = [],
    selectedCategory = $bindable(''),
    selectedLanguage = $bindable(''),
    queued = false,
    queuePosition = null,
    queueWaitSeconds = null,
    onjoin,
    onleave
  }: {
    categories?: { id: string; name: string }[];
    languages?: { id: string; name: string }[];
    selectedCategory?: string;
    selectedLanguage?: string;
    queued?: boolean;
    queuePosition?: number | null;
    queueWaitSeconds?: number | null;
    onjoin?: () => void;
    onleave?: () => void;
  } = $props();
</script>

<div class="surface space-y-4 p-4">
  <h2 class="text-lg font-semibold">Matchmaking</h2>

  <div>
    <label for="category-select" class="mb-1 block text-xs uppercase tracking-wide text-[var(--text-muted)]">Category</label>
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
    <label for="language-select" class="mb-1 block text-xs uppercase tracking-wide text-[var(--text-muted)]">Language</label>
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
    <div class="rounded-xl bg-[var(--bg-overlay)] p-3 text-sm text-[var(--text-secondary)]">
      Queue position: {queuePosition ?? '-'}
      <br />
      Estimated wait: {queueWaitSeconds ?? '-'}s
    </div>
    <button class="btn-secondary w-full" onclick={() => onleave?.()}>Leave queue</button>
  {:else}
    <button class="btn-primary w-full" onclick={() => onjoin?.()}>Find chat</button>
  {/if}
</div>
