<script lang="ts">
  import { page } from '$app/stores';
  import { auth, clearSession } from '$lib/stores/auth';
  import { theme, toggleTheme } from '$lib/stores/theme';
  import { Sun, Moon } from 'lucide-svelte';
  import NotificationPanel from '$lib/components/NotificationPanel.svelte';
  import Avatar from '$lib/components/Avatar.svelte';

  let mobileMenuOpen = $state(false);

  function isActive(href: string, pathname: string): boolean {
    if (href === '/') return pathname === '/';
    return pathname === href || pathname.startsWith(href + '/');
  }

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
    <!-- Hamburger button: visible on mobile only -->
    <button
      class="flex h-9 w-9 flex-col items-center justify-center gap-1 rounded-lg text-[var(--text-muted)] transition-colors hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)] md:hidden"
      onclick={() => mobileMenuOpen = !mobileMenuOpen}
      aria-label="Toggle navigation menu"
      aria-expanded={mobileMenuOpen}
    >
      <span class="block h-0.5 w-5 rounded bg-current"></span>
      <span class="block h-0.5 w-5 rounded bg-current"></span>
      <span class="block h-0.5 w-5 rounded bg-current"></span>
    </button>

    <!-- Desktop nav links -->
    <div class="hidden items-center gap-6 md:flex">
      {#each links as link (link.href)}
        <a
          href={link.href}
          class="text-sm transition hover:text-[var(--text-primary)] {isActive(link.href, $page.url.pathname) ? 'border-b-2 border-[var(--accent)] text-[var(--accent)]' : 'text-[var(--text-secondary)]'}"
        >{link.label}</a>
      {/each}
    </div>

    <div class="flex items-center gap-2">
      <NotificationPanel />
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
        <a href="/profile" class="flex items-center gap-2">
          <Avatar url={$auth.user.avatar_url} username={$auth.user.username} size={32} />
          <span class="text-sm text-[var(--text-secondary)]">{$auth.user.username}</span>
        </a>
        <button class="btn-secondary" onclick={clearSession}>Log out</button>
      {:else}
        <a href="/login" class="btn-primary">Login</a>
      {/if}
    </div>
  </div>

  <!-- Mobile dropdown menu -->
  {#if mobileMenuOpen}
    <div class="flex flex-col border-t border-[var(--border-default)] px-4 py-2 md:hidden">
      {#each links as link (link.href)}
        <a
          href={link.href}
          class="py-2 text-sm transition hover:text-[var(--text-primary)] {isActive(link.href, $page.url.pathname) ? 'border-l-2 border-[var(--accent)] pl-2 text-[var(--accent)]' : 'text-[var(--text-secondary)]'}"
          onclick={() => mobileMenuOpen = false}
        >{link.label}</a>
      {/each}
    </div>
  {/if}
</nav>
