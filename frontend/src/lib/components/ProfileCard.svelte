<script lang="ts">
  import FlareDisplay from '$lib/components/FlareDisplay.svelte';

  let {
    username = 'stranger',
    bio = '',
    badges = [],
    interests = [],
    flare = {},
    awards = []
  }: {
    username?: string;
    bio?: string;
    badges?: string[];
    interests?: string[];
    flare?: Record<string, any>;
    awards?: { award_type: string; spark_amount: number }[];
  } = $props();
</script>

<div class="surface space-y-2 p-4">
  <h3 class="text-lg font-semibold">{username}</h3>
  <p class="text-sm text-[var(--text-secondary)]">{bio || 'No bio yet.'}</p>
  <div class="flex flex-wrap gap-2">
    {#each badges as badge (badge)}
      <span class="rounded-full bg-[var(--bg-elevated)] px-2 py-1 text-xs text-[var(--text-secondary)]">{badge}</span>
    {/each}
  </div>

  {#if interests.length > 0}
    <div class="flex flex-wrap gap-2">
      {#each interests as interest (interest)}
        <span class="rounded-full bg-[var(--bg-elevated)] px-2 py-1 text-xs text-[var(--text-secondary)]">{interest}</span>
      {/each}
    </div>
  {/if}

  <FlareDisplay {flare} {awards} />
</div>
