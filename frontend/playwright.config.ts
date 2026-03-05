import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',

  use: {
    baseURL: 'http://localhost:5173',
    trace: 'on-first-retry',
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],

  webServer: [
    {
      command:
        'cd ../backend && DATABASE_URL=postgres://test:test@localhost:5433/othergirl_test REDIS_URL=redis://127.0.0.1:6380 JWT_SECRET=test-secret-that-is-at-least-32-characters-long CHAT_KEY_ENCRYPTION_KEY_B64=KJbAlRi7wEq2AWH/LpzBMz0fKaM5vjzz1LaE8fH++K0= CORS_ORIGIN=http://localhost:5173 cargo run',
      url: 'http://localhost:8080',
      reuseExistingServer: true,
      timeout: 120_000,
    },
    {
      command: 'bun run dev',
      url: 'http://localhost:5173',
      reuseExistingServer: true,
      timeout: 30_000,
    },
  ],
});
