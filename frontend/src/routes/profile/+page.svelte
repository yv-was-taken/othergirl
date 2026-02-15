<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';

  import { apiFetch } from '$lib/api';
  import { auth, setSession } from '$lib/stores/auth';

  type Category = { id: string; name: string };

  let bio = $state('');
  let isAgeVerified = $state(false);
  let loading = $state(false);

  let categories: Category[] = $state([]);
  let selectedInterests: string[] = $state([]);

  onMount(async () => {
    if (!$auth.token) return;

    loading = true;
    try {
      const [profile, categoryList] = await Promise.all([
        apiFetch<{
          id: string;
          username: string;
          email?: string;
          bio: string;
          is_premium: boolean;
          is_age_verified: boolean;
          created_at: string;
          interest_category_ids: string[];
        }>('/api/users/me'),
        apiFetch<Category[]>('/api/categories')
      ]);

      categories = categoryList;
      bio = profile.bio ?? '';
      isAgeVerified = profile.is_age_verified;
      selectedInterests = profile.interest_category_ids ?? [];
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load profile');
    } finally {
      loading = false;
    }
  });

  function toggleInterest(id: string) {
    if (selectedInterests.includes(id)) {
      selectedInterests = selectedInterests.filter((v) => v !== id);
    } else {
      selectedInterests = [...selectedInterests, id];
    }
  }

  async function save() {
    if (!$auth.user) return;

    loading = true;
    try {
      const profile = await apiFetch<{
        id: string;
        username: string;
        email?: string;
        bio: string;
        is_premium: boolean;
        is_age_verified: boolean;
        created_at: string;
      }>('/api/users/me', {
        method: 'PUT',
        body: JSON.stringify({
          bio,
          is_age_verified: isAgeVerified,
          interest_category_ids: selectedInterests
        })
      });

      if ($auth.token) {
        setSession($auth.token, {
          id: profile.id,
          username: profile.username,
          email: profile.email,
          is_premium: profile.is_premium,
          is_age_verified: profile.is_age_verified,
          created_at: profile.created_at
        });
      }

      toast.success('Profile updated');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to save profile');
    } finally {
      loading = false;
    }
  }
</script>

<section class="mx-auto max-w-3xl space-y-4">
  <h1 class="text-2xl font-bold">Profile</h1>

  {#if !$auth.user}
    <div class="surface p-4 text-slate-300">Login to edit your profile.</div>
  {:else}
    <div class="surface space-y-4 p-5">
      <div>
        <label for="bio-input" class="mb-1 block text-xs uppercase tracking-wide text-slate-400">Bio</label>
        <textarea id="bio-input" class="input min-h-28" bind:value={bio} placeholder="Tell strangers what you are into"></textarea>
      </div>

      <label class="flex items-center gap-2 text-sm text-slate-300">
        <input type="checkbox" bind:checked={isAgeVerified} />
        I confirm I am 18+
      </label>

      <div class="space-y-2">
        <p class="text-xs uppercase tracking-wide text-slate-400">Interests</p>
        <div class="grid gap-2 md:grid-cols-3">
          {#each categories as category}
            <button
              class={`rounded-xl border px-3 py-2 text-left text-sm transition ${
                selectedInterests.includes(category.id)
                  ? 'border-white/40 bg-white/15 text-white'
                  : 'border-white/10 bg-white/5 text-slate-200 hover:bg-white/10'
              }`}
              onclick={() => toggleInterest(category.id)}
              type="button"
            >
              {category.name}
            </button>
          {/each}
        </div>
      </div>

      <button class="btn-primary" onclick={save} disabled={loading}>Save profile</button>
    </div>
  {/if}
</section>
