<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { Camera, ArrowLeft, ArrowRight, Sparkles, Check } from 'lucide-svelte';

  import Avatar from '$lib/components/Avatar.svelte';
  import { apiFetch } from '$lib/api';
  import { auth, setSession } from '$lib/stores/auth';
  import type { UserResponse } from '$lib/types';

  type Category = { id: string; name: string; description?: string };
  type Language = { code: string; name: string };

  let step = $state(1);
  const totalSteps = 4;

  // Step 1 state
  let displayName = $state('');
  let bio = $state('');
  let avatarUrl: string | null = $state(null);
  let avatarUploading = $state(false);
  let avatarInput: HTMLInputElement = $state(null!);

  // Step 2 state
  let categories: Category[] = $state([]);
  let selectedInterests: string[] = $state([]);
  let categoriesLoading = $state(false);

  // Step 3 state
  let languages: Language[] = $state([]);
  let selectedLanguage = $state('en');
  let languagesLoading = $state(false);

  // General
  let saving = $state(false);

  let progress = $derived((step / totalSteps) * 100);

  let canAdvance = $derived.by(() => {
    if (step === 2) return selectedInterests.length >= 3 && selectedInterests.length <= 5;
    return true;
  });

  let interestHint = $derived.by(() => {
    const count = selectedInterests.length;
    if (count < 3) return `Select at least ${3 - count} more`;
    if (count > 5) return 'Too many selected, remove some';
    return `${count} of 5 selected`;
  });

  onMount(() => {
    if ($auth.user) {
      avatarUrl = $auth.user.avatar_url ?? null;
      displayName = $auth.user.username ?? '';
    }
  });

  async function loadCategories() {
    if (categories.length > 0) return;
    categoriesLoading = true;
    try {
      const result = await apiFetch<{ categories: Category[] }>('/api/categories');
      categories = result.categories;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load categories');
    } finally {
      categoriesLoading = false;
    }
  }

  async function loadLanguages() {
    if (languages.length > 0) return;
    languagesLoading = true;
    try {
      const result = await apiFetch<{ languages: Language[] }>('/api/languages');
      languages = result.languages;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load languages');
    } finally {
      languagesLoading = false;
    }
  }

  function toggleInterest(id: string) {
    if (selectedInterests.includes(id)) {
      selectedInterests = selectedInterests.filter((v) => v !== id);
    } else if (selectedInterests.length < 5) {
      selectedInterests = [...selectedInterests, id];
    }
  }

  async function uploadAvatar(event: Event) {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;

    avatarUploading = true;
    try {
      const formData = new FormData();
      formData.append('avatar', file);
      const result = await apiFetch<{ avatar_url: string }>('/api/users/me/avatar', {
        method: 'POST',
        body: formData
      });
      avatarUrl = result.avatar_url;

      if ($auth.token && $auth.user) {
        setSession($auth.token, { ...$auth.user, avatar_url: result.avatar_url });
      }

      toast.success('Avatar uploaded');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to upload avatar');
    } finally {
      avatarUploading = false;
      target.value = '';
    }
  }

  async function saveProfile() {
    saving = true;
    try {
      const profile = await apiFetch<UserResponse>('/api/users/me', {
        method: 'PUT',
        body: JSON.stringify({
          bio,
          interest_category_ids: selectedInterests
        })
      });

      if ($auth.token) {
        setSession($auth.token, {
          id: profile.id,
          username: profile.username,
          email: profile.email,
          avatar_url: profile.avatar_url,
          is_premium: profile.is_premium,
          is_age_verified: profile.is_age_verified,
          created_at: profile.created_at
        });
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to save profile');
      throw error;
    } finally {
      saving = false;
    }
  }

  async function saveLanguagePreference() {
    saving = true;
    try {
      await apiFetch('/api/users/me/preferences', {
        method: 'PUT',
        body: JSON.stringify({ language: selectedLanguage })
      });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to save preferences');
      throw error;
    } finally {
      saving = false;
    }
  }

  async function nextStep() {
    try {
      if (step === 1) {
        await saveProfile();
      } else if (step === 2) {
        await saveProfile();
      } else if (step === 3) {
        await saveLanguagePreference();
      }

      if (step < totalSteps) {
        step += 1;

        if (step === 2) await loadCategories();
        if (step === 3) await loadLanguages();
      }
    } catch {
      // Error already toasted in save functions
    }
  }

  function prevStep() {
    if (step > 1) step -= 1;
  }

  function startChatting() {
    goto('/chat');
  }
</script>

<svelte:head><title>Get Started - othergirl</title></svelte:head>

<section class="mx-auto flex min-h-[80vh] max-w-xl flex-col px-4 py-8">
  <!-- Progress bar -->
  <div class="mb-8">
    <div class="mb-2 flex items-center justify-between text-xs text-[var(--text-muted)]">
      <span>Step {step} of {totalSteps}</span>
      <span>{Math.round(progress)}%</span>
    </div>
    <div class="h-1.5 w-full overflow-hidden rounded-full bg-[var(--bg-overlay)]">
      <div
        class="h-full rounded-full bg-[var(--accent)] transition-all duration-500 ease-out"
        style="width: {progress}%"
      ></div>
    </div>
  </div>

  <!-- Step content -->
  <div class="flex-1">
    {#if step === 1}
      <div class="surface space-y-6 p-6">
        <div class="text-center">
          <h1 class="mb-2 text-2xl font-bold text-[var(--text-primary)]">Welcome to othergirl!</h1>
          <p class="text-sm text-[var(--text-secondary)]">
            Let's set up your profile. Don't worry, you can always change this later.
          </p>
        </div>

        <!-- Avatar upload -->
        <div class="flex flex-col items-center gap-3">
          <button
            class="group relative cursor-pointer"
            onclick={() => avatarInput.click()}
            aria-label="Upload avatar"
            disabled={avatarUploading}
            type="button"
          >
            <Avatar url={avatarUrl} username={$auth.user?.username ?? '?'} size={96} />
            <span class="absolute inset-0 flex items-center justify-center rounded-full bg-black/40 opacity-0 transition group-hover:opacity-100">
              <Camera size={24} class="text-white" />
            </span>
          </button>
          <p class="text-xs text-[var(--text-muted)]">
            {avatarUploading ? 'Uploading...' : 'Click to upload a photo'}
          </p>
          <input
            bind:this={avatarInput}
            type="file"
            accept="image/jpeg,image/png,image/webp,image/gif"
            class="hidden"
            onchange={uploadAvatar}
          />
        </div>

        <!-- Display name (read-only, from registration) -->
        <div>
          <label for="display-name" class="mb-1 block text-xs uppercase tracking-wide text-[var(--text-muted)]">
            Display name
          </label>
          <input
            id="display-name"
            type="text"
            class="input"
            value={displayName}
            disabled
            placeholder="Your username"
          />
          <p class="mt-1 text-xs text-[var(--text-muted)]">This was set during registration</p>
        </div>

        <!-- Bio -->
        <div>
          <label for="bio-input" class="mb-1 block text-xs uppercase tracking-wide text-[var(--text-muted)]">
            Bio
          </label>
          <textarea
            id="bio-input"
            class="input min-h-24"
            bind:value={bio}
            placeholder="Tell people what you're into..."
            maxlength={500}
          ></textarea>
          <p class="mt-1 text-right text-xs text-[var(--text-muted)]">{bio.length}/500</p>
        </div>
      </div>

    {:else if step === 2}
      <div class="surface space-y-6 p-6">
        <div class="text-center">
          <h1 class="mb-2 text-2xl font-bold text-[var(--text-primary)]">Pick your interests</h1>
          <p class="text-sm text-[var(--text-secondary)]">
            Choose 3 to 5 categories so we can match you with like-minded people.
          </p>
        </div>

        {#if categoriesLoading}
          <div class="flex justify-center py-8">
            <div class="h-8 w-8 animate-spin rounded-full border-2 border-[var(--accent)] border-t-transparent"></div>
          </div>
        {:else}
          <div class="grid gap-2 sm:grid-cols-2">
            {#each categories as category (category.id)}
              <button
                class={`rounded-xl border px-4 py-3 text-left text-sm transition ${
                  selectedInterests.includes(category.id)
                    ? 'border-[var(--accent)] bg-[var(--accent)]/10 text-[var(--text-primary)]'
                    : 'border-[var(--border-default)] bg-[var(--bg-overlay)] text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)]'
                }`}
                onclick={() => toggleInterest(category.id)}
                type="button"
              >
                <span class="font-medium">{category.name}</span>
                {#if category.description}
                  <span class="mt-0.5 block text-xs text-[var(--text-muted)]">{category.description}</span>
                {/if}
                {#if selectedInterests.includes(category.id)}
                  <Check size={14} class="absolute right-2 top-2 text-[var(--accent)]" />
                {/if}
              </button>
            {/each}
          </div>

          <p class="text-center text-xs text-[var(--text-muted)]">{interestHint}</p>
        {/if}
      </div>

    {:else if step === 3}
      <div class="surface space-y-6 p-6">
        <div class="text-center">
          <h1 class="mb-2 text-2xl font-bold text-[var(--text-primary)]">Preferences</h1>
          <p class="text-sm text-[var(--text-secondary)]">
            Choose your preferred language for the chat experience.
          </p>
        </div>

        {#if languagesLoading}
          <div class="flex justify-center py-8">
            <div class="h-8 w-8 animate-spin rounded-full border-2 border-[var(--accent)] border-t-transparent"></div>
          </div>
        {:else}
          <div>
            <label for="language-select" class="mb-1 block text-xs uppercase tracking-wide text-[var(--text-muted)]">
              Language
            </label>
            <select id="language-select" class="input" bind:value={selectedLanguage}>
              {#each languages as lang (lang.code)}
                <option value={lang.code}>{lang.name}</option>
              {/each}
            </select>
          </div>
        {/if}
      </div>

    {:else if step === 4}
      <div class="surface space-y-6 p-6 text-center">
        <div class="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-[var(--accent)]/10">
          <Sparkles size={32} class="text-[var(--accent)]" />
        </div>
        <div>
          <h1 class="mb-2 text-2xl font-bold text-[var(--text-primary)]">You're all set!</h1>
          <p class="text-sm text-[var(--text-secondary)]">
            Your profile is ready. Jump into a chat and meet someone new.
          </p>
        </div>
        <button class="btn-primary w-full" onclick={startChatting}>
          Start chatting
        </button>
      </div>
    {/if}
  </div>

  <!-- Navigation -->
  {#if step < 4}
    <div class="mt-6 flex items-center justify-between gap-4">
      {#if step > 1}
        <button class="btn-secondary flex items-center gap-1.5" onclick={prevStep} type="button">
          <ArrowLeft size={16} />
          Back
        </button>
      {:else}
        <div></div>
      {/if}

      <button
        class="btn-primary flex items-center gap-1.5"
        onclick={nextStep}
        disabled={!canAdvance || saving}
        type="button"
      >
        {saving ? 'Saving...' : 'Next'}
        <ArrowRight size={16} />
      </button>
    </div>
  {/if}
</section>
