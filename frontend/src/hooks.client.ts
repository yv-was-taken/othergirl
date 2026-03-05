import * as Sentry from '@sentry/sveltekit';

const dsn = import.meta.env.VITE_SENTRY_DSN;

if (dsn) {
  Sentry.init({
    dsn,
    tracesSampleRate: 0.1,
    replaysSessionSampleRate: 0,
    replaysOnErrorSampleRate: 1.0
  });
}

export const handleError = Sentry.handleErrorWithSentry();
