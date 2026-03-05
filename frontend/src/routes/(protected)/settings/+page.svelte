<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';

  import { apiFetch } from '$lib/api';
  import { auth, setSession } from '$lib/stores/auth';
  import type { UserResponse } from '$lib/types';
  import { Eye, EyeOff } from 'lucide-svelte';

  let balance = $state(0);
  let transactions: { id: string; amount: number; transaction_type: string; created_at: string }[] = $state([]);
  let connectStatus: { stripe_account_id: string; payouts_enabled: boolean } | null = $state(null);
  let blockedUsers: { id: string; username: string; created_at: string }[] = $state([]);
  let cashoutAmount = $state(1000);
  let sparkBundles: { index: number; sparks: number; price_cents: number }[] = $state([]);
  let selectedBundleIndex = $state(0);
  let emoteToken = $state(':my_emote:');
  let emoteName = $state('My Emote');
  let emotePrice = $state(100);
  let emoteFile: File | null = $state(null);
  let currentPassword = $state('');
  let newPassword = $state('');
  let confirmPassword = $state('');
  let showCurrentPw = $state(false);
  let showNewPw = $state(false);
  let changingPassword = $state(false);
  let deletePassword = $state('');
  let deleteConfirmed = $state(false);
  let deletionScheduledAt: string | null = $state(null);
  let deletingAccount = $state(false);
  let cancellingDeletion = $state(false);
  let activeTab = $state<'account' | 'subscription' | 'privacy'>('account');

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    if (!$auth.token) return;

    try {
      const [meRes, balanceRes, txRes, cashoutRes, blocksRes, bundlesRes] = await Promise.all([
        apiFetch<UserResponse>('/api/users/me'),
        apiFetch<{ balance: number }>('/api/sparks/balance'),
        apiFetch<{ transactions: { id: string; amount: number; transaction_type: string; created_at: string }[] }>('/api/sparks/transactions'),
        apiFetch<{ connect: { stripe_account_id: string; payouts_enabled: boolean } | null }>('/api/cashout/status'),
        apiFetch<{ blocked_users: { id: string; username: string; created_at: string }[] }>('/api/blocks'),
        apiFetch<{ bundles: { index: number; sparks: number; price_cents: number }[] }>('/api/payments/spark-bundles')
      ]);

      balance = balanceRes.balance;
      transactions = txRes.transactions ?? [];
      connectStatus = cashoutRes.connect;
      blockedUsers = blocksRes.blocked_users ?? [];
      sparkBundles = bundlesRes.bundles ?? [];
      deletionScheduledAt = meRes.deletion_scheduled_at ?? null;

      if ($auth.token) {
        setSession($auth.token, {
          id: meRes.id,
          username: meRes.username,
          email: meRes.email ?? null,
          is_premium: meRes.is_premium,
          is_age_verified: meRes.is_age_verified,
          created_at: meRes.created_at
        });
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load settings');
    }
  }

  async function subscribe() {
    try {
      const response = await apiFetch<{ checkout_url: string }>('/api/payments/subscribe', {
        method: 'POST',
        body: JSON.stringify({})
      });
      window.location.assign(response.checkout_url);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Subscription failed');
    }
  }

  async function buySparks(bundleIndex: number) {
    try {
      const response = await apiFetch<{ checkout_url: string }>('/api/payments/buy-sparks', {
        method: 'POST',
        body: JSON.stringify({ bundle_index: bundleIndex })
      });
      window.location.assign(response.checkout_url);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Spark purchase failed');
    }
  }

  async function connectCashout() {
    try {
      const response = await apiFetch<{ onboarding_url: string }>('/api/cashout/connect', {
        method: 'POST',
        body: JSON.stringify({})
      });
      window.open(response.onboarding_url, '_blank', 'noopener,noreferrer');
      toast.success('Opened Stripe Connect onboarding');
      await refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Cashout connect failed');
    }
  }

  async function requestCashout() {
    try {
      await apiFetch('/api/cashout/request', {
        method: 'POST',
        body: JSON.stringify({ amount: cashoutAmount })
      });
      toast.success('Cashout requested');
      await refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Cashout request failed');
    }
  }

  async function unblock(userId: string) {
    try {
      await apiFetch(`/api/blocks/${userId}`, { method: 'DELETE' });
      toast.success('User unblocked');
      await refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to unblock user');
    }
  }

  async function changePassword() {
    if (newPassword !== confirmPassword) {
      toast.error('Passwords do not match');
      return;
    }
    if (newPassword.length < 8) {
      toast.error('New password must be at least 8 characters');
      return;
    }
    changingPassword = true;
    try {
      await apiFetch('/api/auth/change-password', {
        method: 'POST',
        body: JSON.stringify({ current_password: currentPassword, new_password: newPassword })
      });
      toast.success('Password changed');
      currentPassword = '';
      newPassword = '';
      confirmPassword = '';
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to change password');
    } finally {
      changingPassword = false;
    }
  }

  async function deleteAccount() {
    if (!deleteConfirmed) {
      toast.error('Please confirm you understand your account will be deleted');
      return;
    }
    deletingAccount = true;
    try {
      await apiFetch('/api/users/me/delete', {
        method: 'POST',
        body: JSON.stringify({ password: deletePassword })
      });
      toast.success('Account scheduled for deletion in 30 days');
      deletePassword = '';
      deleteConfirmed = false;
      await refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to delete account');
    } finally {
      deletingAccount = false;
    }
  }

  async function cancelDeletion() {
    cancellingDeletion = true;
    try {
      await apiFetch('/api/users/me/cancel-deletion', { method: 'POST' });
      toast.success('Account deletion cancelled');
      await refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to cancel deletion');
    } finally {
      cancellingDeletion = false;
    }
  }

  async function uploadEmote() {
    if (!emoteFile) {
      toast.error('Select an emote file first');
      return;
    }

    const form = new FormData();
    form.append('token', emoteToken);
    form.append('name', emoteName);
    form.append('price_sparks', String(emotePrice));
    form.append('file', emoteFile);

    try {
      await apiFetch('/api/emotes/upload', {
        method: 'POST',
        body: form
      });
      toast.success('Emote uploaded');
      emoteFile = null;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Emote upload failed');
    }
  }
