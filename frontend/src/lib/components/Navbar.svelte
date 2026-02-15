<script lang="ts">
  import { auth, clearSession } from '$lib/stores/auth';

  const links = [
    { href: '/', label: 'Home' },
    { href: '/chat', label: 'Chat' },
    { href: '/history', label: 'History' },
    { href: '/store', label: 'Store' },
    { href: '/settings', label: 'Settings' }
  ];
</script>

<nav class="sticky top-0 z-20 border-b border-white/10 bg-slate-950/70 backdrop-blur">
  <div class="mx-auto flex w-full max-w-6xl items-center justify-between px-4 py-3">
    <a href="/" class="text-lg font-bold text-white">Othergirl</a>

    <div class="hidden items-center gap-4 md:flex">
      {#each links as link}
        <a href={link.href} class="text-sm text-slate-300 transition hover:text-white">{link.label}</a>
      {/each}
    </div>

    {#if $auth.user}
      <div class="flex items-center gap-3">
        <span class="text-sm text-slate-200">{$auth.user.username}</span>
        <button class="btn-secondary" onclick={clearSession}>Log out</button>
      </div>
    {:else}
      <a href="/login" class="btn-primary">Login</a>
    {/if}
  </div>
</nav>
