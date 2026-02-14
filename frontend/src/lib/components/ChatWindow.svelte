<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  import MessageBubble from '$lib/components/MessageBubble.svelte';
  import TypingIndicator from '$lib/components/TypingIndicator.svelte';

  export let messages: {
    id: string;
    sender_id: string;
    content: string;
    timestamp: string;
    flagged?: boolean;
  }[] = [];
  export let currentUserId: string | null = null;
  export let partnerTyping = false;
  export let connected = false;
  export let partnerName = 'stranger';

  const dispatch = createEventDispatcher<{
    send: { content: string };
    leave: undefined;
    next: undefined;
    typing: { isTyping: boolean };
    keep_vote: { keep: boolean };
    award: { award_type: string; spark_amount: number };
  }>();

  let draft = '';
  let sparkAmount = 100;

  function submitMessage() {
    const content = draft.trim();
    if (!content || !connected) return;

    dispatch('send', { content });
    draft = '';
    dispatch('typing', { isTyping: false });
  }

  function sendAward() {
    if (!connected || sparkAmount <= 0) return;

    dispatch('award', {
      award_type: 'spark',
      spark_amount: sparkAmount
    });
  }
</script>

<div class="surface flex h-[72vh] flex-col">
  <div class="flex items-center justify-between border-b border-white/10 px-4 py-3">
    <div>
      <h2 class="font-semibold">Active Chat</h2>
      <p class="text-xs text-slate-400">with {partnerName}</p>
    </div>
    <div class="flex items-center gap-2">
      <button class="btn-secondary" on:click={() => dispatch('keep_vote', { keep: true })} disabled={!connected}>Keep</button>
      <button class="btn-secondary" on:click={() => dispatch('keep_vote', { keep: false })} disabled={!connected}>Skip Keep</button>
      <button class="btn-secondary" on:click={() => dispatch('leave')}>Leave</button>
      <button class="btn-primary" on:click={() => dispatch('next')}>Next</button>
    </div>
  </div>

  <div class="flex-1 space-y-3 overflow-y-auto px-4 py-4">
    {#if messages.length === 0}
      <p class="text-sm text-slate-400">No messages yet. Say hi.</p>
    {/if}

    {#each messages as message (message.id)}
      <MessageBubble message={message} mine={message.sender_id === currentUserId} />
    {/each}
  </div>

  <div class="border-t border-white/10 px-4 py-3">
    <TypingIndicator isTyping={partnerTyping} />

    <div class="mt-2 flex items-center gap-2">
      <input class="input max-w-36" type="number" min="1" bind:value={sparkAmount} disabled={!connected} />
      <button class="btn-secondary" on:click={sendAward} disabled={!connected}>Send Award</button>
    </div>

    <form
      class="mt-2 flex items-center gap-2"
      on:submit|preventDefault={submitMessage}
    >
      <input
        class="input"
        placeholder={connected ? 'Type a message...' : 'Connect to a chat first'}
        bind:value={draft}
        disabled={!connected}
        on:input={() => dispatch('typing', { isTyping: draft.trim().length > 0 })}
      />
      <button class="btn-primary" type="submit" disabled={!connected}>Send</button>
    </form>
  </div>
</div>
