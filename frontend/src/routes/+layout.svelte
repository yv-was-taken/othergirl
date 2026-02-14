<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import { Toaster } from 'svelte-sonner';

  import Navbar from '$lib/components/Navbar.svelte';
  import { loadEmotes } from '$lib/utils/emotes';

  onMount(() => {
    loadEmotes();
  });
</script>

<Navbar />

<main class="mx-auto w-full max-w-6xl px-4 py-6">
  <slot />
</main>

<Toaster richColors position="top-right" />
