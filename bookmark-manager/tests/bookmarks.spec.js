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

/** The add form starts collapsed; open it if it is not already. */
async function openForm(page) {
  const toggle = page.getByRole('button', { name: 'Add a bookmark', exact: true });
  if (await toggle.isVisible()) await toggle.click();
}

/** @param {import('@playwright/test').Page} page */
async function addBookmark(page, { url, title, notes, tags }) {
  await openForm(page);
  await page.getByLabel('URL').fill(url);
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Notes').fill(notes);
  await page.getByLabel('Tags').fill(tags);
  await page.getByRole('button', { name: 'Save', exact: true }).click();
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
    await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeVisible();

    await page.getByLabel('Title').fill('MDN Renamed');
    await page.getByLabel('Tags').fill('Docs, HTML');
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();

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
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.getByRole('alert')).toBeVisible();
    expect((await storedBookmarks(page))[0].title).toBe(MDN.title);

    await page.getByLabel('Title').fill('Changed');
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await openForm(page);
    await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeVisible();
    await expect(page.getByLabel('Title')).toHaveValue('');

    await addBookmark(page, { url: 'https://example.org', title: 'Third', notes: '', tags: '' });
    expect(await storedBookmarks(page)).toHaveLength(3);
  });

  test('deleting the bookmark mid-edit resets to add mode', async ({ page }) => {
    page.on('dialog', (dialog) => dialog.accept());
    await page.getByRole('button', { name: `Edit ${MDN.title}` }).click();
    await page.getByRole('button', { name: `Delete ${MDN.title}` }).click();
    await openForm(page);
    await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeVisible();
    await expect(page.getByLabel('Title')).toHaveValue('');

    const errors = [];
    page.on('pageerror', (e) => errors.push(e));
    await addBookmark(page, { url: 'https://example.org', title: 'Third', notes: '', tags: '' });
    expect(errors).toHaveLength(0);
    expect(await storedBookmarks(page)).toHaveLength(2);
  });
});

