<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';

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
        class={`btn-secondary ${!keepOnly ? 'bg-white/20' : ''}`}
        onclick={() => {
          keepOnly = false;
          loadHistory();
        }}
        disabled={loading}
      >All</button>
      <button
        class={`btn-secondary ${keepOnly ? 'bg-white/20' : ''}`}
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
    <div class="surface p-5 text-slate-300">Login to view history.</div>
  {:else if chats.length === 0}
    <div class="surface p-5 text-slate-300">No chats yet.</div>
  {:else}
    <div class="space-y-2">
      {#each chats as chat}
        <a href={`/history/${chat.id}`} class="surface block p-4 transition hover:bg-white/10">
          <div class="flex items-center justify-between">
            <p class="font-semibold">{chat.partner_username}</p>
            <p class="text-xs text-slate-400">{new Date(chat.started_at).toLocaleString()}</p>
          </div>
        </a>
      {/each}
    </div>
  {/if}
</section>
