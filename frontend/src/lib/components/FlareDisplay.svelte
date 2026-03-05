<script lang="ts">
  import { Sparkles, Palette, MessageSquare } from 'lucide-svelte';

  let {
    flare = {},
    awards = [],
    compact = false
  }: {
    flare?: Record<string, any>;
    awards?: { award_type: string; spark_amount: number }[];
    compact?: boolean;
  } = $props();

  let flareEntries = $derived(Object.entries(flare));
  let hasContent = $derived(flareEntries.length > 0 || awards.length > 0);
</script>

{#if hasContent}
  <div class={`flex flex-wrap items-center gap-1.5 ${compact ? 'gap-1' : 'gap-1.5'}`}>
    {#each flareEntries as [type, value]}
      {#if type === 'name_color'}
        <span
          class={`inline-flex items-center gap-1 rounded-full border border-[var(--border-default)] bg-[var(--bg-elevated)] ${compact ? 'px-1.5 py-0.5 text-[0.625rem]' : 'px-2 py-0.5 text-xs'} text-[var(--text-muted)]`}
        >
          <span
            class={`inline-block rounded-full ${compact ? 'h-2 w-2' : 'h-2.5 w-2.5'}`}
            style={`background-color: ${(value as any)?.color ?? '#888'}`}
          ></span>
          {#if !compact}Name color{/if}
        </span>
      {:else if type === 'bubble_theme'}
        <span
          class={`inline-flex items-center gap-1 rounded-full border border-[var(--border-default)] bg-[var(--bg-elevated)] ${compact ? 'px-1.5 py-0.5 text-[0.625rem]' : 'px-2 py-0.5 text-xs'} text-[var(--text-muted)]`}
        >
          <MessageSquare size={compact ? 10 : 12} />
          {(value as any)?.theme ?? 'custom'}
        </span>
      {:else}
        <span
          class={`inline-flex items-center gap-1 rounded-full border border-[var(--border-default)] bg-[var(--bg-elevated)] ${compact ? 'px-1.5 py-0.5 text-[0.625rem]' : 'px-2 py-0.5 text-xs'} text-[var(--text-muted)]`}
        >
          <Palette size={compact ? 10 : 12} />
          {type.replace(/_/g, ' ')}
        </span>
      {/if}
    {/each}

    {#each awards as award}
      <span
        class={`inline-flex items-center gap-1 rounded-full border border-[var(--border-default)] bg-[var(--bg-elevated)] ${compact ? 'px-1.5 py-0.5 text-[0.625rem]' : 'px-2 py-0.5 text-xs'} text-[var(--text-muted)]`}
      >
        <Sparkles size={compact ? 10 : 12} />
        {award.spark_amount}
      </span>
    {/each}
  </div>
{/if}
