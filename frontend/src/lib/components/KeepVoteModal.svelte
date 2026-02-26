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

  let dialogEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (!open || !dialogEl) return;

    // Focus the first button on mount
    const focusable = dialogEl.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    if (focusable.length > 0) {
      focusable[0].focus();
    }

    // Trap focus and handle Escape
    function handleKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        onclose?.();
        return;
      }

      if (e.key === 'Tab' && dialogEl) {
        const focusableEls = dialogEl.querySelectorAll<HTMLElement>(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        );
        if (focusableEls.length === 0) return;

        const first = focusableEls[0];
        const last = focusableEls[focusableEls.length - 1];

        if (e.shiftKey) {
          if (document.activeElement === first) {
            e.preventDefault();
            last.focus();
          }
        } else {
          if (document.activeElement === last) {
            e.preventDefault();
            first.focus();
          }
        }
      }
    }

    document.addEventListener('keydown', handleKeydown);
    return () => document.removeEventListener('keydown', handleKeydown);
  });
</script>

{#if open}
  <div class="fixed inset-0 z-30 flex items-center justify-center bg-[var(--bg-backdrop)] p-4">
    <div
      bind:this={dialogEl}
      role="dialog"
      aria-modal="true"
      aria-labelledby="keepvote-title"
      class="surface w-full max-w-md space-y-4 p-5"
    >
      <h3 id="keepvote-title" class="text-lg font-semibold">Keep this chat?</h3>
      <p class="text-sm text-[var(--text-secondary)]">If both people choose keep, it will be saved in history.</p>
      <div class="flex gap-2">
        <button class="btn-primary flex-1" onclick={() => onvote?.({ keep: true })}>Keep</button>
        <button class="btn-secondary flex-1" onclick={() => onvote?.({ keep: false })}>Skip</button>
      </div>
      <button class="btn-secondary w-full" onclick={() => onclose?.()}>Close</button>
    </div>
  </div>
{/if}
