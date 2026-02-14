<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import '../app.css';
  import { Toaster } from 'svelte-sonner';

  import Navbar from '$lib/components/Navbar.svelte';
  import { loadEmotes } from '$lib/utils/emotes';

  let { children }: { children: Snippet } = $props();

  onMount(() => {
    loadEmotes();
  });
</script>

<Navbar />

<main class="mx-auto w-full max-w-6xl px-4 py-6">
  {@render children()}
</main>

<Toaster richColors position="top-right" />