</script>

<svelte:head><title>Settings - othergirl</title></svelte:head>

<section class="space-y-4">
  <h1 class="text-2xl font-bold">Settings</h1>

  {#if !$auth.user}
    <div class="surface p-4 text-[var(--text-secondary)]">Login to manage your account settings.</div>
  {:else}
    <div class="flex gap-4 border-b border-[var(--border-default)]">
      <button
        class="pb-2 text-sm font-medium {activeTab === 'account' ? 'border-b-2 border-[var(--accent,var(--text-primary))] text-[var(--text-primary)]' : 'text-[var(--text-muted)] hover:text-[var(--text-secondary)]'}"
        onclick={() => (activeTab = 'account')}
      >Account</button>
      <button
        class="pb-2 text-sm font-medium {activeTab === 'subscription' ? 'border-b-2 border-[var(--accent,var(--text-primary))] text-[var(--text-primary)]' : 'text-[var(--text-muted)] hover:text-[var(--text-secondary)]'}"
        onclick={() => (activeTab = 'subscription')}
      >Subscription</button>
      <button
        class="pb-2 text-sm font-medium {activeTab === 'privacy' ? 'border-b-2 border-[var(--accent,var(--text-primary))] text-[var(--text-primary)]' : 'text-[var(--text-muted)] hover:text-[var(--text-secondary)]'}"
        onclick={() => (activeTab = 'privacy')}
      >Privacy</button>
    </div>

    {#if activeTab === 'account'}
      <div class="space-y-4">
        <div class="surface space-y-3 p-5">
          <h2 class="text-lg font-semibold">Change Password</h2>
          <div class="space-y-2">
            <div class="relative">
              {#if showCurrentPw}
                <input class="input w-full pr-10" type="text" bind:value={currentPassword} placeholder="Current password" />
              {:else}
                <input class="input w-full pr-10" type="password" bind:value={currentPassword} placeholder="Current password" />
              {/if}
              <button
                type="button"
                class="absolute right-2 top-1/2 -translate-y-1/2 text-[var(--text-muted)]"
                onclick={() => (showCurrentPw = !showCurrentPw)}
              >
                {#if showCurrentPw}<EyeOff size={16} />{:else}<Eye size={16} />{/if}
              </button>
            </div>
            <div class="relative">
              {#if showNewPw}
                <input class="input w-full pr-10" type="text" bind:value={newPassword} placeholder="New password (min 8 chars)" />
              {:else}
                <input class="input w-full pr-10" type="password" bind:value={newPassword} placeholder="New password (min 8 chars)" />
              {/if}
              <button
                type="button"
                class="absolute right-2 top-1/2 -translate-y-1/2 text-[var(--text-muted)]"
                onclick={() => (showNewPw = !showNewPw)}
              >
                {#if showNewPw}<EyeOff size={16} />{:else}<Eye size={16} />{/if}
              </button>
            </div>
            <input class="input w-full" type="password" bind:value={confirmPassword} placeholder="Confirm new password" />
          </div>
          <button class="btn-primary" onclick={changePassword} disabled={changingPassword}>
            {changingPassword ? 'Changing...' : 'Change Password'}
          </button>
        </div>

        <div class="surface space-y-3 p-5">
          <h2 class="text-lg font-semibold">Edit Profile</h2>
          <p class="text-sm text-[var(--text-secondary)]">Update your display name, bio, and avatar.</p>
          <a href="/profile" class="btn-secondary inline-block">Go to Profile</a>
        </div>

        <div class="surface space-y-3 p-5">
          <h2 class="text-lg font-semibold">Admin Emote Upload</h2>
          <p class="text-sm text-[var(--text-secondary)]">Requires your user ID in backend `ADMIN_USER_IDS`.</p>
          <div class="grid gap-2 md:grid-cols-3">
            <input class="input" bind:value={emoteToken} placeholder=":my_emote:" />
            <input class="input" bind:value={emoteName} placeholder="Emote name" />
            <input class="input" type="number" min="0" bind:value={emotePrice} />
          </div>
          <input
            type="file"
            accept="image/png,image/jpeg,image/webp,image/gif"
            onchange={(event: Event) => {
              const input = event.currentTarget as HTMLInputElement;
              emoteFile = input.files?.[0] ?? null;
            }}
          />
          <button class="btn-secondary" onclick={uploadEmote}>Upload Emote</button>
        </div>
      </div>
    {/if}

    {#if activeTab === 'subscription'}
      <div class="space-y-4">
        <div class="surface space-y-3 p-5">
          <h2 class="text-lg font-semibold">Premium</h2>
          <p class="text-sm text-[var(--text-secondary)]">
            Status: {$auth.user.is_premium ? 'Premium active' : 'Free tier'}
          </p>
          <button class="btn-primary" onclick={subscribe}>Activate Premium</button>
        </div>

        <div class="surface space-y-3 p-5">
          <h2 class="text-lg font-semibold">Sparks</h2>
          <p class="text-sm text-[var(--text-secondary)]">Current balance: {balance}</p>
          {#if sparkBundles.length === 0}
            <p class="text-sm text-[var(--text-muted)]">Loading bundles...</p>
          {:else}
            <div class="grid gap-2">
              {#each sparkBundles as bundle}
                <button
                  class="btn-primary w-full"
                  onclick={() => buySparks(bundle.index)}
                >
                  {bundle.sparks} sparks &mdash; ${(bundle.price_cents / 100).toFixed(2)}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <div class="surface space-y-3 p-5">
          <h2 class="text-lg font-semibold">Cash Out</h2>
          {#if connectStatus}
            <p class="text-sm text-[var(--text-secondary)]">Connected: {connectStatus.stripe_account_id}</p>
            <p class="text-sm text-[var(--text-secondary)]">Payouts enabled: {connectStatus.payouts_enabled ? 'Yes' : 'No'}</p>
          {:else}
            <p class="text-sm text-[var(--text-secondary)]">No cashout account linked.</p>
          {/if}
          <button class="btn-secondary" onclick={connectCashout}>Link cashout account</button>
          <input class="input" type="number" min="1000" bind:value={cashoutAmount} />
          <button class="btn-primary" onclick={requestCashout}>Request Cashout</button>
        </div>

        <div class="surface space-y-3 p-5">
          <h2 class="text-lg font-semibold">Recent Transactions</h2>
          {#if transactions.length === 0}
            <p class="text-sm text-[var(--text-secondary)]">No transactions yet.</p>
          {:else}
            <div class="space-y-2">
              {#each transactions.slice(0, 10) as tx}
                <div class="rounded-xl border border-[var(--border-default)] bg-[var(--bg-overlay)] p-3 text-sm">
                  <p class="font-semibold">{tx.transaction_type}</p>
                  <p class="text-[var(--text-secondary)]">{tx.amount > 0 ? '+' : ''}{tx.amount}</p>
                  <p class="text-xs text-[var(--text-muted)]">{new Date(tx.created_at).toLocaleString()}</p>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}

    {#if activeTab === 'privacy'}
      <div class="space-y-4">
        <div class="surface space-y-3 p-5">
          <h2 class="text-lg font-semibold">Blocked Users</h2>
          {#if blockedUsers.length === 0}
            <p class="text-sm text-[var(--text-secondary)]">No blocked users.</p>
          {:else}
            <div class="grid gap-2 md:grid-cols-2">
              {#each blockedUsers as blocked}
                <div class="rounded-xl border border-[var(--border-default)] bg-[var(--bg-overlay)] p-3 text-sm">
                  <p class="font-semibold">{blocked.username}</p>
                  <p class="text-xs text-[var(--text-muted)]">{new Date(blocked.created_at).toLocaleString()}</p>
                  <button class="btn-secondary mt-2" onclick={() => unblock(blocked.id)}>Unblock</button>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <div class="surface space-y-3 border-2 border-red-500/50 p-5">
          <h2 class="text-lg font-semibold text-red-400">Danger Zone</h2>
          {#if deletionScheduledAt}
            {@const scheduledDate = new Date(deletionScheduledAt)}
            {@const daysLeft = Math.max(0, Math.ceil((scheduledDate.getTime() - Date.now()) / (1000 * 60 * 60 * 24)))}
            <div class="rounded-xl border border-red-500/30 bg-red-500/10 p-4 space-y-2">
              <p class="text-sm font-semibold text-red-300">Account scheduled for deletion</p>
              <p class="text-sm text-[var(--text-secondary)]">
                Your account will be permanently deleted on {scheduledDate.toLocaleDateString()} ({daysLeft} days remaining).
              </p>
              <p class="text-sm text-[var(--text-secondary)]">You can cancel the deletion at any time before then.</p>
              <button
                class="btn-secondary mt-2"
                onclick={cancelDeletion}
                disabled={cancellingDeletion}
              >
                {cancellingDeletion ? 'Cancelling...' : 'Cancel Deletion'}
              </button>
            </div>
          {:else}
            <p class="text-sm text-[var(--text-secondary)]">
              Once you delete your account, it will be permanently removed after 30 days. You can cancel during the grace period.
            </p>
            <input
              class="input w-full"
              type="password"
              bind:value={deletePassword}
              placeholder="Enter your password to confirm"
            />
            <label class="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
              <input type="checkbox" bind:checked={deleteConfirmed} />
              I understand my account will be permanently deleted after 30 days
            </label>
            <button
              class="rounded-lg bg-red-600 px-4 py-2 text-sm font-semibold text-white hover:bg-red-700 disabled:opacity-50"
              onclick={deleteAccount}
              disabled={deletingAccount || !deletePassword || !deleteConfirmed}
            >
              {deletingAccount ? 'Deleting...' : 'Delete my account'}
            </button>
          {/if}
        </div>
      </div>
    {/if}
  {/if}
</section>
