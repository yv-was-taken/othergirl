<script lang="ts">
  let {
    username = 'othergirl',
    color = '#a8a8b3',
    bubbleTheme = 'default'
  }: {
    username?: string;
    color?: string;
    bubbleTheme?: string;
  } = $props();
</script>

<div class="surface p-4">
  <p class="mb-2 text-xs uppercase tracking-wide text-[var(--text-muted)]">Flare preview</p>
  <p class="text-sm" style={`color:${color}`}>{username}</p>
  <div class={`mt-2 rounded-xl p-3 text-sm ${bubbleTheme === 'midnight' ? 'bg-[var(--bg-elevated)]' : 'bg-[var(--bg-overlay)]'}`}>
    Your message bubble theme preview.
  </div>
</div>
