<script lang="ts">
  import { applyEmotes } from '$lib/utils/emotes';
  import { renderMarkdown } from '$lib/utils/markdown';

  let { message, mine = false }: {
    message: {
      content: string;
      sender_id: string;
      timestamp: string;
      flagged?: boolean;
    };
    mine?: boolean;
  } = $props();

  const rendered = $derived(renderMarkdown(applyEmotes(message.content)));
</script>

<div class={`max-w-[85%] rounded-2xl px-4 py-2 text-sm shadow ${mine ? 'ml-auto bg-[var(--bubble-mine)]' : 'bg-[var(--bubble-theirs)]'}`}>
  <div class="prose prose-sm max-w-none leading-relaxed">{@html rendered}</div>
  <div class={`mt-1 text-[11px] ${mine ? 'text-[var(--text-tertiary)]' : 'text-[var(--text-muted)]'}`}>
    {new Date(message.timestamp).toLocaleTimeString()}
    {#if message.flagged}
      <span class="ml-2 rounded bg-amber-300/20 px-1 py-0.5 text-amber-200">flagged</span>
    {/if}
  </div>
</div>
