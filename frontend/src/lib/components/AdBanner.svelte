<script lang="ts">
  let { visible = true }: { visible?: boolean } = $props();
</script>

{#if visible}
  <div class="surface rounded-xl p-3 text-center text-xs text-slate-300">
    Free tier ad slot (Phase 3 integration)
  </div>
{/if}
