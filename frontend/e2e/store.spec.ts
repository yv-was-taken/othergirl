import { test, expect } from '@playwright/test';

const ts = Date.now();

/**
 * Helper: register a fresh user and return to the desired page while logged in.
 */
async function registerAndNavigate(page: import('@playwright/test').Page, targetPath: string) {
  const user = {
    username: `store${ts}${Math.random().toString(36).slice(2, 6)}`,
    email: `store${ts}${Math.random().toString(36).slice(2, 6)}@example.com`,
    password: 'TestPassword123!',
  };

  await page.goto('/login');
  await page.evaluate(() => localStorage.removeItem('othergirl.session'));

  await page.getByRole('button', { name: 'Register', exact: true }).click();
  await page.locator('#username-input').fill(user.username);
  await page.locator('#email-input').fill(user.email);
  await page.locator('#password-input').fill(user.password);
  await page.getByLabel('I confirm I am 18+').check();
  await page.getByRole('button', { name: 'Create account' }).click();
  await expect(page).not.toHaveURL(/\/login/);

  await page.goto(targetPath);
  return user;
}

test.describe('Store page', () => {
  test('store page loads when authenticated', async ({ page }) => {
    await registerAndNavigate(page, '/store');

    // The heading "Flare Store" should be visible.
    await expect(page.getByRole('heading', { name: 'Flare Store' })).toBeVisible();
  });

  test('spark balance is shown for authenticated users', async ({ page }) => {
    await registerAndNavigate(page, '/store');

    // The balance badge shows "<number> sparks".
    await expect(page.getByText(/sparks/)).toBeVisible();
  });

  test('balance is NOT shown for unauthenticated users on /store', async ({ page }) => {
    // Clear session.
    await page.goto('/login');
    await page.evaluate(() => localStorage.removeItem('othergirl.session'));

    // Navigate to /store — since it is under (protected), user will be redirected to /login.
    await page.goto('/store');

    // We should be on /login now.
    await expect(page).toHaveURL(/\/login/);

    // The "sparks" balance badge should not be visible anywhere on the login page.
    await expect(page.getByText(/sparks/)).not.toBeVisible();
  });
});
