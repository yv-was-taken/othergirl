import { test, expect } from '@playwright/test';

test.describe('Navigation', () => {
  test('navbar links are visible on desktop viewport', async ({ page }) => {
    await page.goto('/');

    const navLinks = ['Home', 'Chat', 'History', 'Store', 'Settings'];

    for (const label of navLinks) {
      await expect(page.locator('nav').getByRole('link', { name: label })).toBeVisible();
    }
  });

  test('hamburger menu works on mobile viewport', async ({ page }) => {
    // Set a mobile viewport.
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/');

    // Desktop nav links should be hidden on mobile.
    // The desktop links container has class md:flex and is hidden by default.
    // Check that the first nav link is not visible initially.
    const desktopNav = page.locator('nav .hidden.md\\:flex');
    await expect(desktopNav).not.toBeVisible();

    // The hamburger button should be visible.
    const hamburger = page.getByRole('button', { name: 'Toggle navigation menu' });
    await expect(hamburger).toBeVisible();

    // Click the hamburger to open mobile menu.
    await hamburger.click();

    // Mobile dropdown should now show all links.
    const mobileMenu = page.locator('nav').locator('.md\\:hidden').last();
    await expect(mobileMenu).toBeVisible();

    const navLinks = ['Home', 'Chat', 'History', 'Store', 'Settings'];
    for (const label of navLinks) {
      await expect(mobileMenu.getByRole('link', { name: label })).toBeVisible();
    }

    // Click hamburger again to close.
    await hamburger.click();
    await expect(mobileMenu).not.toBeVisible();
  });

  test('active link highlighting works', async ({ page }) => {
    await page.goto('/');

    // The "Home" link should have the accent border class when on /.
    const homeLink = page.locator('nav').getByRole('link', { name: 'Home' }).first();
    await expect(homeLink).toHaveClass(/border-\[var\(--accent\)\]/);

    // Navigate to /store (protected, will redirect to /login).
    // Instead test a non-protected page — / is already tested.
    // Let's just verify a different link does NOT have the accent class.
    const storeLink = page.locator('nav').getByRole('link', { name: 'Store' }).first();
    await expect(storeLink).not.toHaveClass(/border-\[var\(--accent\)\]/);
  });

  test('unauthenticated users are redirected to /login from protected pages', async ({ page }) => {
    // Clear any session.
    await page.goto('/login');
    await page.evaluate(() => localStorage.removeItem('othergirl.session'));

    // Try navigating to a protected page.
    await page.goto('/chat');

    // Should be redirected to /login with a next parameter.
    await expect(page).toHaveURL(/\/login/);
    await expect(page).toHaveURL(/next=/);
  });
});
