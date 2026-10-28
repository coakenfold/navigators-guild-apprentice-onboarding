// @ts-check
import { test, expect } from '@playwright/test';

const MDN = {
  url: 'https://developer.mozilla.org',
  title: 'MDN Web Docs',
  notes: 'Great reference for HTML and CSS.',
  tags: 'JavaScript, reference , javascript',
};
const EXAMPLE = {
  url: 'https://example.com/pasta',
  title: 'Easy Weeknight Pasta',
  notes: '',
  tags: 'recipes, dinner',
};

/** @param {import('@playwright/test').Page} page */
async function addBookmark(page, { url, title, notes, tags }) {
  await page.getByLabel('URL').fill(url);
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Notes').fill(notes);
  await page.getByLabel('Tags').fill(tags);
  await page.getByRole('button', { name: 'Save' }).click();
}

/** @param {import('@playwright/test').Page} page */
const storedBookmarks = (page) =>
  page.evaluate(() => JSON.parse(localStorage.getItem('bookmarks') ?? 'null'));

test.describe('Add & list', () => {
  /** @type {string[]} */
  let consoleErrors;

  test.beforeEach(async ({ page }) => {
    consoleErrors = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });
    page.on('pageerror', (err) => consoleErrors.push(err.message));
    await page.goto('/');
  });

  test('first load shows the empty state and no console errors', async ({ page }) => {
    await expect(
      page.getByText('No bookmarks yet. Add your first one above.')
    ).toBeVisible();
    await expect(page.locator('.bookmark')).toHaveCount(0);
    expect(consoleErrors).toEqual([]);
  });

  test('adding a bookmark lists it with cleaned tags and clears the form', async ({ page }) => {
    await addBookmark(page, MDN);

    const items = page.locator('.bookmark');
    await expect(items).toHaveCount(1);
    await expect(items.first().locator('.bookmark-title')).toHaveText(MDN.title);
    await expect(items.first().locator('.bookmark-url')).toHaveText(MDN.url);
    await expect(items.first().locator('.bookmark-notes')).toHaveText(MDN.notes);
    await expect(items.first().locator('.tag')).toHaveText(['javascript', 'reference']);
    await expect(items.first().locator('.bookmark-date')).toHaveText(
      /^[A-Z][a-z]{2} \d{1,2}, \d{4}$/
    );
    await expect(page.getByText('No bookmarks yet.')).toBeHidden();

    for (const label of ['URL', 'Title', 'Notes', 'Tags']) {
      await expect(page.getByLabel(label)).toHaveValue('');
    }
    expect(consoleErrors).toEqual([]);
  });

  test('a second bookmark appears above the first', async ({ page }) => {
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);

    await expect(page.locator('.bookmark .bookmark-title')).toHaveText([
      EXAMPLE.title,
      MDN.title,
    ]);
  });

  test('bookmarks persist across a refresh in the same order', async ({ page }) => {
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);

    await page.reload();

    await expect(page.locator('.bookmark .bookmark-title')).toHaveText([
      EXAMPLE.title,
      MDN.title,
    ]);
    await expect(page.locator('.bookmark').nth(1).locator('.tag')).toHaveText([
      'javascript',
      'reference',
    ]);
  });

  test('an ftp:// URL shows an error and saves nothing', async ({ page }) => {
    await addBookmark(page, { ...MDN, url: 'ftp://x.com' });

    await expect(page.getByRole('alert')).toContainText('http://');
    await expect(page.locator('.bookmark')).toHaveCount(0);
    await expect(page.getByText('No bookmarks yet.')).toBeVisible();
    expect(await storedBookmarks(page)).toBeNull();
  });

  test('an empty title shows an error and saves nothing', async ({ page }) => {
    await addBookmark(page, { ...MDN, title: '' });

    await expect(page.getByRole('alert')).toBeVisible();
    await expect(page.locator('.bookmark')).toHaveCount(0);
    expect(await storedBookmarks(page)).toBeNull();
  });

  test('a missing URL shows an error and saves nothing', async ({ page }) => {
    await addBookmark(page, { ...MDN, url: '' });

    await expect(page.getByRole('alert')).toBeVisible();
    expect(await storedBookmarks(page)).toBeNull();
  });

  test('clicking a title opens the link in a new tab', async ({ page, context }) => {
    // Stub the external site so the test does not need network access.
    await context.route('https://developer.mozilla.org/**', (route) =>
      route.fulfill({ contentType: 'text/html', body: '<title>MDN stub</title>' })
    );
    await addBookmark(page, MDN);

    const link = page.getByRole('link', { name: MDN.title });
    await expect(link).toHaveAttribute('target', '_blank');
    await expect(link).toHaveAttribute('rel', /noopener/);

    const popupPromise = context.waitForEvent('page');
    await link.click();
    const popup = await popupPromise;
    await popup.waitForLoadState();

    expect(popup.url()).toMatch(/^https:\/\/developer\.mozilla\.org\/?$/);
    expect(page.url()).toBe('http://localhost:3000/');
  });

  test('localStorage.bookmarks matches the data model', async ({ page }) => {
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);

    const stored = await storedBookmarks(page);

    expect(stored).toHaveLength(2);
    for (const bookmark of stored) {
      expect(Object.keys(bookmark).sort()).toEqual(
        ['createdAt', 'id', 'notes', 'tags', 'title', 'url']
      );
      expect(bookmark.id).toMatch(/^\d+$/);
      expect(new Date(bookmark.createdAt).toISOString()).toBe(bookmark.createdAt);
      expect(Array.isArray(bookmark.tags)).toBe(true);
    }

    const mdn = stored.find((b) => b.title === MDN.title);
    expect(mdn).toMatchObject({
      url: MDN.url,
      notes: MDN.notes,
      tags: ['javascript', 'reference'],
    });
    expect(new Set(stored.map((b) => b.id)).size).toBe(2);
  });
});
