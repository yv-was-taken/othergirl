import { test, expect } from '@playwright/test';

test.describe('Theme', () => {
  test.beforeEach(async ({ page }) => {
    // Clear theme preference so we get the default.
    await page.goto('/');
    await page.evaluate(() => localStorage.removeItem('othergirl.theme'));
  });

  test('default theme is dark', async ({ page }) => {
    // Clear and reload to get true default.
    await page.evaluate(() => localStorage.removeItem('othergirl.theme'));
    await page.reload();

    const theme = await page.locator('html').getAttribute('data-theme');
    expect(theme).toBe('dark');
  });

  test('toggle theme switches between dark and light', async ({ page }) => {
    await page.evaluate(() => localStorage.removeItem('othergirl.theme'));
    await page.reload();

    // Should start dark.
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');

    // Click the theme toggle button.
    await page.getByRole('button', { name: 'Toggle theme' }).click();

    // Should now be light.
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');

    // Toggle back.
    await page.getByRole('button', { name: 'Toggle theme' }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  });

  test('theme persists across reload via localStorage', async ({ page }) => {
    await page.evaluate(() => localStorage.removeItem('othergirl.theme'));
    await page.reload();

    // Start dark, toggle to light.
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    await page.getByRole('button', { name: 'Toggle theme' }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');

    // Verify localStorage was set.
    const stored = await page.evaluate(() => localStorage.getItem('othergirl.theme'));
    expect(stored).toBe('light');

    // Reload and verify it is still light.
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  });
});
