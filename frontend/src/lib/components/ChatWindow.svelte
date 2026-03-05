<script lang="ts">
  import MessageBubble from '$lib/components/MessageBubble.svelte';
  import TypingIndicator from '$lib/components/TypingIndicator.svelte';
  import EmotePicker from '$lib/components/EmotePicker.svelte';
  import FlareDisplay from '$lib/components/FlareDisplay.svelte';
  import { Smile, Sparkles } from 'lucide-svelte';
  import { tick } from 'svelte';

  let {
    messages = [],
    currentUserId = null,
    partnerTyping = false,
    connected = false,
    connectionError = '',
    partnerName = 'stranger',
    partnerFlare = {},
    partnerAwards = [],
    onsend,
    onleave,
    onnext,
    ontyping,
    onkeepvote,
    onaward
  }: {
    messages?: { id: string; sender_id: string; content: string; timestamp: string; flagged?: boolean; status?: 'sent' | 'read' }[];
    currentUserId?: string | null;
    partnerTyping?: boolean;
    connected?: boolean;
    connectionError?: string;
    partnerName?: string;
    partnerFlare?: Record<string, any>;
    partnerAwards?: { award_type: string; spark_amount: number }[];
    onsend?: (detail: { content: string }) => void;
    onleave?: () => void;
    onnext?: () => void;
    ontyping?: (detail: { isTyping: boolean }) => void;
    onkeepvote?: (detail: { keep: boolean }) => void;
    onaward?: (detail: { award_type: string; spark_amount: number }) => void;
  } = $props();

  const MAX_SPARK_AMOUNT = 10000;

  let draft = $state('');
  let sparkAmount = $state(100);
  let emotePickerOpen = $state(false);
  let inputEl: HTMLInputElement | undefined = $state();
  let emoteBtn: HTMLButtonElement | undefined = $state();
  let messagesEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    // Subscribe to messages array changes
    messages;
    if (!messagesEl) return;
    const { scrollTop, scrollHeight, clientHeight } = messagesEl;
    const nearBottom = scrollHeight - scrollTop - clientHeight < 100;
    if (nearBottom) {
      tick().then(() => {
        messagesEl!.scrollTop = messagesEl!.scrollHeight;
      });
    }
  });

  async function insertEmote(token: string) {
    if (!inputEl) return;
    const start = inputEl.selectionStart ?? draft.length;
    const end = inputEl.selectionEnd ?? draft.length;
    draft = draft.slice(0, start) + token + draft.slice(end);
    const cursor = start + token.length;
    await tick();
    inputEl.focus();
    inputEl.setSelectionRange(cursor, cursor);
  }

  function submitMessage() {
    const content = draft.trim();
    if (!content || !connected) return;

    onsend?.({ content });
    draft = '';
    ontyping?.({ isTyping: false });
  }

  function sendAward() {
    if (!connected || sparkAmount <= 0) return;
    if (sparkAmount > MAX_SPARK_AMOUNT) {
      sparkAmount = MAX_SPARK_AMOUNT;
      return;
    }

    onaward?.({
      award_type: 'spark',
      spark_amount: sparkAmount
    });
  }
</script>

<div class="surface flex h-[calc(100dvh-theme(spacing.20))] flex-col">
  <div class="flex items-center justify-between border-b border-[var(--border-default)] px-4 py-3">
    <div>
      <h2 class="font-semibold">Active Chat</h2>
      <div class="flex items-center gap-2">
        <p class="text-xs text-[var(--text-muted)]">with {partnerName}</p>
        <FlareDisplay flare={partnerFlare} awards={partnerAwards} compact />
      </div>
    </div>
    <div class="flex items-center gap-2">
      <button class="btn-secondary" onclick={() => onkeepvote?.({ keep: true })} disabled={!connected}>Keep</button>
      <button class="btn-secondary" onclick={() => onkeepvote?.({ keep: false })} disabled={!connected}>Skip Keep</button>
      <button class="btn-secondary" onclick={() => onleave?.()}>Leave</button>
      <button class="btn-primary" onclick={() => onnext?.()}>Next</button>
    </div>
  </div>

  <div bind:this={messagesEl} class="flex-1 space-y-3 overflow-y-auto px-4 py-4">
    {#if connectionError}
      <div class="rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-3 text-center text-sm text-red-300">
        {connectionError}
      </div>
    {:else if !connected && messages.length === 0}
      <div class="flex flex-1 flex-col items-center justify-center py-12 text-center">
        <Sparkles size={48} class="mb-4 text-[var(--text-muted)]" />
        <h2 class="text-lg font-semibold">Ready to chat?</h2>
        <p class="mt-1 text-sm text-[var(--text-muted)]">Find someone to talk to</p>
      </div>
    {:else if messages.length === 0 && !partnerTyping}
      <p class="text-sm text-[var(--text-muted)]">No messages yet. Say hi.</p>
    {/if}

    {#each messages as message (message.id)}
      <MessageBubble {message} mine={message.sender_id === currentUserId} />
    {/each}
  </div>

  <div class="border-t border-[var(--border-default)] px-4 py-3">
    <TypingIndicator isTyping={partnerTyping} />

    <div class="mt-2 flex items-center gap-2">
      <input class="input max-w-36" type="number" min="1" max={MAX_SPARK_AMOUNT} bind:value={sparkAmount} disabled={!connected} />
      <button class="btn-secondary" onclick={sendAward} disabled={!connected}>Send Award</button>
    </div>

    <form
      class="mt-2 flex items-center gap-2"
      onsubmit={(e) => { e.preventDefault(); submitMessage(); }}
    >
      <div class="relative">
        <button
          bind:this={emoteBtn}
          type="button"
          class="flex h-9 w-9 items-center justify-center rounded-lg text-[var(--text-muted)] transition-colors hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)]"
          onclick={() => (emotePickerOpen = !emotePickerOpen)}
          disabled={!connected}
        >
          <Smile size={20} />
        </button>
        <EmotePicker bind:open={emotePickerOpen} onselect={insertEmote} anchorEl={emoteBtn} />
      </div>
      <input
        bind:this={inputEl}
        class="input"
        placeholder={connected ? 'Type a message...' : 'Connect to a chat first'}
        bind:value={draft}
        disabled={!connected}
        oninput={() => ontyping?.({ isTyping: draft.trim().length > 0 })}
      />
      <button class="btn-primary" type="submit" disabled={!connected}>Send</button>
    </form>
  </div>
</div>
