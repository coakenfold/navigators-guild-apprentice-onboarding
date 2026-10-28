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

test.describe('Delete', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);
  });

  test('cancelling the dialog leaves the bookmark in place', async ({ page }) => {
    /** @type {string[]} */
    const messages = [];
    page.once('dialog', (dialog) => {
      messages.push(dialog.message());
      dialog.dismiss();
    });
    await page.getByRole('button', { name: `Delete ${EXAMPLE.title}` }).click();

    expect(messages).toEqual(['Are you sure?']);
    await expect(page.locator('.bookmark')).toHaveCount(2);
    expect(await storedBookmarks(page)).toHaveLength(2);
  });

  test('confirming removes it, and it stays gone after a refresh', async ({ page }) => {
    page.once('dialog', (dialog) => dialog.accept());
    await page.getByRole('button', { name: `Delete ${EXAMPLE.title}` }).click();

    await expect(page.locator('.bookmark .bookmark-title')).toHaveText([MDN.title]);

    await page.reload();
    await expect(page.locator('.bookmark .bookmark-title')).toHaveText([MDN.title]);
    expect(await storedBookmarks(page)).toHaveLength(1);
  });

  test('deleting every bookmark brings back the empty-state message', async ({ page }) => {
    page.on('dialog', (dialog) => dialog.accept());
    await page.getByRole('button', { name: `Delete ${EXAMPLE.title}` }).click();
    await page.getByRole('button', { name: `Delete ${MDN.title}` }).click();

    await expect(page.locator('.bookmark')).toHaveCount(0);
    await expect(
      page.getByText('No bookmarks yet. Add your first one above.')
    ).toBeVisible();
    expect(await storedBookmarks(page)).toEqual([]);
  });
});

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

test.describe('Edit', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);
  });

  test('updates in place, keeping id, createdAt and position, and survives refresh', async ({ page }) => {
    const before = await storedBookmarks(page);
    const original = before.find((b) => b.title === MDN.title);

    await page.getByRole('button', { name: `Edit ${MDN.title}` }).click();
    await expect(page.getByLabel('Tags')).toHaveValue('javascript, reference');
    await expect(page.getByRole('button', { name: 'Update' })).toBeVisible();

    await page.getByLabel('Title').fill('MDN Renamed');
    await page.getByLabel('Tags').fill('Docs, HTML');
    await page.getByRole('button', { name: 'Update' }).click();

    await page.reload();
    const after = await storedBookmarks(page);
    expect(after).toHaveLength(2);
    const edited = after.find((b) => b.id === original.id);
    expect(edited).toMatchObject({
      title: 'MDN Renamed',
      tags: ['docs', 'html'],
      createdAt: original.createdAt,
    });
    await expect(page.locator('.bookmark')).toHaveCount(2);
  });

  test('rejects invalid input, and a later submit creates a new bookmark', async ({ page }) => {
    await page.getByRole('button', { name: `Edit ${MDN.title}` }).click();
    await page.getByLabel('Title').fill('');
    await page.getByRole('button', { name: 'Update' }).click();
    await expect(page.getByRole('alert')).toBeVisible();
    expect((await storedBookmarks(page))[0].title).toBe(MDN.title);

    await page.getByLabel('Title').fill('Changed');
    await page.getByRole('button', { name: 'Update' }).click();
    await expect(page.getByRole('button', { name: 'Save' })).toBeVisible();
    await expect(page.getByLabel('Title')).toHaveValue('');

    await addBookmark(page, { url: 'https://example.org', title: 'Third', notes: '', tags: '' });
    expect(await storedBookmarks(page)).toHaveLength(3);
  });

  test('deleting the bookmark mid-edit resets to add mode', async ({ page }) => {
    page.on('dialog', (dialog) => dialog.accept());
    await page.getByRole('button', { name: `Edit ${MDN.title}` }).click();
    await page.getByRole('button', { name: `Delete ${MDN.title}` }).click();
    await expect(page.getByRole('button', { name: 'Save' })).toBeVisible();
    await expect(page.getByLabel('Title')).toHaveValue('');

    const errors = [];
    page.on('pageerror', (e) => errors.push(e));
    await addBookmark(page, { url: 'https://example.org', title: 'Third', notes: '', tags: '' });
    expect(errors).toHaveLength(0);
    expect(await storedBookmarks(page)).toHaveLength(2);
  });
});

test.describe('Live search', () => {
  const titles = (/** @type {import('@playwright/test').Page} */ page) =>
    page.locator('.bookmark .bookmark-title');
  const search = (/** @type {import('@playwright/test').Page} */ page) =>
    page.getByLabel('Search bookmarks');
  const NO_MATCH = 'No bookmarks match your search.';
  const NO_BOOKMARKS = 'No bookmarks yet. Add your first one above.';

  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);
  });

  test('finds the MDN bookmark by title, URL or notes, ignoring case', async ({ page }) => {
    for (const query of ['mdn', 'MOZILLA', 'reference for html']) {
      await search(page).fill(query);
      await expect(titles(page)).toHaveText([MDN.title]);
    }
  });

  test('updates on every keystroke', async ({ page }) => {
    await search(page).pressSequentially('pas');
    await expect(titles(page)).toHaveText([EXAMPLE.title]);
    await search(page).pressSequentially('ta');
    await expect(titles(page)).toHaveText([EXAMPLE.title]);
    await search(page).pressSequentially('zzz');
    await expect(page.getByText(NO_MATCH)).toBeVisible();
  });

  test('does not match against tags', async ({ page }) => {
    await search(page).fill('dinner');
    await expect(titles(page)).toHaveCount(0);
    await expect(page.getByText(NO_MATCH)).toBeVisible();
  });

  test('a search with no results shows the no-match message', async ({ page }) => {
    await search(page).fill('zzzz');
    await expect(titles(page)).toHaveCount(0);
    await expect(page.getByText(NO_MATCH)).toBeVisible();
    await expect(page.getByText(NO_BOOKMARKS)).toBeHidden();
  });

  test('clearing the box shows everything again', async ({ page }) => {
    await search(page).fill('zzzz');
    await search(page).fill('');
    await expect(titles(page)).toHaveCount(2);
    await expect(page.getByText(NO_MATCH)).toBeHidden();
  });

  test('searches within the active tag filter', async ({ page }) => {
    await addBookmark(page, {
      url: 'https://example.com/soup',
      title: 'Soup Basics',
      notes: '',
      tags: 'recipes',
    });
    await page.locator('#tag-filter').getByRole('button', { name: 'recipes' }).click();
    await expect(titles(page)).toHaveCount(2);

    await search(page).fill('example.com');
    await expect(titles(page)).toHaveCount(2);

    await search(page).fill('soup');
    await expect(titles(page)).toHaveText(['Soup Basics']);

    // MDN matches the text but is outside the recipes filter.
    await search(page).fill('mdn');
    await expect(page.getByText(NO_MATCH)).toBeVisible();
  });

  test('with zero bookmarks the no-bookmarks message still shows', async ({ page }) => {
    await page.evaluate(() => localStorage.clear());
    await page.reload();
    await expect(page.getByText(NO_BOOKMARKS)).toBeVisible();

    await search(page).fill('anything');
    await expect(page.getByText(NO_BOOKMARKS)).toBeVisible();
    await expect(page.getByText(NO_MATCH)).toBeHidden();
  });
});
