<script lang="ts">
  let { url = null, username, size = 40 }: { url?: string | null; username: string; size?: number } = $props();

  const palette = [
    '#e74c3c',
    '#3498db',
    '#2ecc71',
    '#9b59b6',
    '#e67e22',
    '#1abc9c',
    '#f39c12',
    '#6c5ce7'
  ];

  let initials = $derived(username.slice(0, 2).toUpperCase());

  let bgColor = $derived(() => {
    let hash = 0;
    for (let i = 0; i < username.length; i++) {
      hash = username.charCodeAt(i) + ((hash << 5) - hash);
    }
    return palette[Math.abs(hash) % palette.length];
  });
</script>

{#if url}
  <img
    src={url}
    alt="{username}'s avatar"
    width={size}
    height={size}
    loading="lazy"
    class="avatar-img"
    style="--avatar-size: {size}px;"
  />
{:else}
  <span
    class="avatar-fallback"
    style="--avatar-size: {size}px; --avatar-bg: {bgColor()};"
    aria-label="{username}'s avatar"
  >
    {initials}
  </span>
{/if}

<style>
  .avatar-img {
    width: var(--avatar-size);
    height: var(--avatar-size);
    border-radius: 9999px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .avatar-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--avatar-size);
    height: var(--avatar-size);
    border-radius: 9999px;
    background: var(--avatar-bg);
    color: #fff;
    font-weight: 600;
    font-size: calc(var(--avatar-size) * 0.4);
    line-height: 1;
    flex-shrink: 0;
    user-select: none;
  }
</style>
