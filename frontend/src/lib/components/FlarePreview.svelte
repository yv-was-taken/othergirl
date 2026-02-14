<script lang="ts">
  let {
    username = 'othergirl',
    color = '#ff6b35',
    bubbleTheme = 'default'
  }: {
    username?: string;
    color?: string;
    bubbleTheme?: string;
  } = $props();
</script>

<div class="surface p-4">
  <p class="mb-2 text-xs uppercase tracking-wide text-slate-400">Flare preview</p>
  <p class="text-sm" style={`color:${color}`}>{username}</p>
  <div class={`mt-2 rounded-xl p-3 text-sm ${bubbleTheme === 'midnight' ? 'bg-slate-900 text-slate-100' : 'bg-white/10 text-slate-100'}`}>
    Your message bubble theme preview.
  </div>
</div>
