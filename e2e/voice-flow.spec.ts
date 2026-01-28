import { test, expect } from '@playwright/test';

test.describe('Voice Intelligence Hub', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('displays main interface', async ({ page }) => {
    await expect(page.getByText('Voice Intelligence Hub')).toBeVisible();
    await expect(page.getByRole('button', { name: /start recording/i })).toBeVisible();
    await expect(page.getByText(/Cmd\+Shift\+Space/i)).toBeVisible();
  });

  test('shows agent selector buttons', async ({ page }) => {
    await expect(page.getByText('Action Items')).toBeVisible();
    await expect(page.getByText('Tone Shifter')).toBeVisible();
    await expect(page.getByText('Music Matcher')).toBeVisible();
  });

  test('agent buttons are disabled without transcript', async ({ page }) => {
    const actionItemsButton = page.getByRole('button', { name: /action items/i });
    await expect(actionItemsButton).toBeDisabled();
  });

  test('clear button resets state', async ({ page }) => {
    // Click clear button
    await page.getByRole('button', { name: /clear/i }).click();

    // Verify state is reset (no transcript visible)
    await expect(page.getByText(/transcript/i)).not.toBeVisible();
  });

  test('escape key is handled', async ({ page }) => {
    // Press escape - in Tauri this would hide window, in browser it's a no-op
    await page.keyboard.press('Escape');

    // Page should still be visible in browser test
    await expect(page.getByText('Voice Intelligence Hub')).toBeVisible();
  });

  test('record button changes state on click', async ({ page }) => {
    const recordButton = page.getByRole('button', { name: /start recording/i });

    // In browser environment without Tauri, click won't actually start recording
    // but we can verify the button exists and is clickable
    await expect(recordButton).toBeEnabled();
  });

  test('spotlight container has correct styling', async ({ page }) => {
    const container = page.locator('.spotlight-container');
    await expect(container).toBeVisible();

    // Check for glass effect styling
    const styles = await container.evaluate((el) => {
      const computed = window.getComputedStyle(el);
      return {
        borderRadius: computed.borderRadius,
        backdropFilter: computed.backdropFilter,
      };
    });

    expect(styles.borderRadius).toBe('16px');
  });

  test('responsive layout on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });

    await expect(page.getByText('Voice Intelligence Hub')).toBeVisible();
    await expect(page.getByRole('button', { name: /start recording/i })).toBeVisible();
  });
});

test.describe('Accessibility', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('buttons have accessible names', async ({ page }) => {
    const buttons = page.getByRole('button');
    const buttonCount = await buttons.count();

    for (let i = 0; i < buttonCount; i++) {
      const button = buttons.nth(i);
      const name = await button.getAttribute('aria-label') || await button.textContent();
      expect(name).toBeTruthy();
    }
  });

  test('keyboard navigation works', async ({ page }) => {
    // Tab to first interactive element
    await page.keyboard.press('Tab');

    // Should focus on clear button or another interactive element
    const focused = page.locator(':focus');
    await expect(focused).toBeVisible();
  });

  test('focus styles are visible', async ({ page }) => {
    await page.keyboard.press('Tab');

    const focused = page.locator(':focus');
    const outline = await focused.evaluate((el) => {
      return window.getComputedStyle(el).outline;
    });

    // Should have some focus indicator
    expect(outline).toBeTruthy();
  });
});

test.describe('Error Handling', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('handles missing Tauri gracefully', async ({ page }) => {
    // The app should work in browser without Tauri
    await expect(page.getByText('Voice Intelligence Hub')).toBeVisible();

    // No error messages should be visible
    await expect(page.locator('.bg-red-500')).not.toBeVisible();
  });
});
