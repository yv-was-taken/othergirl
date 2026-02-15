<script lang="ts">
  let {
    open = false,
    onvote,
    onclose
  }: {
    open?: boolean;
    onvote?: (detail: { keep: boolean }) => void;
    onclose?: () => void;
  } = $props();
</script>

{#if open}
  <div class="fixed inset-0 z-30 flex items-center justify-center bg-[var(--bg-backdrop)] p-4">
    <div class="surface w-full max-w-md space-y-4 p-5">
      <h3 class="text-lg font-semibold">Keep this chat?</h3>
      <p class="text-sm text-[var(--text-secondary)]">If both people choose keep, it will be saved in history.</p>
      <div class="flex gap-2">
        <button class="btn-primary flex-1" onclick={() => onvote?.({ keep: true })}>Keep</button>
        <button class="btn-secondary flex-1" onclick={() => onvote?.({ keep: false })}>Skip</button>
      </div>
      <button class="btn-secondary w-full" onclick={() => onclose?.()}>Close</button>
    </div>
  </div>
{/if}
