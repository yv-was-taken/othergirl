<script lang="ts">
  import { MessageCircle, Sparkles, Shield, Users, Zap, Heart } from 'lucide-svelte';
</script>

<svelte:head><title>othergirl - Anonymous Chat</title></svelte:head>

<!-- Hero -->
<section class="flex flex-col items-center py-16 text-center md:py-24">
  <img src="/assets/othergirl-logo-transparent.png" alt="Othergirl" class="mb-8 h-16" />
  <h1 class="max-w-3xl text-4xl font-black leading-tight md:text-6xl">
    Meet strangers by interest, not&nbsp;profile.
  </h1>
  <p class="mt-4 max-w-xl text-lg text-[var(--text-secondary)]">
    Othergirl pairs you into 1:1 text chats by category and language. Keep the rare good conversation, skip the rest.
  </p>
  <div class="mt-8 flex flex-wrap justify-center gap-3">
    <a href="/chat" class="btn-primary px-6 py-3 text-lg">Start Chatting</a>
    <a href="/login" class="btn-secondary px-6 py-3 text-lg">Create Account</a>
  </div>
</section>

<!-- Features -->
<section class="grid gap-6 py-12 md:grid-cols-3">
  <div class="surface flex flex-col items-center p-6 text-center">
    <MessageCircle size={32} class="mb-3 text-[var(--text-muted)]" />
    <h3 class="font-semibold">Interest-Based Matching</h3>
    <p class="mt-2 text-sm text-[var(--text-secondary)]">Pick a category and get matched with someone who shares your interests. No swiping, no profiles.</p>
  </div>
  <div class="surface flex flex-col items-center p-6 text-center">
    <Shield size={32} class="mb-3 text-[var(--text-muted)]" />
    <h3 class="font-semibold">Safe &amp; Private</h3>
    <p class="mt-2 text-sm text-[var(--text-secondary)]">End-to-end encrypted messages, content safety filters, and easy blocking. Your conversations stay yours.</p>
  </div>
  <div class="surface flex flex-col items-center p-6 text-center">
    <Heart size={32} class="mb-3 text-[var(--text-muted)]" />
    <h3 class="font-semibold">Keep or Skip</h3>
    <p class="mt-2 text-sm text-[var(--text-secondary)]">Great conversation? Both vote to keep it. Otherwise, move on to the next match instantly.</p>
  </div>
</section>

<!-- How it works -->
<section class="py-12">
  <h2 class="mb-8 text-center text-2xl font-bold">How it works</h2>
  <div class="grid gap-6 md:grid-cols-3">
    <div class="flex flex-col items-center text-center">
      <div class="mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-[var(--bg-elevated)] text-sm font-bold">1</div>
      <h3 class="font-semibold">Choose a topic</h3>
      <p class="mt-1 text-sm text-[var(--text-secondary)]">Pick from dozens of categories — anime, music, tech, philosophy, and more.</p>
    </div>
    <div class="flex flex-col items-center text-center">
      <div class="mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-[var(--bg-elevated)] text-sm font-bold">2</div>
      <h3 class="font-semibold">Get matched</h3>
      <p class="mt-1 text-sm text-[var(--text-secondary)]">We pair you with a stranger who picked the same topic. Chat starts instantly.</p>
    </div>
    <div class="flex flex-col items-center text-center">
      <div class="mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-[var(--bg-elevated)] text-sm font-bold">3</div>
      <h3 class="font-semibold">Keep or next</h3>
      <p class="mt-1 text-sm text-[var(--text-secondary)]">If you click, both vote to keep the chat. If not, hit next for a fresh match.</p>
    </div>
  </div>
</section>

<!-- CTA -->
<section class="surface mx-auto max-w-2xl p-8 text-center">
  <Sparkles size={32} class="mx-auto mb-3 text-[var(--text-muted)]" />
  <h2 class="text-xl font-bold">Ready to meet someone new?</h2>
  <p class="mt-2 text-sm text-[var(--text-secondary)]">No sign-up required to start chatting. Create an account to keep conversations and earn sparks.</p>
  <div class="mt-6 flex justify-center gap-3">
    <a href="/chat" class="btn-primary">Start Now</a>
    <a href="/login" class="btn-secondary">Sign Up Free</a>
  </div>
</section>
