<script lang="ts">
  import { getEmoteList } from '$lib/utils/emotes';

  let {
    open = $bindable(false),
    onselect,
    anchorEl
  }: {
    open: boolean;
    onselect: (token: string) => void;
    anchorEl?: HTMLElement;
  } = $props();

  let search = $state('');
  let pickerEl: HTMLDivElement | undefined = $state();

  const filtered = $derived.by(() => {
    const list = getEmoteList();
    if (!search.trim()) return list;
    const q = search.trim().toLowerCase();
    return list.filter(
      (e) => e.name.toLowerCase().includes(q) || e.token.toLowerCase().includes(q)
    );
  });

  function select(token: string) {
    onselect(token);
    open = false;
    search = '';
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      open = false;
      search = '';
    }
  }

  function onclickoutside(e: MouseEvent) {
    if (!open) return;
    const target = e.target as Node;
    if (pickerEl?.contains(target)) return;
    if (anchorEl?.contains(target)) return;
    open = false;
    search = '';
  }
</script>

<svelte:window onkeydown={onkeydown} onmousedown={onclickoutside} />

{#if open}
  <div
    bind:this={pickerEl}
    class="absolute bottom-full left-0 z-20 mb-2 w-80 rounded-xl border border-[var(--border-default)] bg-[var(--bg-popover)] p-3 shadow-xl backdrop-blur"
  >
    <input
      class="input mb-2 w-full text-sm"
      placeholder="Search emotes..."
      bind:value={search}
    />

    <div class="max-h-52 overflow-y-auto">
      {#if filtered.length === 0}
        <p class="py-4 text-center text-xs text-[var(--text-muted)]">No emotes found</p>
      {:else}
        <div class="grid grid-cols-8 gap-1">
          {#each filtered as emote (emote.token)}
            <button
              type="button"
              class="flex h-9 w-9 items-center justify-center rounded-lg transition-colors hover:bg-[var(--bg-elevated)]"
              title={emote.name}
              onclick={() => select(emote.token)}
            >
              <img src={emote.imageUrl} alt={emote.token} class="h-7 w-7" />
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}
