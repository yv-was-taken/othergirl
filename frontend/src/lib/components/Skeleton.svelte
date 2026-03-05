<script lang="ts">
  type Props = {
    variant?: 'text' | 'circle' | 'rect';
    width?: string;
    height?: string;
    count?: number;
  };

  let { variant = 'text', width, height, count = 1 }: Props = $props();
</script>

{#each Array(count) as _}
  <div
    class="skeleton"
    class:skeleton-text={variant === 'text'}
    class:skeleton-circle={variant === 'circle'}
    class:skeleton-rect={variant === 'rect'}
    style:width={width}
    style:height={height}
  ></div>
{/each}

<style>
  .skeleton {
    background: var(--bg-elevated);
    position: relative;
    overflow: hidden;
  }

  .skeleton::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent 0%,
      var(--bg-overlay) 50%,
      transparent 100%
    );
    animation: shimmer 1.5s infinite;
  }

  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }

  .skeleton-text {
    height: 0.875rem;
    border-radius: 0.5rem;
    width: 100%;
  }

  .skeleton-text + .skeleton-text {
    margin-top: 0.625rem;
  }

  .skeleton-text:last-child {
    width: 60%;
  }

  .skeleton-circle {
    border-radius: 9999px;
    flex-shrink: 0;
  }

  .skeleton-rect {
    border-radius: 0.75rem;
  }
</style>
