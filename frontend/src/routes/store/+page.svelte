<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';

  import FlarePreview from '$lib/components/FlarePreview.svelte';
  import { apiFetch } from '$lib/api';
  import { auth } from '$lib/stores/auth';

  type StoreItem = {
    id: string;
    name: string;
    description: string;
    item_type: string;
    price_sparks: number;
    rarity: string;
    asset_data: Record<string, string>;
  };

  type OwnedItem = {
    id: string;
    name: string;
    item_type: string;
    is_equipped: boolean;
  };

  type EmoteItem = {
    id: string;
    token: string;
    name: string;
    image_url: string;
    price_sparks: number;
    is_active: boolean;
  };

  let items: StoreItem[] = $state([]);
  let owned: OwnedItem[] = $state([]);
  let equippedIds: string[] = $state([]);
  let emotes: EmoteItem[] = $state([]);
  let ownedEmoteIds: string[] = $state([]);
  let balance = $state(0);

  let loading = $state(false);

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    if (!$auth.token) return;

    loading = true;
    try {
      const [store, flare, sparks, emoteCatalog, myEmotes] = await Promise.all([
        apiFetch<{ items: StoreItem[] }>('/api/store/items'),
        apiFetch<{ owned: OwnedItem[]; equipped: { id: string }[] }>('/api/users/me/flare'),
        apiFetch<{ balance: number }>('/api/sparks/balance'),
        apiFetch<{ emotes: EmoteItem[] }>('/api/emotes'),
        apiFetch<{ emotes: EmoteItem[] }>('/api/emotes/me')
      ]);

      items = store.items;
      owned = flare.owned ?? [];
      equippedIds = (flare.equipped ?? []).map((x) => x.id);
      balance = sparks.balance;
      emotes = emoteCatalog.emotes ?? [];
      ownedEmoteIds = (myEmotes.emotes ?? []).map((emote) => emote.id);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load store');
    } finally {
      loading = false;
    }
  }

  async function buy(itemId: string) {
    try {
      await apiFetch('/api/store/purchase', {
        method: 'POST',
        body: JSON.stringify({ item_id: itemId })
      });
      toast.success('Item purchased');
      await refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Purchase failed');
    }
  }

  async function toggleEquip(itemId: string) {
    const next = equippedIds.includes(itemId)
      ? equippedIds.filter((id) => id !== itemId)
      : [...equippedIds, itemId];

    try {
      await apiFetch('/api/users/me/flare', {
        method: 'PUT',
        body: JSON.stringify({ flare_item_ids: next })
      });
      equippedIds = next;
      toast.success('Flare updated');
      await refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to equip item');
    }
  }

  function isOwned(itemId: string) {
    return owned.some((item) => item.id === itemId);
  }

  async function buyEmote(emoteId: string) {
    try {
      await apiFetch('/api/emotes/purchase', {
        method: 'POST',
        body: JSON.stringify({ emote_id: emoteId })
      });
      toast.success('Emote purchased');
      await refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to purchase emote');
    }
  }
</script>

<section class="space-y-4">
  <div class="flex items-center justify-between">
    <h1 class="text-2xl font-bold">Flare Store</h1>
    <span class="rounded-xl bg-[var(--bg-elevated)] px-3 py-1 text-sm text-[var(--text-secondary)]">{balance} sparks</span>
  </div>

  <FlarePreview />

  {#if !$auth.user}
    <div class="surface p-4 text-[var(--text-secondary)]">Login to use the store.</div>
  {:else if items.length === 0}
    <div class="surface p-4 text-[var(--text-secondary)]">No store items available.</div>
  {:else}
    <div class="grid gap-3 md:grid-cols-2">
      {#each items as item}
        <div class="surface space-y-2 p-4">
          <p class="font-semibold">{item.name}</p>
          <p class="text-sm text-[var(--text-secondary)]">{item.description}</p>
          <p class="text-xs text-[var(--text-muted)]">{item.item_type} • {item.rarity}</p>
          <p class="text-sm text-[var(--text-secondary)]">{item.price_sparks} sparks</p>

          {#if isOwned(item.id)}
            <button class="btn-secondary w-full" onclick={() => toggleEquip(item.id)} disabled={loading}>
              {equippedIds.includes(item.id) ? 'Unequip' : 'Equip'}
            </button>
          {:else}
            <button class="btn-primary w-full" onclick={() => buy(item.id)} disabled={loading}>Buy</button>
          {/if}
        </div>
      {/each}
    </div>

    <div class="surface space-y-3 p-4">
      <h2 class="text-lg font-semibold">Custom Emotes</h2>
      {#if emotes.length === 0}
        <p class="text-sm text-[var(--text-secondary)]">No emotes listed.</p>
      {:else}
        <div class="grid gap-3 md:grid-cols-2">
          {#each emotes as emote}
            <div class="rounded-2xl border border-[var(--border-default)] bg-[var(--bg-overlay)] p-3">
              <div class="mb-2 flex items-center gap-3">
                <img src={emote.image_url} alt={emote.token} class="h-8 w-8 rounded" />
                <div>
                  <p class="font-semibold">{emote.name}</p>
                  <p class="text-xs text-[var(--text-muted)]">{emote.token}</p>
                </div>
              </div>
              <p class="text-sm text-[var(--text-secondary)]">{emote.price_sparks} sparks</p>
              {#if ownedEmoteIds.includes(emote.id)}
                <button class="btn-secondary mt-2 w-full" disabled>Owned</button>
              {:else}
                <button class="btn-primary mt-2 w-full" onclick={() => buyEmote(emote.id)} disabled={loading}>
                  Buy Emote
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</section>
