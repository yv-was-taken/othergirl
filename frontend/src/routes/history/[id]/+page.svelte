<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';

  import MessageBubble from '$lib/components/MessageBubble.svelte';
  import { apiFetch } from '$lib/api';
  import { auth } from '$lib/stores/auth';

  type ChatMessage = {
    id: string;
    sender_id: string;
    content: string;
    created_at: string;
    flagged?: boolean;
  };

  let messages: ChatMessage[] = $state([]);
  let loading = $state(false);

  onMount(loadChat);

  async function loadChat() {
    const authState = get(auth);
    if (!authState.token) return;

    loading = true;

    try {
      const id = get(page).params.id;
      const response = await apiFetch<{ id: string; messages: ChatMessage[] }>(`/api/chats/${id}`);
      messages = response.messages;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load chat');
    } finally {
      loading = false;
    }
  }
</script>

<section class="space-y-4">
  <div class="flex items-center justify-between">
    <h1 class="text-2xl font-bold">Chat Transcript</h1>
    <a class="btn-secondary" href="/history">Back</a>
  </div>

  {#if loading}
    <div class="surface p-4 text-slate-300">Loading...</div>
  {:else if messages.length === 0}
    <div class="surface p-4 text-slate-300">No messages in this chat.</div>
  {:else}
    <div class="surface space-y-3 p-4">
      {#each messages as message (message.id)}
        <MessageBubble
          message={{
            content: message.content,
            sender_id: message.sender_id,
            timestamp: message.created_at,
            flagged: message.flagged
          }}
          mine={message.sender_id === $auth.user?.id}
        />
      {/each}
    </div>
  {/if}
</section>
