<script lang="ts">
  import { applyEmotes } from '$lib/utils/emotes';
  import { renderMarkdown } from '$lib/utils/markdown';

  export let message: {
    content: string;
    sender_id: string;
    timestamp: string;
    flagged?: boolean;
  };
  export let mine = false;

  $: rendered = applyEmotes(renderMarkdown(message.content));
</script>

<div class={`max-w-[85%] rounded-2xl px-4 py-2 text-sm shadow ${mine ? 'ml-auto bg-brand-400 text-white' : 'bg-white/10 text-slate-100'}`}>
  <div class="prose prose-sm prose-invert max-w-none leading-relaxed">{@html rendered}</div>
  <div class={`mt-1 text-[11px] ${mine ? 'text-white/80' : 'text-slate-400'}`}>
    {new Date(message.timestamp).toLocaleTimeString()}
    {#if message.flagged}
      <span class="ml-2 rounded bg-amber-300/20 px-1 py-0.5 text-amber-200">flagged</span>
    {/if}
  </div>
</div>
