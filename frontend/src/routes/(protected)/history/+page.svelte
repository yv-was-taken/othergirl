<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';

  import { MessageCircle } from 'lucide-svelte';
  import Skeleton from '$lib/components/Skeleton.svelte';
  import { apiFetch } from '$lib/api';
  import { auth } from '$lib/stores/auth';

  type ChatSummary = {
    id: string;
    partner_id: string;
    partner_username: string;
    started_at: string;
    ended_at?: string;
  };

  let chats: ChatSummary[] = $state([]);
  let loading = $state(false);
  let keepOnly = $state(false);

  onMount(loadHistory);

  async function loadHistory() {
    if (!$auth.token) return;

    loading = true;
    try {
      chats = await apiFetch<ChatSummary[]>(keepOnly ? '/api/chats/keeps' : '/api/chats');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load history');
    } finally {
      loading = false;
    }
  }
</script>

<section class="space-y-4">
  <div class="flex items-center justify-between">
    <h1 class="text-2xl font-bold">Chat History</h1>
    <div class="flex gap-2">
      <button
        class={`btn-secondary ${!keepOnly ? 'bg-[var(--btn-secondary-hover)]' : ''}`}
        onclick={() => {
          keepOnly = false;
          loadHistory();
        }}
        disabled={loading}
      >All</button>
      <button
        class={`btn-secondary ${keepOnly ? 'bg-[var(--btn-secondary-hover)]' : ''}`}
        onclick={() => {
          keepOnly = true;
          loadHistory();
        }}
        disabled={loading}
      >Keeps</button>
      <button class="btn-secondary" onclick={loadHistory} disabled={loading}>Refresh</button>
    </div>
  </div>

  {#if !$auth.user}
    <div class="surface p-5 text-[var(--text-secondary)]">Login to view history.</div>
  {:else if loading}
    <div class="space-y-2">
      {#each Array(5) as _}
        <div class="surface flex items-center justify-between p-4">
          <Skeleton variant="text" width="40%" />
          <Skeleton variant="text" width="20%" height="0.625rem" />
        </div>
      {/each}
    </div>
  {:else if chats.length === 0}
    <div class="surface flex flex-col items-center justify-center py-12 text-center">
      <MessageCircle size={48} class="mb-4 text-[var(--text-muted)]" />
      <h2 class="text-lg font-semibold">No conversations yet</h2>
      <p class="mt-1 text-sm text-[var(--text-muted)]">Start chatting to see your history here</p>
      <a href="/chat" class="btn-primary mt-4">Start chatting</a>
    </div>
  {:else}
    <div class="space-y-2">
      {#each chats as chat}
        <a href={`/history/${chat.id}`} class="surface block p-4 transition hover:bg-[var(--bg-elevated)]">
          <div class="flex items-center justify-between">
            <p class="font-semibold">{chat.partner_username}</p>
            <p class="text-xs text-[var(--text-muted)]">{new Date(chat.started_at).toLocaleString()}</p>
          </div>
        </a>
      {/each}
    </div>
  {/if}
</section>
