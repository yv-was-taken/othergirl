<script lang="ts">
  import { applyEmotes } from '$lib/utils/emotes';
  import { renderMarkdown } from '$lib/utils/markdown';
  import dayjs from 'dayjs';
  import relativeTime from 'dayjs/plugin/relativeTime';
  import { Check, CheckCheck } from 'lucide-svelte';

  dayjs.extend(relativeTime);

  let { message, mine = false, nowMs = Date.now() }: {
    message: {
      content: string;
      sender_id: string;
      timestamp: string;
      flagged?: boolean;
      is_read?: boolean;
    };
    mine?: boolean;
    nowMs?: number;
  } = $props();

  const rendered = $derived(renderMarkdown(applyEmotes(message.content)));

  const relativeTimestamp = $derived.by(() => {
    const ts = dayjs(message.timestamp);
    const now = dayjs(nowMs);
    const diffSec = now.diff(ts, 'second');
    const diffMin = now.diff(ts, 'minute');
    const diffHour = now.diff(ts, 'hour');

    if (diffSec < 60) return 'just now';
    if (diffMin < 60) return `${diffMin}m ago`;
    if (diffHour < 24) return `${diffHour}h ago`;
    return ts.format('MMM D, YYYY');
  });

  const absoluteTimestamp = $derived(dayjs(message.timestamp).format('ddd, MMM D, YYYY h:mm:ss A'));
</script>

<div class={`max-w-[85%] rounded-2xl px-4 py-2 text-sm shadow ${mine ? 'ml-auto bg-[var(--bubble-mine)]' : 'bg-[var(--bubble-theirs)]'}`}>
  <div class="prose prose-sm max-w-none leading-relaxed">{@html rendered}</div>
  <div class={`mt-1 flex items-center gap-1 text-[11px] ${mine ? 'text-[var(--text-tertiary)]' : 'text-[var(--text-muted)]'}`}>
    <span title={absoluteTimestamp}>{relativeTimestamp}</span>
    {#if mine}
      {#if message.is_read}
        <CheckCheck size={14} />
      {:else}
        <Check size={14} />
      {/if}
    {/if}
    {#if message.flagged}
      <span class="ml-2 rounded bg-amber-300/20 px-1 py-0.5 text-amber-200">flagged</span>
    {/if}
  </div>
</div>
