export type EmoteEntry = {
  token: string;
  name: string;
  imageUrl: string;
};

const EMOTES: EmoteEntry[] = [
  // Faces
  { token: ':smile:', name: 'Smile', imageUrl: '/emotes/smile.svg' },
  { token: ':grin:', name: 'Grin', imageUrl: '/emotes/grin.svg' },
  { token: ':laugh:', name: 'Laugh', imageUrl: '/emotes/laugh.svg' },
  { token: ':rofl:', name: 'ROFL', imageUrl: '/emotes/rofl.svg' },
  { token: ':wink:', name: 'Wink', imageUrl: '/emotes/wink.svg' },
  { token: ':blush:', name: 'Blush', imageUrl: '/emotes/blush.svg' },
  { token: ':heart_eyes:', name: 'Heart Eyes', imageUrl: '/emotes/heart_eyes.svg' },
  { token: ':kiss:', name: 'Kiss', imageUrl: '/emotes/kiss.svg' },
  { token: ':thinking:', name: 'Thinking', imageUrl: '/emotes/thinking.svg' },
  { token: ':neutral:', name: 'Neutral', imageUrl: '/emotes/neutral.svg' },
  { token: ':unamused:', name: 'Unamused', imageUrl: '/emotes/unamused.svg' },
  { token: ':rolling_eyes:', name: 'Rolling Eyes', imageUrl: '/emotes/rolling_eyes.svg' },
  { token: ':cry:', name: 'Cry', imageUrl: '/emotes/cry.svg' },
  { token: ':sob:', name: 'Sob', imageUrl: '/emotes/sob.svg' },
  { token: ':angry:', name: 'Angry', imageUrl: '/emotes/angry.svg' },
  { token: ':rage:', name: 'Rage', imageUrl: '/emotes/rage.svg' },
  { token: ':scream:', name: 'Scream', imageUrl: '/emotes/scream.svg' },
  { token: ':skull:', name: 'Skull', imageUrl: '/emotes/skull.svg' },
  { token: ':sunglasses:', name: 'Sunglasses', imageUrl: '/emotes/sunglasses.svg' },
  { token: ':nerd:', name: 'Nerd', imageUrl: '/emotes/nerd.svg' },
  // Gestures
  { token: ':wave:', name: 'Wave', imageUrl: '/emotes/wave.svg' },
  { token: ':thumbsup:', name: 'Thumbs Up', imageUrl: '/emotes/thumbsup.svg' },
  { token: ':thumbsdown:', name: 'Thumbs Down', imageUrl: '/emotes/thumbsdown.svg' },
  { token: ':clap:', name: 'Clap', imageUrl: '/emotes/clap.svg' },
  { token: ':pray:', name: 'Pray', imageUrl: '/emotes/pray.svg' },
  { token: ':muscle:', name: 'Muscle', imageUrl: '/emotes/muscle.svg' },
  { token: ':ok_hand:', name: 'OK Hand', imageUrl: '/emotes/ok_hand.svg' },
  { token: ':peace:', name: 'Peace', imageUrl: '/emotes/peace.svg' },
  // Hearts & Symbols
  { token: ':heart:', name: 'Heart', imageUrl: '/emotes/heart.svg' },
  { token: ':broken_heart:', name: 'Broken Heart', imageUrl: '/emotes/broken_heart.svg' },
  { token: ':purple_heart:', name: 'Purple Heart', imageUrl: '/emotes/purple_heart.svg' },
  { token: ':blue_heart:', name: 'Blue Heart', imageUrl: '/emotes/blue_heart.svg' },
  { token: ':spark:', name: 'Spark', imageUrl: '/emotes/spark.svg' },
  { token: ':star:', name: 'Star', imageUrl: '/emotes/star.svg' },
  { token: ':fire:', name: 'Fire', imageUrl: '/emotes/fire.svg' },
  { token: ':100:', name: 'Hundred', imageUrl: '/emotes/100.svg' },
  // Reactions & Objects
  { token: ':party:', name: 'Party', imageUrl: '/emotes/party.svg' },
  { token: ':confetti:', name: 'Confetti', imageUrl: '/emotes/confetti.svg' },
  { token: ':eyes:', name: 'Eyes', imageUrl: '/emotes/eyes.svg' },
  { token: ':tada:', name: 'Tada', imageUrl: '/emotes/tada.svg' },
  { token: ':trophy:', name: 'Trophy', imageUrl: '/emotes/trophy.svg' },
  { token: ':crown:', name: 'Crown', imageUrl: '/emotes/crown.svg' },
  { token: ':rocket:', name: 'Rocket', imageUrl: '/emotes/rocket.svg' },
  { token: ':rainbow:', name: 'Rainbow', imageUrl: '/emotes/rainbow.svg' },
  { token: ':ghost:', name: 'Ghost', imageUrl: '/emotes/ghost.svg' },
  { token: ':poop:', name: 'Poop', imageUrl: '/emotes/poop.svg' },
  { token: ':money:', name: 'Money', imageUrl: '/emotes/money.svg' },
  { token: ':gift:', name: 'Gift', imageUrl: '/emotes/gift.svg' },
  { token: ':lightning:', name: 'Lightning', imageUrl: '/emotes/lightning.svg' },
  { token: ':check:', name: 'Check', imageUrl: '/emotes/check.svg' }
];

const emoteMap = new Map(EMOTES.map((e) => [e.token, e.imageUrl]));

export function getEmoteList(): readonly EmoteEntry[] {
  return EMOTES;
}

export function applyEmotes(input: string): string {
  let result = input;
  for (const [token, url] of emoteMap) {
    result = result.replaceAll(
      token,
      `<img src="${url}" alt="${token}" class="inline-block h-5 w-5 align-middle" />`
    );
  }
  return result;
}
