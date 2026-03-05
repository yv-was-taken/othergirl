import { test, expect } from '@playwright/test';

// Generate a unique user per test run so parallel / repeated runs don't collide.
const ts = Date.now();
const TEST_USER = {
  username: `testuser${ts}`,
  email: `testuser${ts}@example.com`,
  password: 'TestPassword123!',
};

test.describe('Authentication flow', () => {
  test.beforeEach(async ({ page }) => {
    // Clear any existing session so each test starts logged-out.
    await page.goto('/login');
    await page.evaluate(() => localStorage.removeItem('othergirl.session'));
  });

  test('login page loads', async ({ page }) => {
    await page.goto('/login');

    // The page should show both Login and Register mode buttons.
    await expect(page.getByRole('button', { name: 'Login', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Register', exact: true })).toBeVisible();

    // Email and password fields should be visible in login mode.
    await expect(page.locator('#email-input')).toBeVisible();
    await expect(page.locator('#password-input')).toBeVisible();

    // Username field should NOT be visible in login mode.
    await expect(page.locator('#username-input')).not.toBeVisible();
  });

  test('register a new user', async ({ page }) => {
    await page.goto('/login');

    // Switch to register mode.
    await page.getByRole('button', { name: 'Register', exact: true }).click();

    // Username field should now be visible.
    await expect(page.locator('#username-input')).toBeVisible();

    // Fill in the registration form.
    await page.locator('#username-input').fill(TEST_USER.username);
    await page.locator('#email-input').fill(TEST_USER.email);
    await page.locator('#password-input').fill(TEST_USER.password);

    // Check the age-verification checkbox.
    await page.getByLabel('I confirm I am 18+').check();

    // Submit the form.
    await page.getByRole('button', { name: 'Create account' }).click();

    // After successful registration, the user should be redirected away from /login.
    await expect(page).not.toHaveURL(/\/login/);

    // The navbar should show the username, confirming we are logged in.
    await expect(page.getByText(TEST_USER.username)).toBeVisible();
  });

  test('log out', async ({ page }) => {
    // First, register / log in.
    await page.goto('/login');
    await page.getByRole('button', { name: 'Register', exact: true }).click();

    const user = {
      username: `logout${ts}`,
      email: `logout${ts}@example.com`,
      password: 'TestPassword123!',
    };

    await page.locator('#username-input').fill(user.username);
    await page.locator('#email-input').fill(user.email);
    await page.locator('#password-input').fill(user.password);
    await page.getByLabel('I confirm I am 18+').check();
    await page.getByRole('button', { name: 'Create account' }).click();

    // Wait until redirected away from /login.
    await expect(page).not.toHaveURL(/\/login/);

    // Click the "Log out" button in the navbar.
    await page.getByRole('button', { name: 'Log out' }).click();

    // Should be back on /login.
    await expect(page).toHaveURL(/\/login/);
  });

  test('log back in with same credentials', async ({ page }) => {
    // Register a fresh user first.
    await page.goto('/login');
    await page.getByRole('button', { name: 'Register', exact: true }).click();

    const user = {
      username: `relogin${ts}`,
      email: `relogin${ts}@example.com`,
      password: 'TestPassword123!',
    };

    await page.locator('#username-input').fill(user.username);
    await page.locator('#email-input').fill(user.email);
    await page.locator('#password-input').fill(user.password);
    await page.getByLabel('I confirm I am 18+').check();
    await page.getByRole('button', { name: 'Create account' }).click();
    await expect(page).not.toHaveURL(/\/login/);

    // Log out.
    await page.getByRole('button', { name: 'Log out' }).click();
    await expect(page).toHaveURL(/\/login/);

    // Now log back in — should be in login mode by default.
    await page.locator('#email-input').fill(user.email);
    await page.locator('#password-input').fill(user.password);
    await page.getByRole('button', { name: 'Login', exact: true }).first().click();

    // Wait for submit button text (it doubles as mode button, so target the submit).
    await expect(page).not.toHaveURL(/\/login/);
    await expect(page.getByText(user.username)).toBeVisible();
  });

  test('login with wrong password shows error', async ({ page }) => {
    // Register a user so the email exists.
    await page.goto('/login');
    await page.getByRole('button', { name: 'Register', exact: true }).click();

    const user = {
      username: `badpw${ts}`,
      email: `badpw${ts}@example.com`,
      password: 'TestPassword123!',
    };

    await page.locator('#username-input').fill(user.username);
    await page.locator('#email-input').fill(user.email);
    await page.locator('#password-input').fill(user.password);
    await page.getByLabel('I confirm I am 18+').check();
    await page.getByRole('button', { name: 'Create account' }).click();
    await expect(page).not.toHaveURL(/\/login/);

    // Log out.
    await page.getByRole('button', { name: 'Log out' }).click();
    await expect(page).toHaveURL(/\/login/);

    // Attempt login with wrong password.
    await page.locator('#email-input').fill(user.email);
    await page.locator('#password-input').fill('WrongPassword999!');

    // The submit button in login mode says "Login" — it is the <button type="submit">.
    await page.getByRole('button', { name: 'Login', exact: true }).first().click();

    // Should stay on /login and show a toast error.
    await expect(page).toHaveURL(/\/login/);

    // svelte-sonner renders toasts as <li> inside an <ol> with data-sonner-toaster.
    // We just check that some error-ish text appears on the page.
    await expect(page.locator('[data-sonner-toast]')).toBeVisible({ timeout: 5000 });
  });
});
