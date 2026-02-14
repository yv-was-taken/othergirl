<script lang="ts">
  let { isTyping = false }: { isTyping?: boolean } = $props();
</script>

{#if isTyping}
  <p class="text-xs text-slate-400">Partner is typing...</p>
{/if}
