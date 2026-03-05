<script lang="ts">
  import { type Snippet } from 'svelte';
  import { page } from '$app/stores';
  import '../app.css';
  import '$lib/stores/theme';
  import { Toaster } from 'svelte-sonner';

  import Navbar from '$lib/components/Navbar.svelte';

  let { children }: { children: Snippet } = $props();
</script>

<Navbar />

<main class="mx-auto w-full max-w-6xl px-4 py-6">
  {#key $page.url.pathname}
    <div class="page-transition">
      {@render children()}
    </div>
  {/key}
</main>

<Toaster richColors position="top-right" />

<style>
  .page-transition {
    animation: fadeIn 180ms ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
