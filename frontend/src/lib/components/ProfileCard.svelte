<script lang="ts">
  let {
    username = 'stranger',
    bio = '',
    badges = [],
    interests = [],
    flare = {}
  }: {
    username?: string;
    bio?: string;
    badges?: string[];
    interests?: string[];
    flare?: Record<string, unknown>;
  } = $props();
</script>

<div class="surface space-y-2 p-4">
  <h3 class="text-lg font-semibold">{username}</h3>
  <p class="text-sm text-[var(--text-secondary)]">{bio || 'No bio yet.'}</p>
  <div class="flex flex-wrap gap-2">
    {#each badges as badge}
      <span class="rounded-full bg-[var(--bg-elevated)] px-2 py-1 text-xs text-[var(--text-secondary)]">{badge}</span>
    {/each}
  </div>

  {#if interests.length > 0}
    <div class="flex flex-wrap gap-2">
      {#each interests as interest}
        <span class="rounded-full bg-[var(--bg-elevated)] px-2 py-1 text-xs text-[var(--text-secondary)]">{interest}</span>
      {/each}
    </div>
  {/if}

  {#if Object.keys(flare).length > 0}
    <pre class="overflow-x-auto rounded-xl bg-[var(--bg-elevated)] p-2 text-xs text-[var(--text-secondary)]">{JSON.stringify(flare, null, 2)}</pre>
  {/if}
</div>
