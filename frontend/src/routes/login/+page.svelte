<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';

  import { Eye, EyeOff } from 'lucide-svelte';

  import { apiFetch } from '$lib/api';
  import { auth, setSession } from '$lib/stores/auth';
  import type { AuthResponse } from '$lib/types';

  let mode: 'login' | 'register' = $state('login');
  let showPassword = $state(false);

  let username = $state('');
  let email = $state('');
  let password = $state('');
  let isAgeVerified = $state(false);

  let loading = $state(false);

  const oauthProviders = ['google', 'discord', 'github', 'telegram'] as const;
  const OAUTH_NEXT_KEY = 'othergirl.oauth.next';

  onMount(async () => {
    const state = get(auth);
    if (state.token && state.user) {
      await goto('/chat', { replaceState: true });
      return;
    }

    const params = new URLSearchParams(window.location.search);
    const oauthCode = params.get('oauth_code');

    if (!oauthCode) {
      return;
    }

    const nextSearchParams = new URLSearchParams(window.location.search);
    nextSearchParams.delete('oauth_code');
    const nextSearch = nextSearchParams.toString();
    const cleanUrl = `${window.location.pathname}${nextSearch ? `?${nextSearch}` : ''}${window.location.hash}`;
    window.history.replaceState(null, '', cleanUrl);

    loading = true;
    try {
      const response = await apiFetch<AuthResponse>('/api/auth/oauth/exchange', {
        method: 'POST',
        body: JSON.stringify({ code: oauthCode })
      });

      setSession(response.token, response.user);
      toast.success('OAuth login complete');
      await goto(getOauthPostAuthRedirectPath(), { replaceState: true });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'OAuth login failed');
      await goto('/login', { replaceState: true });
    } finally {
      clearStoredOauthNextPath();
      loading = false;
    }
  });

  async function submit() {
    loading = true;

    try {
      if (mode === 'login') {
        const response = await apiFetch<AuthResponse>('/api/auth/login', {
          method: 'POST',
          body: JSON.stringify({ email, password })
        });

        setSession(response.token, response.user);
        toast.success('Logged in');
        await goto(getPostAuthRedirectPath(), { replaceState: true });
      } else {
        const response = await apiFetch<AuthResponse>('/api/auth/register', {
          method: 'POST',
          body: JSON.stringify({ username, email, password, is_age_verified: isAgeVerified })
        });

        setSession(response.token, response.user);
        toast.success('Account created');
        await goto(getPostAuthRedirectPath(), { replaceState: true });
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      toast.error(message);
    } finally {
      loading = false;
    }
  }

  async function oauthLogin(provider: (typeof oauthProviders)[number]) {
    loading = true;
    try {
      const next = getSafeNextFromQuery();
      if (next) {
        localStorage.setItem(OAUTH_NEXT_KEY, next);
      } else {
        clearStoredOauthNextPath();
      }

      const start = await apiFetch<{ redirect_url: string }>(`/api/auth/oauth/${provider}`);
      window.location.assign(start.redirect_url);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : `OAuth failed for ${provider}`);
      loading = false;
    }
  }

  function getPostAuthRedirectPath() {
    const next = getSafeNextFromQuery();

    if (!next) {
      return '/chat';
    }

    return next;
  }

  function getOauthPostAuthRedirectPath() {
    const nextFromQuery = getSafeNextFromQuery();
    if (nextFromQuery) {
      return nextFromQuery;
    }

    const stored = localStorage.getItem(OAUTH_NEXT_KEY);
    if (isSafeInternalPath(stored)) {
      return stored;
    }

    return '/chat';
  }

  function clearStoredOauthNextPath() {
    localStorage.removeItem(OAUTH_NEXT_KEY);
  }

  function getSafeNextFromQuery() {
    const params = new URLSearchParams(window.location.search);
    const next = params.get('next');
    return isSafeInternalPath(next) ? next : null;
  }

  function isSafeInternalPath(path: string | null): path is string {
    if (!path || path.includes('\\') || !path.startsWith('/')) {
      return false;
    }

    try {
      const parsed = new URL(path, window.location.origin);
      return parsed.origin === window.location.origin;
    } catch {
      return false;
    }
  }
</script>

<svelte:head><title>Login - othergirl</title></svelte:head>

<section class="mx-auto max-w-xl">
  <img src="/assets/othergirl-logo-transparent.png" alt="Othergirl" class="mx-auto mb-6 h-16" />
  <div class="surface space-y-5 p-6">
    <div class="flex gap-2">
      <button
        type="button"
        class={`btn-secondary flex-1 ${mode === 'login' ? 'bg-[var(--btn-secondary-hover)]' : ''}`}
        onclick={() => (mode = 'login')}
      >
        Login
      </button>
      <button
        type="button"
        class={`btn-secondary flex-1 ${mode === 'register' ? 'bg-[var(--btn-secondary-hover)]' : ''}`}
        onclick={() => (mode = 'register')}
      >
        Register
      </button>
    </div>

    <form class="space-y-3" onsubmit={(e) => { e.preventDefault(); submit(); }}>
      {#if mode === 'register'}
        <div>
          <label for="username-input" class="mb-1 block text-xs uppercase tracking-wide text-[var(--text-muted)]">Username</label>
          <input id="username-input" class="input" bind:value={username} required minlength={3} maxlength={32} />
        </div>
      {/if}

      <div>
        <label for="email-input" class="mb-1 block text-xs uppercase tracking-wide text-[var(--text-muted)]">Email</label>
        <input id="email-input" class="input" bind:value={email} required type="email" />
      </div>

      <div>
        <label for="password-input" class="mb-1 block text-xs uppercase tracking-wide text-[var(--text-muted)]">Password</label>
        <div class="relative">
          <input id="password-input" class="input pr-10" bind:value={password} required type={showPassword ? 'text' : 'password'} minlength={8} />
          <button type="button" class="absolute right-2 top-1/2 -translate-y-1/2 text-[var(--text-muted)] hover:text-[var(--text-primary)]" onclick={() => showPassword = !showPassword}>
            {#if showPassword}<EyeOff size={18} />{:else}<Eye size={18} />{/if}
          </button>
        </div>
      </div>

      {#if mode === 'register'}
        <label class="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
          <input type="checkbox" bind:checked={isAgeVerified} />
          I confirm I am 18+
        </label>
      {/if}

      <button class="btn-primary w-full" type="submit" disabled={loading}>
        {loading ? 'Please wait...' : mode === 'login' ? 'Login' : 'Create account'}
      </button>
    </form>

    <div class="space-y-2">
      <p class="text-center text-xs uppercase tracking-wide text-[var(--text-muted)]">or continue with OAuth</p>
      <div class="grid grid-cols-2 gap-2">
        {#each oauthProviders as provider}
          <button type="button" class="btn-secondary" disabled={loading} onclick={() => oauthLogin(provider)}>
            {provider}
          </button>
        {/each}
      </div>
    </div>
  </div>
</section>
