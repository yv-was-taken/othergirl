<script lang="ts">
  export let isTyping = false;
</script>

{#if isTyping}
  <p class="text-xs text-slate-400">Partner is typing...</p>
{/if}
