<script lang="ts">
  let { isTyping = false }: { isTyping?: boolean } = $props();

  let visible = $state(false);
  let timeout: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (isTyping) {
      visible = true;
      if (timeout) clearTimeout(timeout);
      timeout = setTimeout(() => {
        visible = false;
      }, 3000);
    } else {
      visible = false;
      if (timeout) {
        clearTimeout(timeout);
        timeout = null;
      }
    }

    return () => {
      if (timeout) clearTimeout(timeout);
    };
  });
</script>

{#if visible}
  <div class="typing-indicator">
    <span class="dot"></span>
    <span class="dot"></span>
    <span class="dot"></span>
  </div>
{/if}

<style>
  .typing-indicator {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 12px;
    background: var(--bg-elevated);
    border-radius: 12px;
    width: fit-content;
  }

  .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--text-muted);
    animation: bounce 1.2s ease-in-out infinite;
  }

  .dot:nth-child(2) {
    animation-delay: 0.15s;
  }

  .dot:nth-child(3) {
    animation-delay: 0.3s;
  }

  @keyframes bounce {
    0%, 60%, 100% {
      transform: translateY(0);
      opacity: 0.4;
    }
    30% {
      transform: translateY(-4px);
      opacity: 1;
    }
  }
</style>
