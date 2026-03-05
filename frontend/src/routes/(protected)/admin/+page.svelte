<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';

  import { ShieldCheck, Users, MessageCircle, Sparkles } from 'lucide-svelte';
  import Skeleton from '$lib/components/Skeleton.svelte';
  import { apiFetch } from '$lib/api';
  import { auth } from '$lib/stores/auth';

  type Stats = {
    total_users: number;
    active_chats: number;
    total_messages: number;
    total_awards: number;
  };

  type User = {
    id: string;
    username: string;
    email: string;
    is_premium: boolean;
    is_suspended: boolean;
    created_at: string;
  };

  type UsersResponse = {
    users: User[];
    page: number;
    per_page: number;
  };

  let stats: Stats | null = $state(null);
  let users: User[] = $state([]);
  let page = $state(1);
  let perPage = $state(50);
  let loading = $state(false);
  let actionLoading: string | null = $state(null);

  function getStatCards(s: Stats) {
    return [
      { label: 'Total Users', value: s.total_users, icon: Users },
      { label: 'Active Chats', value: s.active_chats, icon: MessageCircle },
      { label: 'Total Messages', value: s.total_messages, icon: MessageCircle },
      { label: 'Total Awards', value: s.total_awards, icon: Sparkles }
    ];
  }

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    if (!$auth.token) return;

    loading = true;
    try {
      const [statsRes, usersRes] = await Promise.all([
        apiFetch<Stats>('/api/admin/stats'),
        apiFetch<UsersResponse>(`/api/admin/users?page=${page}&per_page=${perPage}`)
      ]);

      stats = statsRes;
      users = usersRes.users;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load admin data');
    } finally {
      loading = false;
    }
  }

  async function toggleSuspend(user: User) {
    actionLoading = user.id;
    const endpoint = user.is_suspended
      ? `/api/admin/users/${user.id}/unsuspend`
      : `/api/admin/users/${user.id}/suspend`;

    try {
      await apiFetch<{ message: string; user_id: string }>(endpoint, { method: 'POST' });
      toast.success(user.is_suspended ? 'User unsuspended' : 'User suspended');
      await refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Action failed');
    } finally {
      actionLoading = null;
    }
  }

  async function goToPage(newPage: number) {
    page = newPage;
    await refresh();
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    });
  }
</script>

<svelte:head><title>Admin - othergirl</title></svelte:head>

<section class="space-y-6">
  <div class="flex items-center gap-3">
    <ShieldCheck size={28} class="text-[var(--accent)]" />
    <h1 class="text-2xl font-bold">Admin Dashboard</h1>
  </div>

  {#if loading && !stats}
    <!-- Stats skeleton -->
    <div class="grid grid-cols-2 gap-3 md:grid-cols-4">
      {#each Array(4) as _}
        <div class="surface space-y-3 p-4">
          <Skeleton variant="text" width="60%" />
          <Skeleton variant="text" width="40%" height="1.5rem" />
        </div>
      {/each}
    </div>

    <!-- Table skeleton -->
    <div class="surface space-y-3 p-4">
      <Skeleton variant="text" width="30%" />
      {#each Array(5) as _}
        <Skeleton variant="rect" width="100%" height="2.5rem" />
      {/each}
    </div>
  {:else if stats}
    <!-- Stats cards -->
    <div class="grid grid-cols-2 gap-3 md:grid-cols-4">
      {#each getStatCards(stats) as card}
        <div class="surface flex items-center gap-3 p-4">
          <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-[var(--bg-overlay)]">
            <card.icon size={20} class="text-[var(--accent)]" />
          </div>
          <div>
            <p class="text-xs text-[var(--text-muted)]">{card.label}</p>
            <p class="text-xl font-bold">{card.value.toLocaleString()}</p>
          </div>
        </div>
      {/each}
    </div>

    <!-- User management table -->
    <div class="surface overflow-x-auto p-4">
      <h2 class="mb-4 text-lg font-semibold">User Management</h2>

      {#if users.length === 0}
        <p class="text-sm text-[var(--text-muted)]">No users found.</p>
      {:else}
        <table class="w-full text-left text-sm">
          <thead>
            <tr class="border-b border-[var(--border-default)] text-[var(--text-muted)]">
              <th class="pb-2 pr-4 font-medium">Username</th>
              <th class="pb-2 pr-4 font-medium">Email</th>
              <th class="pb-2 pr-4 font-medium">Premium</th>
              <th class="pb-2 pr-4 font-medium">Suspended</th>
              <th class="pb-2 pr-4 font-medium">Created</th>
              <th class="pb-2 font-medium">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each users as user}
              <tr class="border-b border-[var(--border-default)] last:border-0">
                <td class="py-3 pr-4 font-medium">{user.username}</td>
                <td class="py-3 pr-4 text-[var(--text-secondary)]">{user.email}</td>
                <td class="py-3 pr-4">
                  {#if user.is_premium}
                    <span class="inline-block rounded-full bg-[var(--accent)] px-2 py-0.5 text-xs font-medium text-white" style="opacity: 0.9;">
                      Yes
                    </span>
                  {:else}
                    <span class="inline-block rounded-full bg-[var(--bg-overlay)] px-2 py-0.5 text-xs font-medium text-[var(--text-muted)]">
                      No
                    </span>
                  {/if}
                </td>
                <td class="py-3 pr-4">
                  {#if user.is_suspended}
                    <span class="inline-block rounded-full bg-red-500 px-2 py-0.5 text-xs font-medium text-white" style="opacity: 0.9;">
                      Yes
                    </span>
                  {:else}
                    <span class="inline-block rounded-full bg-[var(--bg-overlay)] px-2 py-0.5 text-xs font-medium text-[var(--text-muted)]">
                      No
                    </span>
                  {/if}
                </td>
                <td class="py-3 pr-4 text-[var(--text-muted)]">{formatDate(user.created_at)}</td>
                <td class="py-3">
                  <button
                    class={user.is_suspended ? 'btn-primary' : 'btn-secondary'}
                    disabled={actionLoading === user.id}
                    onclick={() => toggleSuspend(user)}
                  >
                    {#if actionLoading === user.id}
                      ...
                    {:else}
                      {user.is_suspended ? 'Unsuspend' : 'Suspend'}
                    {/if}
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>

        <!-- Pagination -->
        <div class="mt-4 flex items-center justify-between">
          <p class="text-sm text-[var(--text-muted)]">Page {page}</p>
          <div class="flex gap-2">
            <button
              class="btn-secondary"
              disabled={page <= 1}
              onclick={() => goToPage(page - 1)}
            >
              Previous
            </button>
            <button
              class="btn-secondary"
              disabled={users.length < perPage}
              onclick={() => goToPage(page + 1)}
            >
              Next
            </button>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</section>
