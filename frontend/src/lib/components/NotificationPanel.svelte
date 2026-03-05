<script lang="ts">
  import { Bell, X } from 'lucide-svelte';
  import { apiFetch } from '$lib/api';
  import { auth } from '$lib/stores/auth';
  import { onMount } from 'svelte';

  type Notification = {
    id: string;
    notification_type: string;
    title: string;
    body: string;
    is_read: boolean;
    created_at: string;
  };

  let open = $state(false);
  let notifications: Notification[] = $state([]);
  let unreadCount = $state(0);
  let loading = $state(false);
  let panelEl: HTMLDivElement | undefined = $state();

  onMount(() => {
    if ($auth.token) {
      loadUnreadCount();
    }
  });

  async function loadUnreadCount() {
    try {
      const res = await apiFetch<{ count: number }>('/api/notifications/unread-count');
      unreadCount = res.count;
    } catch {
      // Silently fail — notifications are non-critical
    }
  }

  async function loadNotifications() {
    if (!$auth.token) return;
    loading = true;
    try {
      const res = await apiFetch<{ notifications: Notification[] }>('/api/notifications?limit=20');
      notifications = res.notifications ?? [];
      await loadUnreadCount();
    } catch {
      // Silently fail
    } finally {
      loading = false;
    }
  }

  async function markAllRead() {
    try {
      await apiFetch('/api/notifications/read-all', { method: 'POST' });
      notifications = notifications.map((n) => ({ ...n, is_read: true }));
      unreadCount = 0;
    } catch {
      // Silently fail
    }
  }

  function toggle() {
    open = !open;
    if (open) {
      loadNotifications();
    }
  }

  function onClickOutside(e: MouseEvent) {
    if (panelEl && !panelEl.contains(e.target as Node)) {
      open = false;
    }
  }

  $effect(() => {
    if (!open) return;
    window.addEventListener('mousedown', onClickOutside);
    return () => window.removeEventListener('mousedown', onClickOutside);
  });

  function timeAgo(dateStr: string): string {
    const diff = Date.now() - new Date(dateStr).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }
</script>

{#if $auth.user}
  <div class="relative" bind:this={panelEl}>
    <button
      class="relative flex h-9 w-9 items-center justify-center rounded-lg text-[var(--text-muted)] transition-colors hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)]"
      onclick={toggle}
      aria-label="Notifications"
    >
      <Bell size={18} />
      {#if unreadCount > 0}
        <span class="absolute -right-0.5 -top-0.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-red-500 px-1 text-[10px] font-bold text-white">
          {unreadCount > 99 ? '99+' : unreadCount}
        </span>
      {/if}
    </button>

    {#if open}
      <div class="absolute right-0 top-full z-30 mt-2 w-80 rounded-xl border border-[var(--border-default)] bg-[var(--bg-popover)] shadow-xl backdrop-blur">
        <div class="flex items-center justify-between border-b border-[var(--border-default)] px-4 py-3">
          <h3 class="text-sm font-semibold">Notifications</h3>
          <div class="flex gap-2">
            {#if unreadCount > 0}
              <button
                class="text-xs text-[var(--text-muted)] transition hover:text-[var(--text-primary)]"
                onclick={markAllRead}
              >Mark all read</button>
            {/if}
            <button
              class="text-[var(--text-muted)] transition hover:text-[var(--text-primary)]"
              onclick={() => (open = false)}
            >
              <X size={16} />
            </button>
          </div>
        </div>

        <div class="max-h-80 overflow-y-auto">
          {#if loading}
            <div class="px-4 py-8 text-center text-sm text-[var(--text-muted)]">Loading...</div>
          {:else if notifications.length === 0}
            <div class="px-4 py-8 text-center text-sm text-[var(--text-muted)]">No notifications yet</div>
          {:else}
            {#each notifications as notif (notif.id)}
              <div
                class="border-b border-[var(--border-default)] px-4 py-3 last:border-b-0 {notif.is_read ? '' : 'bg-[var(--bg-overlay)]'}"
              >
                <div class="flex items-start justify-between gap-2">
                  <p class="text-sm font-medium {notif.is_read ? 'text-[var(--text-secondary)]' : 'text-[var(--text-primary)]'}">
                    {notif.title}
                  </p>
                  {#if !notif.is_read}
                    <span class="mt-1 h-2 w-2 flex-shrink-0 rounded-full bg-blue-500"></span>
                  {/if}
                </div>
                {#if notif.body}
                  <p class="mt-0.5 text-xs text-[var(--text-muted)]">{notif.body}</p>
                {/if}
                <p class="mt-1 text-[10px] text-[var(--text-muted)]">{timeAgo(notif.created_at)}</p>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    {/if}
  </div>
{/if}
