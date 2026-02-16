import { redirect } from '@sveltejs/kit';

const SESSION_KEY = 'othergirl.session';

export const ssr = false;

export const load = ({ url }: { url: URL }) => {
  const raw = localStorage.getItem(SESSION_KEY);

  if (!raw || !hasValidSession(raw)) {
    const next = encodeURIComponent(`${url.pathname}${url.search}`);
    throw redirect(307, `/login?next=${next}`);
  }
};

function hasValidSession(raw: string): boolean {
  try {
    const session = JSON.parse(raw) as {
      token?: string;
      user?: Record<string, unknown> | null;
    };

    return Boolean(session.token && session.user);
  } catch {
    return false;
  }
}
