<script lang="ts">
  let { isTyping = false }: { isTyping?: boolean } = $props();
</script>

{#if isTyping}
  <p class="text-xs text-[var(--text-muted)]">Partner is typing...</p>
{/if}
