<script lang="ts">
  import { auth, clearSession } from '$lib/stores/auth';
  import { theme, toggleTheme } from '$lib/stores/theme';
  import { Sun, Moon } from 'lucide-svelte';

  const links = [
    { href: '/', label: 'Home' },
    { href: '/chat', label: 'Chat' },
    { href: '/history', label: 'History' },
    { href: '/store', label: 'Store' },
    { href: '/settings', label: 'Settings' }
  ];
</script>

<nav class="sticky top-0 z-20 border-b border-[var(--border-default)] bg-[var(--navbar-bg)] backdrop-blur">
  <div class="mx-auto flex w-full max-w-6xl items-center justify-between px-4 py-3">
    <div class="hidden items-center gap-6 md:flex">
      {#each links as link}
        <a href={link.href} class="text-sm text-[var(--text-secondary)] transition hover:text-[var(--text-primary)]">{link.label}</a>
      {/each}
    </div>

    <div class="flex items-center gap-3">
      <button
        class="flex h-9 w-9 items-center justify-center rounded-lg text-[var(--text-muted)] transition-colors hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)]"
        onclick={toggleTheme}
        aria-label="Toggle theme"
      >
        {#if $theme === 'dark'}
          <Sun size={18} />
        {:else}
          <Moon size={18} />
        {/if}
      </button>

      {#if $auth.user}
        <span class="text-sm text-[var(--text-secondary)]">{$auth.user.username}</span>
        <button class="btn-secondary" onclick={clearSession}>Log out</button>
      {:else}
        <a href="/login" class="btn-primary">Login</a>
      {/if}
    </div>
  </div>
</nav>
