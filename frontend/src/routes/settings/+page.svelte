<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';

  import { apiFetch } from '$lib/api';
  import { auth, setSession } from '$lib/stores/auth';

  let balance = $state(0);
  let transactions: { id: string; amount: number; transaction_type: string; created_at: string }[] = $state([]);
  let connectStatus: { stripe_account_id: string; payouts_enabled: boolean } | null = $state(null);
  let blockedUsers: { id: string; username: string; created_at: string }[] = $state([]);
  let cashoutAmount = $state(1000);
  let buyAmount = $state(500);
  let emoteToken = $state(':my_emote:');
  let emoteName = $state('My Emote');
  let emotePrice = $state(100);
  let emoteFile: File | null = $state(null);

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    if (!$auth.token) return;

    try {
      const [meRes, balanceRes, txRes, cashoutRes, blocksRes] = await Promise.all([
        apiFetch<{
          id: string;
          username: string;
          email?: string | null;
          is_premium: boolean;
          is_age_verified: boolean;
          created_at: string;
        }>('/api/users/me'),
        apiFetch<{ balance: number }>('/api/sparks/balance'),
        apiFetch<{ transactions: { id: string; amount: number; transaction_type: string; created_at: string }[] }>('/api/sparks/transactions'),
        apiFetch<{ connect: { stripe_account_id: string; payouts_enabled: boolean } | null }>('/api/cashout/status'),
        apiFetch<{ blocked_users: { id: string; username: string; created_at: string }[] }>('/api/blocks')
      ]);

      balance = balanceRes.balance;
      transactions = txRes.transactions ?? [];
      connectStatus = cashoutRes.connect;
      blockedUsers = blocksRes.blocked_users ?? [];

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

  async function buySparks() {
    try {
      const response = await apiFetch<{ checkout_url: string }>('/api/payments/buy-sparks', {
        method: 'POST',
        body: JSON.stringify({ amount: buyAmount })
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

<section class="space-y-4">
  <h1 class="text-2xl font-bold">Settings</h1>

  {#if !$auth.user}
    <div class="surface p-4 text-slate-300">Login to manage your account settings.</div>
  {:else}
    <div class="grid gap-4 lg:grid-cols-2">
      <div class="surface space-y-3 p-5">
        <h2 class="text-lg font-semibold">Premium</h2>
        <p class="text-sm text-slate-300">
          Status: {$auth.user.is_premium ? 'Premium active' : 'Free tier'}
        </p>
        <button class="btn-primary" onclick={subscribe}>Activate Premium</button>
      </div>

      <div class="surface space-y-3 p-5">
        <h2 class="text-lg font-semibold">Sparks</h2>
        <p class="text-sm text-slate-300">Current balance: {balance}</p>
        <input class="input" type="number" min="1" bind:value={buyAmount} />
        <div class="flex gap-2">
          <button class="btn-primary flex-1" onclick={buySparks}>Buy (Stripe)</button>
        </div>
      </div>

      <div class="surface space-y-3 p-5">
        <h2 class="text-lg font-semibold">Cash Out</h2>
        {#if connectStatus}
          <p class="text-sm text-slate-300">Connected: {connectStatus.stripe_account_id}</p>
          <p class="text-sm text-slate-300">Payouts enabled: {connectStatus.payouts_enabled ? 'Yes' : 'No'}</p>
        {:else}
          <p class="text-sm text-slate-300">No cashout account linked.</p>
        {/if}
        <button class="btn-secondary" onclick={connectCashout}>Link cashout account</button>
        <input class="input" type="number" min="1000" bind:value={cashoutAmount} />
        <button class="btn-primary" onclick={requestCashout}>Request Cashout</button>
      </div>

      <div class="surface space-y-3 p-5">
        <h2 class="text-lg font-semibold">Recent Transactions</h2>
        {#if transactions.length === 0}
          <p class="text-sm text-slate-300">No transactions yet.</p>
        {:else}
          <div class="space-y-2">
            {#each transactions.slice(0, 10) as tx}
              <div class="rounded-xl border border-white/10 bg-white/5 p-3 text-sm">
                <p class="font-semibold">{tx.transaction_type}</p>
                <p class="text-slate-300">{tx.amount > 0 ? '+' : ''}{tx.amount}</p>
                <p class="text-xs text-slate-400">{new Date(tx.created_at).toLocaleString()}</p>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <div class="surface space-y-3 p-5 lg:col-span-2">
        <h2 class="text-lg font-semibold">Blocked Users</h2>
        {#if blockedUsers.length === 0}
          <p class="text-sm text-slate-300">No blocked users.</p>
        {:else}
          <div class="grid gap-2 md:grid-cols-2">
            {#each blockedUsers as blocked}
              <div class="rounded-xl border border-white/10 bg-white/5 p-3 text-sm">
                <p class="font-semibold">{blocked.username}</p>
                <p class="text-xs text-slate-400">{new Date(blocked.created_at).toLocaleString()}</p>
                <button class="btn-secondary mt-2" onclick={() => unblock(blocked.id)}>Unblock</button>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <div class="surface space-y-3 p-5 lg:col-span-2">
        <h2 class="text-lg font-semibold">Admin Emote Upload</h2>
        <p class="text-sm text-slate-300">Requires your user ID in backend `ADMIN_USER_IDS`.</p>
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
</section>
