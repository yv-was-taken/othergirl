<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';

  import AdBanner from '$lib/components/AdBanner.svelte';
  import ChatWindow from '$lib/components/ChatWindow.svelte';
  import KeepVoteModal from '$lib/components/KeepVoteModal.svelte';
  import MatchmakingPanel from '$lib/components/MatchmakingPanel.svelte';
  import ProfileCard from '$lib/components/ProfileCard.svelte';
  import { apiFetch } from '$lib/api';
  import { auth } from '$lib/stores/auth';
  import {
    chat,
    pushMessage,
    resetChatState,
    setCooldown,
    setMatch,
    setPartnerTyping,
    setQueue
  } from '$lib/stores/chat';
  import { connectChatSocket, type ChatSocket } from '$lib/ws';

  type Category = { id: string; name: string };
  type Language = { id: string; name: string };

  let categories: Category[] = $state([]);
  let languages: Language[] = $state([]);

  let selectedCategory = $state('');
  let selectedLanguage = $state('');

  let socket: ChatSocket | null = null;
  let statusTimer: ReturnType<typeof setInterval> | null = null;

  let keepPromptOpen = $state(false);
  let connectionError = $state('');

  // Typing indicator debounce/throttle state
  let typingTimeout: ReturnType<typeof setTimeout> | null = null;
  let lastTypingSentAt = 0;
  const TYPING_THROTTLE_MS = 500;
  const TYPING_STOP_DELAY_MS = 2000;

  onMount(async () => {
    await Promise.all([loadCategories(), loadLanguages()]);
    startQueuePolling();
  });

  onDestroy(() => {
    socket?.close();
    if (statusTimer) clearInterval(statusTimer);
    if (typingTimeout) clearTimeout(typingTimeout);
  });

  async function loadCategories() {
    try {
      const response = await apiFetch<Category[]>('/api/categories');
      categories = response;
      selectedCategory = response[0]?.id ?? '';
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load categories');
    }
  }

  async function loadLanguages() {
    try {
      const response = await apiFetch<Language[]>('/api/languages');
      languages = response;
      selectedLanguage = response[0]?.id ?? '';
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load languages');
    }
  }

  async function joinQueue() {
    const state = get(auth);
    if (!state.token) {
      toast.error('Login first');
      return;
    }

    try {
      const response = await apiFetch<
        | {
            result: 'queued';
            position: number;
            estimated_wait_seconds: number;
            queue: string;
          }
        | {
            result: 'matched';
            chat_id: string;
            partner: {
              id: string;
              username: string;
              bio?: string;
              badges?: string[];
              interests?: string[];
              flare?: Record<string, unknown>;
            };
          }
      >('/api/matchmaking/join', {
        method: 'POST',
        body: JSON.stringify({ category_id: selectedCategory, language_id: selectedLanguage })
      });

      if (response.result === 'queued') {
        setQueue(response.position, response.estimated_wait_seconds);
        toast.message(`Queued at position ${response.position}`);
      } else {
        setMatch(response.chat_id, response.partner);
        connectToChat(response.chat_id);
        toast.success(`Matched with ${response.partner.username}`);
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to join queue');
    }
  }

  async function leaveQueue() {
    try {
      await apiFetch('/api/matchmaking/leave', { method: 'DELETE' });
      resetChatState();
      toast.message('Left queue');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to leave queue');
    }
  }

  async function startQueuePolling() {
    if (statusTimer) clearInterval(statusTimer);

    statusTimer = setInterval(async () => {
      const state = get(auth);
      if (!state.token) return;

      try {
        const status = await apiFetch<
          | { state: 'idle' }
          | { state: 'queued'; position: number; estimated_wait_seconds: number; queue: string }
          | { state: 'matched'; chat_id: string }
        >('/api/matchmaking/status');

        if (status.state === 'queued') {
          setQueue(status.position, status.estimated_wait_seconds);
        }

        if (status.state === 'matched' && !get(chat).chatId) {
          setMatch(status.chat_id, { id: '', username: 'stranger', badges: [], interests: [], flare: {} });
          connectToChat(status.chat_id);
          toast.success('Matched');
        }
      } catch {
        // Keep polling quiet on transient errors.
      }
    }, 1500);
  }

  function connectToChat(chatId: string) {
    if (!get(auth).token) return;

    socket?.close();
    connectionError = '';
    socket = connectChatSocket(chatId, () => get(auth).token, {
      onEvent(event) {
        switch (event.type) {
          case 'queued':
            setQueue(Number(event.position ?? 1), 4);
            break;
          case 'matched':
            if (typeof event.chat_id === 'string' && event.partner && !get(chat).chatId) {
              setMatch(String(event.chat_id), event.partner as Record<string, unknown> as {
                id: string;
                username: string;
                bio?: string;
                badges?: string[];
                interests?: string[];
                flare?: Record<string, unknown>;
              });
            }
            break;
          case 'message':
            pushMessage({
              id: String(event.id ?? Date.now()),
              chat_id: String(event.chat_id ?? chatId),
              sender_id: String(event.sender_id ?? ''),
              content: String(event.content ?? ''),
              timestamp: String(event.timestamp ?? new Date().toISOString()),
              flagged: Boolean(event.flagged)
            });
            break;
          case 'typing':
            setPartnerTyping(Boolean(event.is_typing));
            break;
          case 'partner_left':
            toast.message('Partner left the chat');
            socket?.close();
            socket = null;
            resetChatState();
            break;
          case 'cooldown':
            setCooldown(Number(event.seconds ?? 5));
            break;
          case 'partner_keep_vote':
            keepPromptOpen = true;
            toast.message('Partner voted on keep');
            break;
          case 'keep_result':
            keepPromptOpen = false;
            toast.success(Boolean(event.both_kept) ? 'Chat kept by both users' : 'Chat not kept');
            break;
          case 'award_received': {
            const me = get(auth).user;
            const recipientId = String(event.recipient_id ?? '');
            if (me && recipientId === me.id) {
              toast.success(`You received ${event.your_cut} sparks`);
            } else {
              toast.message('Award sent');
            }
            break;
          }
          case 'error':
            if (event.code === 'CONNECTION_LOST') {
              connectionError = String(event.message);
            }
            toast.error(String(event.message ?? 'WebSocket error'));
            break;
          default:
            break;
        }
      }
    });
  }

  function sendMessage(detail: { content: string }) {
    socket?.send({ type: 'message', content: detail.content });
  }

  function sendTyping(detail: { isTyping: boolean }) {
    // Clear any pending stop-typing timeout
    if (typingTimeout) {
      clearTimeout(typingTimeout);
      typingTimeout = null;
    }

    if (detail.isTyping) {
      const now = Date.now();
      // Throttle: only send isTyping:true at most once per 500ms
      if (now - lastTypingSentAt >= TYPING_THROTTLE_MS) {
        socket?.send({ type: 'typing', is_typing: true });
        lastTypingSentAt = now;
      }

      // Schedule stop-typing after 2s of no input
      typingTimeout = setTimeout(() => {
        socket?.send({ type: 'typing', is_typing: false });
        lastTypingSentAt = 0;
        typingTimeout = null;
      }, TYPING_STOP_DELAY_MS);
    } else {
      // Explicitly stopped typing (e.g. sent message or cleared input)
      socket?.send({ type: 'typing', is_typing: false });
      lastTypingSentAt = 0;
    }
  }

  function keepVote(detail: { keep: boolean }) {
    socket?.send({ type: 'keep_vote', keep: detail.keep });
  }

  function award(detail: { award_type: string; spark_amount: number }) {
    socket?.send({
      type: 'award',
      award_type: detail.award_type,
      spark_amount: detail.spark_amount
    });
  }

  async function reportPartner() {
    const partner = get(chat).partner;
    const chatId = get(chat).chatId;

    if (!partner?.id || !chatId) {
      toast.error('No active partner to report');
      return;
    }

    const reason = window.prompt('Reason (spam/harassment/underage/illegal/other)', 'spam');
    if (!reason) return;

    const details = window.prompt('Optional details', '') ?? '';

    try {
      await apiFetch('/api/reports', {
        method: 'POST',
        body: JSON.stringify({
          reported_id: partner.id,
          chat_id: chatId,
          reason,
          details
        })
      });
      toast.success('Report submitted');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to report user');
    }
  }

  async function blockPartner() {
    const partner = get(chat).partner;
    if (!partner?.id) {
      toast.error('No active partner to block');
      return;
    }

    try {
      await apiFetch('/api/blocks', {
        method: 'POST',
        body: JSON.stringify({ blocked_id: partner.id })
      });
      toast.success('User blocked');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to block user');
    }
  }

  function leaveChat() {
    if (typingTimeout) {
      clearTimeout(typingTimeout);
      typingTimeout = null;
    }
    lastTypingSentAt = 0;
    socket?.send({ type: 'leave' });
    socket?.close();
    socket = null;
    resetChatState();
  }

  async function nextChat() {
    leaveChat();
    await joinQueue();
  }
</script>

{#if !$auth.user}
  <div class="surface p-6 text-center">
    <p class="text-[var(--text-secondary)]">You need an account to start chatting.</p>
    <a class="btn-primary mt-4" href="/login">Go to login</a>
  </div>
{:else}
  <section class="grid gap-4 lg:grid-cols-[320px,1fr]">
    <div class="space-y-4">
      <MatchmakingPanel
        {categories}
        {languages}
        bind:selectedCategory
        bind:selectedLanguage
        queued={$chat.queued}
        queuePosition={$chat.queuePosition}
        queueWaitSeconds={$chat.queueWaitSeconds}
        onjoin={joinQueue}
        onleave={leaveQueue}
      />

      <ProfileCard
        username={$chat.partner?.username ?? 'Waiting for match'}
        bio={$chat.partner?.bio ?? ''}
        badges={$chat.partner?.badges ?? []}
        interests={$chat.partner?.interests ?? []}
        flare={$chat.partner?.flare ?? {}}
      />

      <div class="surface flex gap-2 p-3">
        <button class="btn-secondary flex-1" onclick={reportPartner} disabled={!$chat.chatId}>Report</button>
        <button class="btn-secondary flex-1" onclick={blockPartner} disabled={!$chat.partner?.id}>Block</button>
      </div>

      <AdBanner visible={!$auth.user.is_premium} />
    </div>

    <ChatWindow
      messages={$chat.messages}
      currentUserId={$auth.user.id}
      partnerTyping={$chat.partnerTyping}
      connected={$chat.chatId !== null}
      {connectionError}
      partnerName={$chat.partner?.username ?? 'stranger'}
      onsend={sendMessage}
      ontyping={sendTyping}
      onleave={leaveChat}
      onnext={nextChat}
      onkeepvote={keepVote}
      onaward={award}
    />
  </section>

  <KeepVoteModal
    open={keepPromptOpen}
    onvote={keepVote}
    onclose={() => (keepPromptOpen = false)}
  />
{/if}
