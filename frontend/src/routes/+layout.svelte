<script lang="ts">
  import { type Snippet } from 'svelte';
  import '../app.css';
  import { Toaster } from 'svelte-sonner';

  import Navbar from '$lib/components/Navbar.svelte';

  let { children }: { children: Snippet } = $props();
</script>

<Navbar />

<main class="mx-auto w-full max-w-6xl px-4 py-6">
  {@render children()}
</main>

<Toaster richColors position="top-right" />