test.describe('Edit mode names', () => {
  /** @type {string[]} */
  let consoleErrors;
  const toggle = (/** @type {import('@playwright/test').Page} */ page) => page.locator('#form-toggle');
  const button = (/** @type {import('@playwright/test').Page} */ page, name) =>
    page.getByRole('button', { name, exact: true });

  /** Opening the form in add mode shows the add-mode names. */
  async function expectAddMode(page) {
    await expect(page.getByRole('form', { name: 'Add a bookmark' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Add a bookmark', exact: true })).toBeVisible();
    await expect(button(page, 'Save')).toBeVisible();
    await expect(button(page, 'Cancel')).toBeVisible();
    await expect(button(page, 'Save changes')).toHaveCount(0);
    await expect(button(page, 'Discard changes')).toHaveCount(0);
    await expect(page.getByLabel('Title')).toHaveValue('');
  }

  test.beforeEach(async ({ page }) => {
    consoleErrors = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });
    page.on('pageerror', (err) => consoleErrors.push(err.message));
    await page.goto('/');
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);
  });

  test.afterEach(() => {
    expect(consoleErrors).toEqual([]);
  });

  test('opening the form shows add-mode names and a "Close form" toggle', async ({ page }) => {
    await expect(toggle(page)).toHaveAccessibleName('Add a bookmark');
    await expect(toggle(page)).toHaveAttribute('aria-expanded', 'false');
    await openForm(page);
    await expectAddMode(page);
    await expect(toggle(page)).toHaveAccessibleName('Close form');
    await expect(toggle(page)).toHaveAttribute('aria-expanded', 'true');
  });

  test('editing names the form after the bookmark and renames the buttons', async ({ page }) => {
    await button(page, `Edit ${MDN.title}`).click();

    await expect(page.getByRole('form', { name: `Edit bookmark: ${MDN.title}` })).toBeVisible();
    await expect(page.locator('#form-heading')).toHaveText(`Edit bookmark: ${MDN.title}`);
    await expect(toggle(page)).toHaveAccessibleName('Close form');
    await expect(toggle(page)).toHaveAttribute('aria-expanded', 'true');
    await expect(button(page, 'Save changes')).toBeVisible();
    await expect(button(page, 'Discard changes')).toBeVisible();
    await expect(button(page, 'Save')).toHaveCount(0);
    await expect(button(page, 'Cancel')).toHaveCount(0);
  });

  test('the heading keeps the title from when the edit started', async ({ page }) => {
    await button(page, `Edit ${MDN.title}`).click();
    await page.getByLabel('Title').fill('Something else');
    await expect(page.locator('#form-heading')).toHaveText(`Edit bookmark: ${MDN.title}`);
  });

  test('editing a second bookmark switches the heading to its title', async ({ page }) => {
    await button(page, `Edit ${MDN.title}`).click();
    await button(page, `Edit ${EXAMPLE.title}`).click();
    await expect(page.getByRole('form', { name: `Edit bookmark: ${EXAMPLE.title}` })).toBeVisible();
    await expect(button(page, 'Save changes')).toBeVisible();
  });

  /** @type {[string, (page: import('@playwright/test').Page) => Promise<void>][]} */
  const exits = [
    ['saving', (page) => button(page, 'Save changes').click()],
    ['Discard changes', (page) => button(page, 'Discard changes').click()],
    ['Escape', (page) => page.getByLabel('Title').press('Escape')],
    ['the toggle', (page) => toggle(page).click()],
    ['deleting the bookmark', async (page) => {
      page.once('dialog', (dialog) => dialog.accept());
      await button(page, `Delete ${MDN.title}`).click();
    }],
  ];
  for (const [name, exit] of exits) {
    test(`${name} returns the form to add mode`, async ({ page }) => {
      await button(page, `Edit ${MDN.title}`).click();
      await exit(page);
      await expect(page.locator('#bookmark-form')).toBeHidden();
      await expect(toggle(page)).toHaveAccessibleName('Add a bookmark');
      await openForm(page);
      await expectAddMode(page);
    });
  }

  test('reopening the form while it is still closing shows add mode', async ({ page }) => {
    await button(page, `Edit ${MDN.title}`).click();
    await expect(page.locator('#bookmark-form')).not.toHaveAttribute('style', /height/);
    await page.getByLabel('Title').press('Escape');
    await toggle(page).click(); // well inside the 450ms close
    await expectAddMode(page);
    await expect(toggle(page)).toHaveAccessibleName('Close form');
  });

  test('with reduced motion the toggle label switches without animating', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.reload(); // drop animations left over from the setup
    await openForm(page);
    await expect(toggle(page)).toHaveAccessibleName('Close form');
    expect(await page.evaluate(() => document.getAnimations().length)).toBe(0);
    await expect(page.locator('#form-toggle-label .roll-text')).toHaveCount(1);

    await button(page, `Edit ${MDN.title}`).click();
    await expect(toggle(page)).toHaveAccessibleName('Close form');
    await toggle(page).click();
    await expect(toggle(page)).toHaveAccessibleName('Add a bookmark');
    expect(await page.evaluate(() => document.getAnimations().length)).toBe(0);
    await expect(page.locator('#form-toggle-label .roll-text')).toHaveCount(1);
  });

  test('a 120-character title in the heading does not scroll sideways at 375px', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 800 });
    const longTitle = 'A'.repeat(60) + 'Bookmark' + 'B'.repeat(52);
    expect(longTitle).toHaveLength(120);
    await addBookmark(page, { url: 'https://example.net', title: longTitle, notes: '', tags: '' });
    await button(page, `Edit ${longTitle}`).click();
    await expect(page.locator('#form-heading')).toHaveText(`Edit bookmark: ${longTitle}`);

    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth
    );
    expect(overflow).toBeLessThanOrEqual(0);
    const heading = await page.locator('#form-heading').boundingBox();
    const form = await page.locator('#bookmark-form').boundingBox();
    expect(heading.x + heading.width).toBeLessThanOrEqual(form.x + form.width + 0.5);
  });

  test('the form opens, edits and closes by keyboard with visible focus rings', async ({ page, browserName }) => {
    // WebKit follows Safari: Tab skips buttons unless Option is held.
    const tab = browserName === 'webkit' ? 'Alt+Tab' : 'Tab';
    const shiftTab = browserName === 'webkit' ? 'Alt+Shift+Tab' : 'Shift+Tab';
    const outline = () =>
      page.evaluate(() => getComputedStyle(document.activeElement).outlineStyle);

    // Let the setup's closing animation finish so the form is out of the tab order.
    await expect(page.locator('#bookmark-form')).toBeHidden();
    // Reach the toggle by keyboard: it sits just before the search box.
    await page.getByLabel('Search bookmarks').focus();
    await page.keyboard.press(shiftTab);
    await expect(toggle(page)).toBeFocused();
    expect(await outline()).toBe('solid');
    await page.keyboard.press('Enter');
    await expect(page.getByLabel('URL')).toBeFocused();
    await expectAddMode(page);
    await page.keyboard.press('Escape');
    await expect(page.locator('#bookmark-form')).toBeHidden();

    const edit = button(page, `Edit ${MDN.title}`);
    await edit.focus();
    await page.keyboard.press(tab);
    await page.keyboard.press(shiftTab);
    await expect(edit).toBeFocused();
    expect(await outline()).toBe('solid');
    await page.keyboard.press('Enter');
    await expect(page.getByRole('form', { name: `Edit bookmark: ${MDN.title}` })).toBeVisible();
    await expect(page.getByLabel('URL')).toBeFocused();

    await page.keyboard.press(tab);
    await page.keyboard.press('ArrowRight'); // Tab selects the field's text
    await page.keyboard.type(' (edited)');
    // Tab through Notes and Tags to Discard changes, then Save changes.
    await page.keyboard.press(tab);
    await page.keyboard.press(tab);
    await page.keyboard.press(tab);
    await expect(button(page, 'Discard changes')).toBeFocused();
    expect(await outline()).toBe('solid');
    await page.keyboard.press(tab);
    await expect(button(page, 'Save changes')).toBeFocused();
    expect(await outline()).toBe('solid');
    await page.keyboard.press('Enter');

    await expect(page.locator('#bookmark-form')).toBeHidden();
    await expect(page.getByRole('link', { name: `${MDN.title} (edited)` })).toBeVisible();
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

test.describe('Review fixes', () => {
  const SUBMIT = '#bookmark-form button[type="submit"]';
  const LONG_TAG = 'averyveryverylongtagnamewithoutanyspacesatallforthetest';

  /** Add a bookmark without relying on the submit button's label. */
  async function add(page, { url, title, notes = '', tags = '' }) {
    await openForm(page);
    await page.getByLabel('URL').fill(url);
    await page.getByLabel('Title').fill(title);
    await page.getByLabel('Notes').fill(notes);
    await page.getByLabel('Tags').fill(tags);
    await page.locator(SUBMIT).click();
  }

  /** Describe the focused element: its tag, text, and owning bookmark title. */
  const focused = (page) =>
    page.evaluate(() => {
      const el = document.activeElement;
      return {
        tag: el.tagName,
        id: el.id,
        text: el.textContent.trim(),
        className: el.className,
        inChips: !!el.closest('#tag-filter'),
        bookmark: el.closest('.bookmark')?.querySelector('.bookmark-title')?.textContent ?? null,
      };
    });

  /** Press Enter on a locator via the keyboard. */
  async function press(locator) {
    await locator.focus();
    await locator.press('Enter');
  }

  // Newest first: Third, Second, First.
  async function seedThree(page) {
    await page.goto('/');
    await add(page, { url: 'https://a.example.com', title: 'First', tags: 'alpha, beta' });
    await page.waitForTimeout(5);
    await add(page, { url: 'https://b.example.com', title: 'Second', tags: 'alpha' });
    await page.waitForTimeout(5);
    await add(page, { url: 'https://c.example.com', title: 'Third', tags: 'beta' });
    await expect(page.locator('.bookmark')).toHaveCount(3);
  }

  test.describe('D1 keyboard focus', () => {
    test.beforeEach(async ({ page }) => seedThree(page));

    test('tag chip keeps focus after filtering, and Tab moves to the next chip', async ({ page, browserName }) => {
      const chip = page.locator('#tag-filter .chip', { hasText: 'alpha' });
      await press(chip);
      expect(await focused(page)).toMatchObject({ tag: 'BUTTON', text: 'alpha', inChips: true });
      // macOS WebKit skips buttons on plain Tab (Safari setting); Option+Tab reaches them.
      await page.keyboard.press(browserName === 'webkit' ? 'Alt+Tab' : 'Tab');
      expect(await focused(page)).toMatchObject({ text: 'beta', inChips: true });
    });

    test('"All" chip keeps focus after clearing the filter', async ({ page }) => {
      await press(page.locator('#tag-filter .chip', { hasText: 'alpha' }));
      await press(page.locator('#tag-filter .chip', { hasText: 'All' }));
      expect(await focused(page)).toMatchObject({ tag: 'BUTTON', text: 'All', inChips: true });
    });

    test('a tag on a bookmark keeps focus on the same tag of the same bookmark', async ({ page }) => {
      const item = page.locator('.bookmark', { hasText: 'Second' });
      await press(item.locator('button.tag', { hasText: 'alpha' }));
      expect(await focused(page)).toMatchObject({
        tag: 'BUTTON',
        text: 'alpha',
        inChips: false,
        bookmark: 'Second',
      });
    });

    test('deleting moves focus to the next bookmark title', async ({ page }) => {
      page.once('dialog', (d) => d.accept());
      await press(page.getByRole('button', { name: 'Delete Second' }));
      expect(await focused(page)).toMatchObject({ tag: 'A', text: 'First' });
    });

    test('deleting the last bookmark moves focus to the previous title', async ({ page }) => {
      page.once('dialog', (d) => d.accept());
      await press(page.getByRole('button', { name: 'Delete First' }));
      expect(await focused(page)).toMatchObject({ tag: 'A', text: 'Second' });
    });

    test('deleting the only bookmark moves focus to the form toggle', async ({ page }) => {
      page.on('dialog', (d) => d.accept());
      await press(page.getByRole('button', { name: 'Delete Third' }));
      await press(page.getByRole('button', { name: 'Delete Second' }));
      await press(page.getByRole('button', { name: 'Delete First' }));
      await expect(page.locator('.bookmark')).toHaveCount(0);
      expect(await focused(page)).toMatchObject({ id: 'form-toggle' });
    });

    test('dismissing the delete dialog keeps focus on Delete', async ({ page }) => {
      page.once('dialog', (d) => d.dismiss());
      const del = page.getByRole('button', { name: 'Delete Second' });
      await press(del);
      expect(await focused(page)).toMatchObject({ className: 'delete-button' });
      await expect(del).toBeFocused();
    });

    test('a successful save moves focus to the form toggle', async ({ page }) => {
      await openForm(page);
      await page.getByLabel('URL').fill('https://d.example.com');
      await page.getByLabel('Title').fill('Fourth');
      await page.locator(SUBMIT).focus();
      await page.locator(SUBMIT).press('Enter');
      await expect(page.locator('.bookmark')).toHaveCount(4);
      expect(await focused(page)).toMatchObject({ id: 'form-toggle' });
    });

    test('a successful update moves focus to the form toggle', async ({ page }) => {
      await press(page.getByRole('button', { name: 'Edit Second' }));
      await page.getByLabel('Title').fill('Second edited');
      await page.locator(SUBMIT).focus();
      await page.locator(SUBMIT).press('Enter');
      await expect(page.locator('.bookmark-title', { hasText: 'Second edited' })).toBeVisible();
      expect(await focused(page)).toMatchObject({ id: 'form-toggle' });
    });
  });

  test.describe('D2 long tags on phones', () => {
    for (const width of [320, 375]) {
      test(`no sideways scrolling at ${width}px`, async ({ page }) => {
        await page.setViewportSize({ width, height: 640 });
        await page.goto('/');
        await add(page, { url: 'https://example.com', title: 'Long tag', tags: LONG_TAG });
        await expect(page.locator('.bookmark')).toHaveCount(1);

        const m = await page.evaluate(() => ({
          scroll: document.documentElement.scrollWidth,
          client: document.documentElement.clientWidth,
          chipRight: document.querySelector('#tag-filter .chip:last-child').getBoundingClientRect().right,
          tagRight: document.querySelector('.bookmark button.tag').getBoundingClientRect().right,
        }));
        expect(m.scroll).toBeLessThanOrEqual(m.client);
        expect(m.chipRight).toBeLessThanOrEqual(m.client);
        expect(m.tagRight).toBeLessThanOrEqual(m.client);
      });
    }
  });

  test.describe('D3 URL validation', () => {
    const MESSAGE = /URL|http/;
    for (const bad of [
      'https:example.com',
      'http:/example.com',
      'https://exa mple.com',
      'ftp://example.com',
    ]) {
      test(`rejects ${bad}`, async ({ page }) => {
        await page.goto('/');
        await add(page, { url: 'https://keep.example.com', title: 'Keep' });
        const before = await storedBookmarks(page);

        await add(page, { url: bad, title: 'Bad' });
        await expect(page.locator('#form-error')).toBeVisible();
        await expect(page.locator('#form-error')).toContainText(MESSAGE);
        await expect(page.locator('.bookmark')).toHaveCount(1);
        expect(await storedBookmarks(page)).toEqual(before);
      });
    }

    for (const good of ['HTTPS://Example.com', 'https://example.com/path?q=1#x']) {
      test(`accepts ${good}`, async ({ page }) => {
        await page.goto('/');
        await add(page, { url: good, title: 'Good' });
        await expect(page.locator('#form-error')).toBeHidden();
        await expect(page.locator('.bookmark')).toHaveCount(1);
        expect(await storedBookmarks(page)).toHaveLength(1);
      });
    }
  });

  test.describe('R5 input border contrast', () => {
    /** WCAG contrast ratio of two "rgb(r, g, b)" strings. */
    function contrast(a, b) {
      const lum = (css) => {
        const [r, g, bl] = css.match(/[\d.]+/g).slice(0, 3).map((v) => {
          const c = Number(v) / 255;
          return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
        });
        return 0.2126 * r + 0.7152 * g + 0.0722 * bl;
      };
      const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
      return (hi + 0.05) / (lo + 0.05);
    }

    for (const scheme of ['light', 'dark']) {
      test(`${scheme}: URL input border is at least 3:1`, async ({ page }) => {
        await page.emulateMedia({ colorScheme: scheme });
        await page.goto('/');
        await openForm(page);
        const s = await page.evaluate(() => {
          const input = document.getElementById('url');
          const style = getComputedStyle(input);
          return {
            border: style.borderTopColor,
            field: style.backgroundColor,
            panel: getComputedStyle(input.closest('.panel')).backgroundColor,
          };
        });
        expect(contrast(s.border, s.field)).toBeGreaterThanOrEqual(3);
        expect(contrast(s.border, s.panel)).toBeGreaterThanOrEqual(3);
      });
    }
  });
});
