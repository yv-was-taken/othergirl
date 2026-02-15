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
  <p class="text-sm text-slate-300">{bio || 'No bio yet.'}</p>
  <div class="flex flex-wrap gap-2">
    {#each badges as badge}
      <span class="rounded-full bg-white/10 px-2 py-1 text-xs text-slate-200">{badge}</span>
    {/each}
  </div>

  {#if interests.length > 0}
    <div class="flex flex-wrap gap-2">
      {#each interests as interest}
        <span class="rounded-full bg-white/10 px-2 py-1 text-xs text-slate-200">{interest}</span>
      {/each}
    </div>
  {/if}

  {#if Object.keys(flare).length > 0}
    <pre class="overflow-x-auto rounded-xl bg-black/20 p-2 text-xs text-slate-300">{JSON.stringify(flare, null, 2)}</pre>
  {/if}
</div>
