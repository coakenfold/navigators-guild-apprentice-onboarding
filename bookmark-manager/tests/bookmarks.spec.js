// @ts-check
import { test, expect } from "@playwright/test";
import path from "node:path";

const MDN = {
  url: "https://developer.mozilla.org",
  title: "MDN Web Docs",
  notes: "Great reference for HTML and CSS.",
  tags: "JavaScript, reference , javascript",
};
const EXAMPLE = {
  url: "https://example.com/pasta",
  title: "Easy Weeknight Pasta",
  notes: "",
  tags: "recipes, dinner",
};

/**
 * The add form starts collapsed. Open it if the toggle says it is closed, then
 * wait for the drawer animation to finish (it leaves inline styles while it runs).
 * @param {import('@playwright/test').Page} page
 */
async function openForm(page) {
  const toggle = page.locator("#form-toggle");
  if ((await toggle.getAttribute("aria-expanded")) === "false")
    await toggle.click();
  const form = page.locator("#bookmark-form");
  await expect(form).toBeVisible();
  // The animation sets overflow first and height a frame later, then clears both
  // when it ends, so "no inline style at all" is the only safe finished signal.
  await expect.poll(() => form.evaluate((el) => el.style.cssText)).toBe("");
}

/**
 * Fill the form and save. A good save closes the form, so wait for that. A bad
 * one shows the error instead, so wait for either outcome.
 * @param {import('@playwright/test').Page} page
 */
async function addBookmark(page, { url, title, notes, tags }) {
  await openForm(page);
  await page.getByLabel("URL").fill(url);
  await page.getByLabel("Title").fill(title);
  await page.getByLabel("Notes").fill(notes);
  await page.getByLabel("Tags").fill(tags);
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          document.getElementById("bookmark-form").hidden ||
          !document.getElementById("form-error").hidden,
      ),
    )
    .toBe(true);
}

/** @param {import('@playwright/test').Page} page */
const storedBookmarks = (page) =>
  page.evaluate(() => JSON.parse(localStorage.getItem("bookmarks") ?? "null"));

test.describe("Delete", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);
  });

  test("cancelling the dialog leaves the bookmark in place", async ({
    page,
  }) => {
    /** @type {string[]} */
    const messages = [];
    page.once("dialog", (dialog) => {
      messages.push(dialog.message());
      dialog.dismiss();
    });
    await page.getByRole("button", { name: `Delete ${EXAMPLE.title}` }).click();

    expect(messages).toEqual(["Are you sure?"]);
    await expect(page.locator(".bookmark")).toHaveCount(2);
    expect(await storedBookmarks(page)).toHaveLength(2);
  });

  test("confirming removes it, and it stays gone after a refresh", async ({
    page,
  }) => {
    page.once("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: `Delete ${EXAMPLE.title}` }).click();

    await expect(page.locator(".bookmark .bookmark-title")).toHaveText([
      MDN.title,
    ]);

    await page.reload();
    await expect(page.locator(".bookmark .bookmark-title")).toHaveText([
      MDN.title,
    ]);
    expect(await storedBookmarks(page)).toHaveLength(1);
  });

  test("deleting every bookmark brings back the empty-state message", async ({
    page,
  }) => {
    page.on("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: `Delete ${EXAMPLE.title}` }).click();
    await page.getByRole("button", { name: `Delete ${MDN.title}` }).click();

    await expect(page.locator(".bookmark")).toHaveCount(0);
    await expect(
      page.getByText("No bookmarks yet. Add your first one above."),
    ).toBeVisible();
    expect(await storedBookmarks(page)).toEqual([]);
  });
});

test.describe("Add & list", () => {
  /** @type {string[]} */
  let consoleErrors;

  test.beforeEach(async ({ page }) => {
    consoleErrors = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") consoleErrors.push(msg.text());
    });
    page.on("pageerror", (err) => consoleErrors.push(err.message));
    await page.goto("/");
  });

  test("first load shows the empty state and no console errors", async ({
    page,
  }) => {
    await expect(
      page.getByText("No bookmarks yet. Add your first one above."),
    ).toBeVisible();
    await expect(page.locator(".bookmark")).toHaveCount(0);
    expect(consoleErrors).toEqual([]);
  });

  test("adding a bookmark lists it with cleaned tags and clears the form", async ({
    page,
  }) => {
    await addBookmark(page, MDN);

    const items = page.locator(".bookmark");
    await expect(items).toHaveCount(1);
    await expect(items.first().locator(".bookmark-title")).toHaveText(
      MDN.title,
    );
    await expect(items.first().locator(".bookmark-url")).toHaveText(MDN.url);
    await expect(items.first().locator(".bookmark-notes")).toHaveText(
      MDN.notes,
    );
    await expect(items.first().locator(".tag")).toHaveText([
      "javascript",
      "reference",
    ]);
    await expect(items.first().locator(".bookmark-date")).toHaveText(
      /^[A-Z][a-z]{2} \d{1,2}, \d{4}$/,
    );
    await expect(page.getByText("No bookmarks yet.")).toBeHidden();

    for (const label of ["URL", "Title", "Notes", "Tags"]) {
      await expect(page.getByLabel(label)).toHaveValue("");
    }
    expect(consoleErrors).toEqual([]);
  });

  test("a second bookmark appears above the first", async ({ page }) => {
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);

    await expect(page.locator(".bookmark .bookmark-title")).toHaveText([
      EXAMPLE.title,
      MDN.title,
    ]);
  });

  test("bookmarks persist across a refresh in the same order", async ({
    page,
  }) => {
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);

    await page.reload();

    await expect(page.locator(".bookmark .bookmark-title")).toHaveText([
      EXAMPLE.title,
      MDN.title,
    ]);
    await expect(page.locator(".bookmark").nth(1).locator(".tag")).toHaveText([
      "javascript",
      "reference",
    ]);
  });

  test("an ftp:// URL shows an error and saves nothing", async ({ page }) => {
    await addBookmark(page, { ...MDN, url: "ftp://x.com" });

    await expect(page.getByRole("alert")).toContainText("http://");
    await expect(page.locator(".bookmark")).toHaveCount(0);
    await expect(page.getByText("No bookmarks yet.")).toBeVisible();
    expect(await storedBookmarks(page)).toBeNull();
  });

  test("an empty title shows an error and saves nothing", async ({ page }) => {
    await addBookmark(page, { ...MDN, title: "" });

    await expect(page.getByRole("alert")).toBeVisible();
    await expect(page.locator(".bookmark")).toHaveCount(0);
    expect(await storedBookmarks(page)).toBeNull();
  });

  test("a missing URL shows an error and saves nothing", async ({ page }) => {
    await addBookmark(page, { ...MDN, url: "" });

    await expect(page.getByRole("alert")).toBeVisible();
    expect(await storedBookmarks(page)).toBeNull();
  });

  test("clicking a title opens the link in a new tab", async ({
    page,
    context,
  }) => {
    // Stub the external site so the test does not need network access.
    await context.route("https://developer.mozilla.org/**", (route) =>
      route.fulfill({
        contentType: "text/html",
        body: "<title>MDN stub</title>",
      }),
    );
    await addBookmark(page, MDN);

    const link = page.getByRole("link", { name: MDN.title });
    await expect(link).toHaveAttribute("target", "_blank");
    await expect(link).toHaveAttribute("rel", /noopener/);

    const popupPromise = context.waitForEvent("page");
    await link.click();
    const popup = await popupPromise;
    await popup.waitForLoadState();

    expect(popup.url()).toMatch(/^https:\/\/developer\.mozilla\.org\/?$/);
    expect(page.url()).toBe("http://localhost:3000/");
  });

  test("localStorage.bookmarks matches the data model", async ({ page }) => {
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);

    const stored = await storedBookmarks(page);

    expect(stored).toHaveLength(2);
    for (const bookmark of stored) {
      expect(Object.keys(bookmark).sort()).toEqual([
        "createdAt",
        "id",
        "notes",
        "tags",
        "title",
        "url",
      ]);
      expect(bookmark.id).toMatch(/^\d+$/);
      expect(new Date(bookmark.createdAt).toISOString()).toBe(
        bookmark.createdAt,
      );
      expect(Array.isArray(bookmark.tags)).toBe(true);
    }

    const mdn = stored.find((b) => b.title === MDN.title);
    expect(mdn).toMatchObject({
      url: MDN.url,
      notes: MDN.notes,
      tags: ["javascript", "reference"],
    });
    expect(new Set(stored.map((b) => b.id)).size).toBe(2);
  });
});

// 05-first-build.md Layer 3: "Clicking edit should let me change the title,
// URL, note, and tags inline." Edit happens inside the bookmark's own card;
// the add form at the top is only for adding.
test.describe("Edit (inline)", () => {
  /** @type {string[]} */
  let consoleErrors;
  const button = (/** @type {import('@playwright/test').Page} */ page, name) =>
    page.getByRole("button", { name, exact: true });
  /** The inline edit form, found by its accessible name. */
  const editForm = (/** @type {import('@playwright/test').Page} */ page, title) =>
    page.getByRole("form", { name: `Edit bookmark: ${title}` });
  const toggle = (/** @type {import('@playwright/test').Page} */ page) =>
    page.locator("#form-toggle");

  test.beforeEach(async ({ page }) => {
    consoleErrors = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") consoleErrors.push(msg.text());
    });
    page.on("pageerror", (err) => consoleErrors.push(err.message));
    await page.goto("/");
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);
  });

  test.afterEach(() => {
    expect(consoleErrors).toEqual([]);
  });

  test("Edit opens filled-in fields inside that bookmark's card, not the add form", async ({
    page,
  }) => {
    await button(page, `Edit ${MDN.title}`).click();

    const form = editForm(page, MDN.title);
    await expect(form).toBeVisible();
    const stored = await storedBookmarks(page);
    const mdnId = stored.find((b) => b.title === MDN.title).id;
    await expect(page.locator(`.bookmark[data-id="${mdnId}"] .edit-form`)).toHaveCount(1);
    await expect(page.locator(".edit-form")).toHaveCount(1);

    await expect(form.getByLabel("URL")).toHaveValue(MDN.url);
    await expect(form.getByLabel("Title")).toHaveValue(MDN.title);
    await expect(form.getByLabel("Notes")).toHaveValue(MDN.notes);
    await expect(form.getByLabel("Tags")).toHaveValue("javascript, reference");
    await expect(form.getByLabel("URL")).toBeFocused();
    await expect(form.getByRole("button", { name: "Save changes" })).toBeVisible();
    await expect(form.getByRole("button", { name: "Discard changes" })).toBeVisible();

    // The add form stays closed, and the other card is untouched.
    await expect(page.locator("#bookmark-form")).toBeHidden();
    await expect(toggle(page)).toHaveAttribute("aria-expanded", "false");
    await expect(page.getByRole("link", { name: EXAMPLE.title })).toBeVisible();
  });

  test("updates in place, keeping id, createdAt and position, and survives refresh", async ({
    page,
  }) => {
    const before = await storedBookmarks(page);
    const original = before.find((b) => b.title === MDN.title);

    await button(page, `Edit ${MDN.title}`).click();
    const form = editForm(page, MDN.title);
    await form.getByLabel("Title").fill("MDN Renamed");
    await form.getByLabel("URL").fill("https://developer.mozilla.org/en-US/");
    await form.getByLabel("Notes").fill("New note");
    await form.getByLabel("Tags").fill("Docs, HTML");
    await form.getByRole("button", { name: "Save changes" }).click();

    await expect(page.locator(".edit-form")).toHaveCount(0);
    await expect(page.locator(".bookmark .bookmark-title")).toHaveText([
      EXAMPLE.title,
      "MDN Renamed",
    ]);

    await page.reload();
    const after = await storedBookmarks(page);
    expect(after).toHaveLength(2);
    expect(after.find((b) => b.id === original.id)).toEqual({
      id: original.id,
      url: "https://developer.mozilla.org/en-US/",
      title: "MDN Renamed",
      notes: "New note",
      tags: ["docs", "html"],
      createdAt: original.createdAt,
    });
    await expect(page.locator(".bookmark .bookmark-title")).toHaveText([
      EXAMPLE.title,
      "MDN Renamed",
    ]);
  });

  test("saving moves focus to that bookmark's Edit button", async ({ page }) => {
    await button(page, `Edit ${MDN.title}`).click();
    const form = editForm(page, MDN.title);
    await form.getByLabel("Title").fill("MDN 2");
    await form.getByRole("button", { name: "Save changes" }).click();
    await expect(button(page, "Edit MDN 2")).toBeFocused();
  });

  test("rejects invalid input inside the card, then saves once it is fixed", async ({
    page,
  }) => {
    await button(page, `Edit ${MDN.title}`).click();
    const form = editForm(page, MDN.title);
    await form.getByLabel("Title").fill("");
    await form.getByRole("button", { name: "Save changes" }).click();

    const alert = form.getByRole("alert");
    await expect(alert).toHaveText("Please fill in both URL and Title.");
    const title = form.getByLabel("Title");
    await expect(title).toHaveAttribute("aria-invalid", "true");
    await expect(title).toHaveAttribute("aria-describedby", /.+/);
    await expect(title).toBeFocused();
    expect((await storedBookmarks(page)).map((b) => b.title)).toContain(
      MDN.title,
    );

    await form.getByLabel("URL").fill("ftp://x.com");
    await title.fill("Changed");
    await form.getByRole("button", { name: "Save changes" }).click();
    await expect(alert).toHaveText("URL must start with http:// or https://");

    await form.getByLabel("URL").fill(MDN.url);
    await form.getByRole("button", { name: "Save changes" }).click();
    await expect(page.locator(".edit-form")).toHaveCount(0);
    await expect(page.getByRole("link", { name: "Changed" })).toBeVisible();
    expect(await storedBookmarks(page)).toHaveLength(2);
  });

  /** @type {[string, (page: import('@playwright/test').Page) => Promise<void>][]} */
  const exits = [
    [
      "Discard changes",
      (page) =>
        editForm(page, MDN.title)
          .getByRole("button", { name: "Discard changes" })
          .click(),
    ],
    [
      "Escape",
      (page) => editForm(page, MDN.title).getByLabel("Title").press("Escape"),
    ],
  ];
  for (const [name, exit] of exits) {
    test(`${name} reverts the edit and returns focus to Edit`, async ({
      page,
    }) => {
      const before = await storedBookmarks(page);
      await button(page, `Edit ${MDN.title}`).click();
      const form = editForm(page, MDN.title);
      await form.getByLabel("Title").fill("Changed");
      await form.getByLabel("Tags").fill("other");
      await exit(page);

      await expect(page.locator(".edit-form")).toHaveCount(0);
      await expect(page.getByRole("link", { name: MDN.title })).toBeVisible();
      await expect(
        page.locator(".bookmark", { hasText: MDN.title }).locator(".tag"),
      ).toHaveText(["javascript", "reference"]);
      expect(await storedBookmarks(page)).toEqual(before);
      await expect(button(page, `Edit ${MDN.title}`)).toBeFocused();

      await page.reload();
      expect(await storedBookmarks(page)).toEqual(before);
    });
  }

  test("the heading keeps the title from when the edit started", async ({
    page,
  }) => {
    await button(page, `Edit ${MDN.title}`).click();
    const form = editForm(page, MDN.title);
    await form.getByLabel("Title").fill("Something else");
    await expect(form.locator(".form-heading")).toHaveText(
      `Edit bookmark: ${MDN.title}`,
    );
  });

  test("editing a second bookmark switches the inline form to it", async ({
    page,
  }) => {
    await button(page, `Edit ${MDN.title}`).click();
    await button(page, `Edit ${EXAMPLE.title}`).click();
    await expect(page.locator(".edit-form")).toHaveCount(1);
    await expect(editForm(page, EXAMPLE.title)).toBeVisible();
    await expect(page.getByRole("link", { name: MDN.title })).toBeVisible();
  });

  test("an unsaved edit survives typing in the search box and deleting another bookmark", async ({
    page,
  }) => {
    await button(page, `Edit ${MDN.title}`).click();
    await editForm(page, MDN.title).getByLabel("Title").fill("Draft title");

    await page.getByLabel("Search bookmarks").fill("mozilla");
    await expect(editForm(page, MDN.title).getByLabel("Title")).toHaveValue(
      "Draft title",
    );
    await page.getByLabel("Search bookmarks").fill("");

    page.once("dialog", (dialog) => dialog.accept());
    await button(page, `Delete ${EXAMPLE.title}`).click();
    await expect(page.locator(".bookmark")).toHaveCount(1);
    await expect(editForm(page, MDN.title).getByLabel("Title")).toHaveValue(
      "Draft title",
    );
  });

  test("the add form stays in add mode while a bookmark is being edited", async ({
    page,
  }) => {
    await button(page, `Edit ${MDN.title}`).click();
    await openForm(page);
    const add = page.getByRole("form", { name: "Add a bookmark" });
    await expect(add).toBeVisible();
    await expect(add.getByRole("button", { name: "Save", exact: true })).toBeVisible();
    await expect(add.getByRole("button", { name: "Cancel", exact: true })).toBeVisible();
    await expect(add.getByLabel("Title")).toHaveValue("");
    await expect(editForm(page, MDN.title)).toBeVisible();

    // Adding a bookmark leaves the edit alone.
    await add.getByLabel("URL").fill("https://example.org");
    await add.getByLabel("Title").fill("Third");
    await add.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByRole("link", { name: "Third" })).toBeVisible();
    await expect(editForm(page, MDN.title)).toBeVisible();
    expect(await storedBookmarks(page)).toHaveLength(3);
  });

  test('opening the form shows add-mode names and a "Close form" toggle', async ({
    page,
  }) => {
    await expect(toggle(page)).toHaveAccessibleName("Add a bookmark");
    await expect(toggle(page)).toHaveAttribute("aria-expanded", "false");
    await openForm(page);
    await expect(
      page.getByRole("heading", { name: "Add a bookmark", exact: true }),
    ).toBeVisible();
    await expect(button(page, "Save")).toBeVisible();
    await expect(button(page, "Cancel")).toBeVisible();
    await expect(toggle(page)).toHaveAccessibleName("Close form");
    await expect(toggle(page)).toHaveAttribute("aria-expanded", "true");
  });

  test("with reduced motion the toggle label switches without animating", async ({
    page,
  }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.reload(); // drop animations left over from the setup
    await openForm(page);
    await expect(toggle(page)).toHaveAccessibleName("Close form");
    expect(await page.evaluate(() => document.getAnimations().length)).toBe(0);
    await expect(page.locator("#form-toggle-label .roll-text")).toHaveCount(1);

    await button(page, `Edit ${MDN.title}`).click();
    await expect(toggle(page)).toHaveAccessibleName("Close form");
    await toggle(page).click();
    await expect(toggle(page)).toHaveAccessibleName("Add a bookmark");
    expect(await page.evaluate(() => document.getAnimations().length)).toBe(0);
    await expect(page.locator("#form-toggle-label .roll-text")).toHaveCount(1);
  });

  test("a 120-character title in the inline form does not scroll sideways at 375px", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 375, height: 800 });
    const longTitle = "A".repeat(60) + "Bookmark" + "B".repeat(52);
    expect(longTitle).toHaveLength(120);
    await addBookmark(page, {
      url: "https://example.net",
      title: longTitle,
      notes: "",
      tags: "",
    });
    await button(page, `Edit ${longTitle}`).click();
    const form = editForm(page, longTitle);
    await expect(form.locator(".form-heading")).toHaveText(
      `Edit bookmark: ${longTitle}`,
    );

    const overflow = await page.evaluate(
      () =>
        document.documentElement.scrollWidth -
        document.documentElement.clientWidth,
    );
    expect(overflow).toBeLessThanOrEqual(0);
    const heading = await form.locator(".form-heading").boundingBox();
    const box = await form.boundingBox();
    expect(heading.x + heading.width).toBeLessThanOrEqual(box.x + box.width + 0.5);

    // Phone-sized targets inside the inline form too.
    const tooShort = await form.evaluate((el) =>
      [...el.querySelectorAll("button, input, textarea")]
        .map((c) => c.getBoundingClientRect().height)
        .filter((h) => h > 0 && h < 43.5),
    );
    expect(tooShort).toEqual([]);
  });

  test("the add form opens and closes, and a bookmark edits inline, by keyboard with visible focus rings", async ({
    page,
    browserName,
  }) => {
    // WebKit follows Safari: Tab skips buttons unless Option is held.
    const tab = browserName === "webkit" ? "Alt+Tab" : "Tab";
    const shiftTab = browserName === "webkit" ? "Alt+Shift+Tab" : "Shift+Tab";
    const outline = () =>
      page.evaluate(
        () => getComputedStyle(document.activeElement).outlineStyle,
      );

    // Let the setup's closing animation finish so the form is out of the tab order.
    await expect(page.locator("#bookmark-form")).toBeHidden();
    // Reach the toggle by keyboard: it sits just before the search box.
    await page.getByLabel("Search bookmarks").focus();
    await page.keyboard.press(shiftTab);
    await expect(toggle(page)).toBeFocused();
    expect(await outline()).toBe("solid");
    await page.keyboard.press("Enter");
    await expect(page.locator("#url")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(page.locator("#bookmark-form")).toBeHidden();

    const edit = button(page, `Edit ${MDN.title}`);
    await edit.focus();
    await page.keyboard.press(tab);
    await page.keyboard.press(shiftTab);
    await expect(edit).toBeFocused();
    expect(await outline()).toBe("solid");
    await page.keyboard.press("Enter");
    const form = editForm(page, MDN.title);
    await expect(form).toBeVisible();
    await expect(form.getByLabel("URL")).toBeFocused();

    await page.keyboard.press(tab);
    await page.keyboard.press("ArrowRight"); // Tab selects the field's text
    await page.keyboard.type(" (edited)");
    // Tab through Notes and Tags to Discard changes, then Save changes.
    await page.keyboard.press(tab);
    await page.keyboard.press(tab);
    await page.keyboard.press(tab);
    await expect(button(page, "Discard changes")).toBeFocused();
    expect(await outline()).toBe("solid");
    await page.keyboard.press(tab);
    await expect(button(page, "Save changes")).toBeFocused();
    expect(await outline()).toBe("solid");
    await page.keyboard.press("Enter");

    await expect(page.locator(".edit-form")).toHaveCount(0);
    await expect(
      page.getByRole("link", { name: `${MDN.title} (edited)` }),
    ).toBeVisible();
    await expect(button(page, `Edit ${MDN.title} (edited)`)).toBeFocused();
  });
});

test.describe("Live search", () => {
  const titles = (/** @type {import('@playwright/test').Page} */ page) =>
    page.locator(".bookmark .bookmark-title");
  const search = (/** @type {import('@playwright/test').Page} */ page) =>
    page.getByLabel("Search bookmarks");
  const NO_MATCH = "No bookmarks match your search.";
  const NO_BOOKMARKS = "No bookmarks yet. Add your first one above.";

  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);
  });

  test("finds the MDN bookmark by title, URL or notes, ignoring case", async ({
    page,
  }) => {
    for (const query of ["mdn", "MOZILLA", "reference for html"]) {
      await search(page).fill(query);
      await expect(titles(page)).toHaveText([MDN.title]);
    }
  });

  test("updates on every keystroke", async ({ page }) => {
    await search(page).pressSequentially("pas");
    await expect(titles(page)).toHaveText([EXAMPLE.title]);
    await search(page).pressSequentially("ta");
    await expect(titles(page)).toHaveText([EXAMPLE.title]);
    await search(page).pressSequentially("zzz");
    await expect(page.getByText(NO_MATCH)).toBeVisible();
  });

  test("does not match against tags", async ({ page }) => {
    await search(page).fill("dinner");
    await expect(titles(page)).toHaveCount(0);
    await expect(page.getByText(NO_MATCH)).toBeVisible();
  });

  test("a search with no results shows the no-match message", async ({
    page,
  }) => {
    await search(page).fill("zzzz");
    await expect(titles(page)).toHaveCount(0);
    await expect(page.getByText(NO_MATCH)).toBeVisible();
    await expect(page.getByText(NO_BOOKMARKS)).toBeHidden();
  });

  test("clearing the box shows everything again", async ({ page }) => {
    await search(page).fill("zzzz");
    await search(page).fill("");
    await expect(titles(page)).toHaveCount(2);
    await expect(page.getByText(NO_MATCH)).toBeHidden();
  });

  test("searches within the active tag filter", async ({ page }) => {
    await addBookmark(page, {
      url: "https://example.com/soup",
      title: "Soup Basics",
      notes: "",
      tags: "recipes",
    });
    await page
      .locator("#tag-filter")
      .getByRole("button", { name: "recipes" })
      .click();
    await expect(titles(page)).toHaveCount(2);

    await search(page).fill("example.com");
    await expect(titles(page)).toHaveCount(2);

    await search(page).fill("soup");
    await expect(titles(page)).toHaveText(["Soup Basics"]);

    // MDN matches the text but is outside the recipes filter.
    await search(page).fill("mdn");
    await expect(page.getByText(NO_MATCH)).toBeVisible();
  });

  test("with zero bookmarks the no-bookmarks message still shows", async ({
    page,
  }) => {
    await page.evaluate(() => localStorage.clear());
    await page.reload();
    await expect(page.getByText(NO_BOOKMARKS)).toBeVisible();

    await search(page).fill("anything");
    await expect(page.getByText(NO_BOOKMARKS)).toBeVisible();
    await expect(page.getByText(NO_MATCH)).toBeHidden();
  });
});

test.describe("Review fixes", () => {
  const SUBMIT = '#bookmark-form button[type="submit"]';
  const LONG_TAG = "averyveryverylongtagnamewithoutanyspacesatallforthetest";

  /** Add a bookmark without relying on the submit button's label. */
  async function add(page, { url, title, notes = "", tags = "" }) {
    await openForm(page);
    await page.getByLabel("URL").fill(url);
    await page.getByLabel("Title").fill(title);
    await page.getByLabel("Notes").fill(notes);
    await page.getByLabel("Tags").fill(tags);
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
        inChips: !!el.closest("#tag-filter"),
        bookmark:
          el.closest(".bookmark")?.querySelector(".bookmark-title")
            ?.textContent ?? null,
      };
    });

  /** Press Enter on a locator via the keyboard. */
  async function press(locator) {
    await locator.focus();
    await locator.press("Enter");
  }

  // Newest first: Third, Second, First.
  async function seedThree(page) {
    await page.goto("/");
    await add(page, {
      url: "https://a.example.com",
      title: "First",
      tags: "alpha, beta",
    });
    await page.waitForTimeout(5);
    await add(page, {
      url: "https://b.example.com",
      title: "Second",
      tags: "alpha",
    });
    await page.waitForTimeout(5);
    await add(page, {
      url: "https://c.example.com",
      title: "Third",
      tags: "beta",
    });
    await expect(page.locator(".bookmark")).toHaveCount(3);
  }

  test.describe("D1 keyboard focus", () => {
    test.beforeEach(async ({ page }) => seedThree(page));

    test("tag chip keeps focus after filtering, and Tab moves to the next chip", async ({
      page,
      browserName,
    }) => {
      const chip = page.locator("#tag-filter .chip", { hasText: "alpha" });
      await press(chip);
      expect(await focused(page)).toMatchObject({
        tag: "BUTTON",
        text: "alpha",
        inChips: true,
      });
      // macOS WebKit skips buttons on plain Tab (Safari setting); Option+Tab reaches them.
      await page.keyboard.press(browserName === "webkit" ? "Alt+Tab" : "Tab");
      expect(await focused(page)).toMatchObject({
        text: "beta",
        inChips: true,
      });
    });

    test('"All" chip keeps focus after clearing the filter', async ({
      page,
    }) => {
      await press(page.locator("#tag-filter .chip", { hasText: "alpha" }));
      await press(page.locator("#tag-filter .chip", { hasText: "All" }));
      expect(await focused(page)).toMatchObject({
        tag: "BUTTON",
        text: "All",
        inChips: true,
      });
    });

    test("a tag on a bookmark keeps focus on the same tag of the same bookmark", async ({
      page,
    }) => {
      const item = page.locator(".bookmark", { hasText: "Second" });
      await press(item.locator("button.tag", { hasText: "alpha" }));
      expect(await focused(page)).toMatchObject({
        tag: "BUTTON",
        text: "alpha",
        inChips: false,
        bookmark: "Second",
      });
    });

    test("deleting moves focus to the next bookmark title", async ({
      page,
    }) => {
      page.once("dialog", (d) => d.accept());
      await press(page.getByRole("button", { name: "Delete Second" }));
      expect(await focused(page)).toMatchObject({ tag: "A", text: "First" });
    });

    test("deleting the last bookmark moves focus to the previous title", async ({
      page,
    }) => {
      page.once("dialog", (d) => d.accept());
      await press(page.getByRole("button", { name: "Delete First" }));
      expect(await focused(page)).toMatchObject({ tag: "A", text: "Second" });
    });

    test("deleting the only bookmark moves focus to the form toggle", async ({
      page,
    }) => {
      page.on("dialog", (d) => d.accept());
      await press(page.getByRole("button", { name: "Delete Third" }));
      await press(page.getByRole("button", { name: "Delete Second" }));
      await press(page.getByRole("button", { name: "Delete First" }));
      await expect(page.locator(".bookmark")).toHaveCount(0);
      expect(await focused(page)).toMatchObject({ id: "form-toggle" });
    });

    test("dismissing the delete dialog keeps focus on Delete", async ({
      page,
    }) => {
      page.once("dialog", (d) => d.dismiss());
      const del = page.getByRole("button", { name: "Delete Second" });
      await press(del);
      expect(await focused(page)).toMatchObject({ className: "delete-button" });
      await expect(del).toBeFocused();
    });

    test("a successful save moves focus to the form toggle", async ({
      page,
    }) => {
      await openForm(page);
      await page.getByLabel("URL").fill("https://d.example.com");
      await page.getByLabel("Title").fill("Fourth");
      await page.locator(SUBMIT).focus();
      await page.locator(SUBMIT).press("Enter");
      await expect(page.locator(".bookmark")).toHaveCount(4);
      expect(await focused(page)).toMatchObject({ id: "form-toggle" });
    });

    // Edit is inline (guide Layer 3), so focus returns to the edited card.
    test("a successful update moves focus to that bookmark's Edit button", async ({
      page,
    }) => {
      await press(page.getByRole("button", { name: "Edit Second" }));
      const edit = page.locator(".edit-form");
      await edit.getByLabel("Title").fill("Second edited");
      await edit.locator('button[type="submit"]').focus();
      await edit.locator('button[type="submit"]').press("Enter");
      await expect(
        page.locator(".bookmark-title", { hasText: "Second edited" }),
      ).toBeVisible();
      expect(await focused(page)).toMatchObject({
        tag: "BUTTON",
        className: "edit-button",
        bookmark: "Second edited",
      });
    });
  });

  test("tabbing right after closing the form never enters its fields", async ({
    page,
  }) => {
    await page.goto("/");
    await openForm(page);
    await page.locator("#form-toggle").click();
    await expect(page.locator("#form-toggle")).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    await page.keyboard.press("Tab");
    expect(
      await page.evaluate(
        () => !!document.activeElement.closest("#bookmark-form"),
      ),
    ).toBe(false);
    await page.waitForTimeout(600);
    expect(await page.evaluate(() => document.activeElement.tagName)).not.toBe(
      "BODY",
    );
  });

  test.describe("D2 long tags on phones", () => {
    for (const width of [320, 375]) {
      test(`no sideways scrolling at ${width}px`, async ({ page }) => {
        await page.setViewportSize({ width, height: 640 });
        await page.goto("/");
        await add(page, {
          url: "https://example.com",
          title: "Long tag",
          tags: LONG_TAG,
        });
        await expect(page.locator(".bookmark")).toHaveCount(1);

        const m = await page.evaluate(() => ({
          scroll: document.documentElement.scrollWidth,
          client: document.documentElement.clientWidth,
          chipRight: document
            .querySelector("#tag-filter .chip:last-child")
            .getBoundingClientRect().right,
          tagRight: document
            .querySelector(".bookmark button.tag")
            .getBoundingClientRect().right,
        }));
        expect(m.scroll).toBeLessThanOrEqual(m.client);
        expect(m.chipRight).toBeLessThanOrEqual(m.client);
        expect(m.tagRight).toBeLessThanOrEqual(m.client);
      });
    }
  });

  test.describe("D3 URL validation", () => {
    const MESSAGE = /URL|http/;
    for (const bad of [
      "https:example.com",
      "http:/example.com",
      "https://exa mple.com",
      "ftp://example.com",
    ]) {
      test(`rejects ${bad}`, async ({ page }) => {
        await page.goto("/");
        await add(page, { url: "https://keep.example.com", title: "Keep" });
        const before = await storedBookmarks(page);

        await add(page, { url: bad, title: "Bad" });
        await expect(page.locator("#form-error")).toBeVisible();
        await expect(page.locator("#form-error")).toContainText(MESSAGE);
        await expect(page.locator(".bookmark")).toHaveCount(1);
        expect(await storedBookmarks(page)).toEqual(before);
      });
    }

    for (const good of [
      "HTTPS://Example.com",
      "https://example.com/path?q=1#x",
    ]) {
      test(`accepts ${good}`, async ({ page }) => {
        await page.goto("/");
        await add(page, { url: good, title: "Good" });
        await expect(page.locator("#form-error")).toBeHidden();
        await expect(page.locator(".bookmark")).toHaveCount(1);
        expect(await storedBookmarks(page)).toHaveLength(1);
      });
    }
  });

  test.describe("R5 input border contrast", () => {
    /** WCAG contrast ratio of two "rgb(r, g, b)" strings. */
    function contrast(a, b) {
      const lum = (css) => {
        const [r, g, bl] = css
          .match(/[\d.]+/g)
          .slice(0, 3)
          .map((v) => {
            const c = Number(v) / 255;
            return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
          });
        return 0.2126 * r + 0.7152 * g + 0.0722 * bl;
      };
      const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
      return (hi + 0.05) / (lo + 0.05);
    }

    for (const scheme of ["light", "dark"]) {
      test(`${scheme}: URL input border is at least 3:1`, async ({ page }) => {
        await page.emulateMedia({ colorScheme: scheme });
        await page.goto("/");
        await openForm(page);
        const s = await page.evaluate(() => {
          const input = document.getElementById("url");
          const style = getComputedStyle(input);
          return {
            border: style.borderTopColor,
            field: style.backgroundColor,
            panel: getComputedStyle(input.closest(".panel")).backgroundColor,
          };
        });
        expect(contrast(s.border, s.field)).toBeGreaterThanOrEqual(3);
        expect(contrast(s.border, s.panel)).toBeGreaterThanOrEqual(3);
      });
    }
  });
});

// ---------------------------------------------------------------------------
// Review fixes 4: O7 (animation), O8 (tag buttons) and O9 (test gaps)
// ---------------------------------------------------------------------------

const SOUP = {
  url: "https://example.com/soup",
  title: "Soup Basics",
  notes: "",
  tags: "recipes",
};
const SUBMIT_BUTTON = '#bookmark-form button[type="submit"]';

/** @param {import('@playwright/test').Page} page */
const titles = (page) => page.locator(".bookmark .bookmark-title");

/** Titles of the bookmarks that have a running animation, sorted. */
const animating = (page) =>
  page.evaluate(() =>
    [...document.querySelectorAll(".bookmark")]
      .filter((el) => el.getAnimations().length > 0)
      .map((el) => el.querySelector(".bookmark-title").textContent)
      .sort(),
  );

/** Wait until no bookmark is animating. */
const settle = (page) => expect.poll(() => animating(page)).toEqual([]);

/** Open the form and fill it, but don't wait for the result of saving. */
async function fillAndSubmit(
  page,
  { url = "", title = "", notes = "", tags = "" },
) {
  await openForm(page);
  await page.getByLabel("URL").fill(url);
  await page.getByLabel("Title").fill(title);
  await page.getByLabel("Notes").fill(notes);
  await page.getByLabel("Tags").fill(tags);
  await page.locator(SUBMIT_BUTTON).click();
}

/** Playwright's macOS WebKit only Tabs onto buttons when Option is held. */
const tabKeys = (browserName) =>
  browserName === "webkit"
    ? { tab: "Alt+Tab", shiftTab: "Alt+Shift+Tab" }
    : { tab: "Tab", shiftTab: "Shift+Tab" };

test.describe("O7 animate only bookmarks that enter the list", () => {
  const search = (page) => page.getByLabel("Search bookmarks");
  const chip = (page, name) =>
    page.locator("#tag-filter").getByRole("button", { name, exact: true });

  /** Three bookmarks, newest first: Soup, Pasta, MDN. Waits for the fade to end. */
  async function seed(page) {
    await page.goto("/");
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);
    await addBookmark(page, SOUP);
    await expect(page.locator(".bookmark")).toHaveCount(3);
    await settle(page);
  }

  test("typing a search character that keeps every bookmark leaves them unanimated", async ({
    page,
  }) => {
    await seed(page);
    await search(page).press("e"); // in all three titles or URLs
    await expect(page.locator(".bookmark")).toHaveCount(3);
    expect(await animating(page)).toEqual([]);
  });

  test("narrowing a filter leaves the remaining bookmarks unanimated", async ({
    page,
  }) => {
    await seed(page);
    await chip(page, "recipes").click();
    await expect(titles(page)).toHaveText([SOUP.title, EXAMPLE.title]);
    expect(await animating(page)).toEqual([]);
  });

  test("clearing a filter animates only the bookmarks that come back", async ({
    page,
  }) => {
    await seed(page);
    await chip(page, "recipes").click();
    await settle(page);
    await chip(page, "All").click();
    await expect(page.locator(".bookmark")).toHaveCount(3);
    expect(await animating(page)).toEqual([MDN.title]);
  });

  test("clearing a search animates only the bookmarks that come back", async ({
    page,
  }) => {
    await seed(page);
    await search(page).fill("pasta");
    await expect(titles(page)).toHaveText([EXAMPLE.title]);
    await settle(page);
    await search(page).fill("");
    await expect(page.locator(".bookmark")).toHaveCount(3);
    expect(await animating(page)).toEqual([MDN.title, SOUP.title].sort());
  });

  test("a newly added bookmark animates and the others do not", async ({
    page,
  }) => {
    await seed(page);
    await fillAndSubmit(page, {
      url: "https://new.example.com",
      title: "Newest Bookmark",
    });
    await expect(page.locator(".bookmark")).toHaveCount(4);
    expect(await animating(page)).toEqual(["Newest Bookmark"]);
  });

  test("deleting leaves the remaining bookmarks unanimated", async ({
    page,
  }) => {
    await seed(page);
    page.once("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: `Delete ${SOUP.title}` }).click();
    await expect(page.locator(".bookmark")).toHaveCount(2);
    expect(await animating(page)).toEqual([]);
  });

  test("nothing animates when the page first loads", async ({ page }) => {
    await seed(page);
    await page.reload();
    await expect(page.locator(".bookmark")).toHaveCount(3);
    expect(await animating(page)).toEqual([]);
  });

  test("with reduced motion no bookmark ever animates", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await seed(page);
    const stillEveryTime = async (count) => {
      await expect(page.locator(".bookmark")).toHaveCount(count);
      expect(await animating(page)).toEqual([]);
    };

    await fillAndSubmit(page, {
      url: "https://new.example.com",
      title: "Newest Bookmark",
    });
    await stillEveryTime(4);
    await search(page).fill("pasta");
    await stillEveryTime(1);
    await search(page).fill("");
    await stillEveryTime(4);
    await chip(page, "recipes").click();
    await stillEveryTime(2);
    await chip(page, "All").click();
    await stillEveryTime(4);
    page.once("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: `Delete ${SOUP.title}` }).click();
    await stillEveryTime(3);
    await page.reload();
    await stillEveryTime(3);
  });
});

test.describe("O8 tag buttons on bookmarks", () => {
  const item = (page, bookmark) =>
    page.locator(".bookmark", { hasText: bookmark.title });
  // Found by data-tag so the state tests don't depend on the accessible name.
  const recipeButtons = (page) =>
    page.locator('.bookmark button.tag[data-tag="recipes"]');

  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);
    await addBookmark(page, SOUP);
  });

  test('is named "Filter by recipes", not pressed, and still reads "recipes"', async ({
    page,
  }) => {
    const button = item(page, EXAMPLE).getByRole("button", {
      name: "Filter by recipes",
      exact: true,
    });
    await expect(button).toHaveCount(1);
    await expect(button).toHaveText("recipes");
    await expect(button).toHaveAttribute("aria-pressed", "false");
    await expect(button).not.toHaveClass(/selected/);
  });

  test("pressing it marks every recipes button as pressed and selected", async ({
    page,
  }) => {
    await item(page, EXAMPLE).locator('button.tag[data-tag="recipes"]').click();

    await expect(recipeButtons(page)).toHaveCount(2);
    for (const button of await recipeButtons(page).all()) {
      await expect(button).toHaveAttribute("aria-pressed", "true");
      await expect(button).toHaveClass(/selected/);
    }
    const dinner = item(page, EXAMPLE).locator('button.tag[data-tag="dinner"]');
    await expect(dinner).toHaveAttribute("aria-pressed", "false");
    await expect(dinner).not.toHaveClass(/selected/);

    const chipRecipes = page.locator('#tag-filter .chip[data-tag="recipes"]');
    await expect(chipRecipes).toHaveClass(/selected/);
    await expect(chipRecipes).toHaveAttribute("aria-pressed", "true");
  });

  test("pressing it again returns every button to not pressed and selects All", async ({
    page,
  }) => {
    const first = item(page, EXAMPLE).locator('button.tag[data-tag="recipes"]');
    await first.click();
    await first.click();

    await expect(recipeButtons(page)).toHaveCount(2);
    for (const button of await recipeButtons(page).all()) {
      await expect(button).toHaveAttribute("aria-pressed", "false");
      await expect(button).not.toHaveClass(/selected/);
    }
    const all = page.locator('#tag-filter .chip[data-tag=""]');
    await expect(all).toHaveClass(/selected/);
    await expect(all).toHaveAttribute("aria-pressed", "true");
  });

  test("the filter row keeps its chip names", async ({ page }) => {
    const row = page.getByRole("group", { name: "Filter by tag" });
    await expect(row.getByRole("button")).toHaveText([
      "All",
      "dinner",
      "javascript",
      "recipes",
      "reference",
    ]);
    await expect(
      row.getByRole("button", { name: "recipes", exact: true }),
    ).toHaveCount(1);
    await expect(
      row.getByRole("button", { name: "All", exact: true }),
    ).toHaveCount(1);
    await expect(row.getByRole("button", { name: /^Filter by/ })).toHaveCount(
      0,
    );
  });
});

test.describe("Tag filter", () => {
  const chip = (page, tag) =>
    page.locator(`#tag-filter .chip[data-tag="${tag}"]`);
  const allChip = (page) => chip(page, "");

  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);
    await addBookmark(page, SOUP);
  });

  test("a chip shows only the bookmarks with that tag", async ({ page }) => {
    await chip(page, "recipes").click();
    await expect(titles(page)).toHaveText([SOUP.title, EXAMPLE.title]);
    await expect(chip(page, "recipes")).toHaveClass(/selected/);
    await expect(allChip(page)).not.toHaveClass(/selected/);
  });

  test("pressing the same chip again clears the filter", async ({ page }) => {
    await chip(page, "recipes").click();
    await chip(page, "recipes").click();
    await expect(titles(page)).toHaveText([
      SOUP.title,
      EXAMPLE.title,
      MDN.title,
    ]);
    await expect(allChip(page)).toHaveClass(/selected/);
  });

  test('"All" clears the filter', async ({ page }) => {
    await chip(page, "javascript").click();
    await expect(titles(page)).toHaveText([MDN.title]);
    await allChip(page).click();
    await expect(titles(page)).toHaveText([
      SOUP.title,
      EXAMPLE.title,
      MDN.title,
    ]);
    await expect(allChip(page)).toHaveClass(/selected/);
    await expect(allChip(page)).toHaveAttribute("aria-pressed", "true");
  });

  test("pressing a tag on a bookmark filters the list", async ({ page }) => {
    await page.locator('.bookmark button.tag[data-tag="dinner"]').click();
    await expect(titles(page)).toHaveText([EXAMPLE.title]);
    await expect(chip(page, "dinner")).toHaveClass(/selected/);
  });

  test('chips are sorted A to Z and start with "All"', async ({ page }) => {
    await expect(page.locator("#tag-filter .chip")).toHaveText([
      "All",
      "dinner",
      "javascript",
      "recipes",
      "reference",
    ]);
  });

  test('resets to "All" when the last bookmark with the selected tag is deleted', async ({
    page,
  }) => {
    await addBookmark(page, {
      url: "https://solo.example.com",
      title: "Solo",
      notes: "",
      tags: "solo",
    });
    await chip(page, "solo").click();
    await expect(titles(page)).toHaveText(["Solo"]);

    page.once("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: "Delete Solo" }).click();

    await expect(chip(page, "solo")).toHaveCount(0);
    await expect(allChip(page)).toHaveClass(/selected/);
    await expect(titles(page)).toHaveText([
      SOUP.title,
      EXAMPLE.title,
      MDN.title,
    ]);
  });

  test('resets to "All" when the last bookmark with the selected tag is edited to drop it', async ({
    page,
  }) => {
    await addBookmark(page, {
      url: "https://solo.example.com",
      title: "Solo",
      notes: "",
      tags: "solo",
    });
    await chip(page, "solo").click();
    await expect(titles(page)).toHaveText(["Solo"]);

    await page.getByRole("button", { name: "Edit Solo" }).click();
    const edit = page.locator(".edit-form");
    await edit.getByLabel("Tags").fill("");
    await edit.locator('button[type="submit"]').click();

    await expect(chip(page, "solo")).toHaveCount(0);
    await expect(allChip(page)).toHaveClass(/selected/);
    await expect(titles(page)).toHaveText([
      "Solo",
      SOUP.title,
      EXAMPLE.title,
      MDN.title,
    ]);
  });
});

test.describe("Form errors", () => {
  const error = (page) => page.locator("#form-error");

  /** @type {[string, object, string, string, string][]} */
  const cases = [
    ["both fields empty", {}, "Please fill in both URL and Title.", "url"],
    [
      "empty title",
      { url: "https://example.com" },
      "Please fill in both URL and Title.",
      "title",
    ],
    [
      "empty URL",
      { title: "No URL" },
      "Please fill in both URL and Title.",
      "url",
    ],
    [
      "wrong scheme",
      { url: "ftp://x.com", title: "T" },
      "URL must start with http:// or https://",
      "url",
    ],
    [
      "unparseable URL",
      { url: "http://", title: "T" },
      "Enter a valid URL, for example https://example.com.",
      "url",
    ],
    [
      "space in the path",
      { url: "https://example.com/a b", title: "T" },
      "URL can't contain spaces. Enter it like https://example.com.",
      "url",
    ],
  ];

  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  for (const [name, fields, message, fieldId] of cases) {
    test(`${name}: exact message, aria attributes and focus`, async ({
      page,
    }) => {
      await fillAndSubmit(page, fields);

      await expect(error(page)).toBeVisible();
      await expect(error(page)).toHaveText(message);
      const field = page.locator(`#${fieldId}`);
      await expect(field).toHaveAttribute("aria-invalid", "true");
      await expect(field).toHaveAttribute("aria-describedby", "form-error");
      await expect(field).toBeFocused();
      await expect(page.locator("#bookmark-form [aria-invalid]")).toHaveCount(
        1,
      );
      await expect(page.locator(".bookmark")).toHaveCount(0);
    });
  }

  test("a successful save hides the error and removes both attributes", async ({
    page,
  }) => {
    await fillAndSubmit(page, { url: "ftp://x.com", title: "T" });
    await expect(error(page)).toBeVisible();
    await expect(page.locator("#url")).toHaveAttribute("aria-invalid", "true");

    await page.getByLabel("URL").fill("https://example.com");
    await page.locator(SUBMIT_BUTTON).click();

    await expect(page.locator(".bookmark")).toHaveCount(1);
    // Check the property: an empty paragraph has no box, so toBeHidden() proves nothing.
    await expect(error(page)).toHaveJSProperty("hidden", true);
    await expect(error(page)).toHaveText("");
    await expect(page.locator("#bookmark-form [aria-invalid]")).toHaveCount(0);
    await expect(page.locator("#bookmark-form [aria-describedby]")).toHaveCount(
      0,
    );
  });
});

test.describe("Phone layout (375x640)", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 640 });
    await page.goto("/");
    await addBookmark(page, MDN);
    await addBookmark(page, EXAMPLE);
  });

  test("the form is a single column with each label above its field", async ({
    page,
  }) => {
    await openForm(page);
    const tracks = await page.evaluate(
      () =>
        getComputedStyle(
          document.getElementById("bookmark-form"),
        ).gridTemplateColumns.split(" ").length,
    );
    expect(tracks).toBe(1);

    for (const id of ["url", "title", "notes", "tags"]) {
      const label = await page.locator(`label[for="${id}"]`).boundingBox();
      const field = await page.locator(`#${id}`).boundingBox();
      expect(
        label.y + label.height,
        `${id} label above field`,
      ).toBeLessThanOrEqual(field.y + 1);
      expect(
        Math.abs(label.x - field.x),
        `${id} label and field share a left edge`,
      ).toBeLessThan(1);
    }
  });

  test("the search box sits above the chips and spans the panel width", async ({
    page,
  }) => {
    const search = await page.locator("#search").boundingBox();
    const chips = await page.locator("#tag-filter").boundingBox();
    expect(search.y + search.height).toBeLessThanOrEqual(chips.y + 1);

    const contentWidth = await page.evaluate(() => {
      const panel = document.getElementById("search").closest(".panel");
      const style = getComputedStyle(panel);
      return (
        panel.clientWidth -
        parseFloat(style.paddingLeft) -
        parseFloat(style.paddingRight)
      );
    });
    expect(Math.abs(search.width - contentWidth)).toBeLessThanOrEqual(1);
  });

  test("the date sits below the title", async ({ page }) => {
    const first = page.locator(".bookmark").first();
    const title = await first.locator(".bookmark-title").boundingBox();
    const date = await first.locator(".bookmark-date").boundingBox();
    expect(date.y).toBeGreaterThanOrEqual(title.y + title.height - 1);
  });

  test("buttons, chips and inputs are at least 44px tall", async ({ page }) => {
    await openForm(page);
    const tooShort = await page.evaluate(() =>
      [...document.querySelectorAll("button, input, textarea")]
        .map((el) => ({ el, height: el.getBoundingClientRect().height }))
        .filter(({ height }) => height > 0 && height < 43.5)
        .map(
          ({ el, height }) =>
            `${el.tagName}#${el.id}.${el.className} ${height}px`,
        ),
    );
    expect(tooShort).toEqual([]);
  });

  test("nothing scrolls sideways, with the form open or closed", async ({
    page,
  }) => {
    const overflow = () =>
      page.evaluate(
        () =>
          document.documentElement.scrollWidth -
          document.documentElement.clientWidth,
      );
    expect(await overflow()).toBeLessThanOrEqual(0);
    await openForm(page);
    expect(await overflow()).toBeLessThanOrEqual(0);
  });
});

test.describe("Keyboard and focus rings", () => {
  const toggle = (page) => page.locator("#form-toggle");
  const outlineStyle = (page) =>
    page.evaluate(() => getComputedStyle(document.activeElement).outlineStyle);

  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await addBookmark(page, MDN);
  });

  test("Enter on the toggle opens the form and focuses the URL field", async ({
    page,
  }) => {
    await toggle(page).focus();
    await page.keyboard.press("Enter");
    await expect(toggle(page)).toHaveAttribute("aria-expanded", "true");
    await expect(page.getByLabel("URL")).toBeFocused();
    await expect(page.locator("#bookmark-form")).toBeVisible();
  });

  test("Escape closes the form and returns focus to the toggle", async ({
    page,
  }) => {
    await openForm(page);
    await page.getByLabel("Title").focus();
    await page.keyboard.press("Escape");
    await expect(toggle(page)).toHaveAttribute("aria-expanded", "false");
    await expect(toggle(page)).toBeFocused();
    await expect(page.locator("#bookmark-form")).toBeHidden();
  });

  test("a control focused by keyboard has a visible outline", async ({
    page,
    browserName,
  }) => {
    const { tab, shiftTab } = tabKeys(browserName);
    await page.getByLabel("Search bookmarks").focus();
    expect(await outlineStyle(page)).not.toBe("none");

    await page.keyboard.press(shiftTab);
    await expect(toggle(page)).toBeFocused();
    expect(await outlineStyle(page)).not.toBe("none");

    await page.keyboard.press(tab); // search box
    await page.keyboard.press(tab); // the "All" chip
    await expect(page.locator('#tag-filter .chip[data-tag=""]')).toBeFocused();
    expect(await outlineStyle(page)).not.toBe("none");
  });
});

test.describe("Reduced motion", () => {
  const toggle = (page) => page.locator("#form-toggle");

  test.beforeEach(async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/");
  });

  test("opening the form shows it at full height straight away", async ({
    page,
  }) => {
    await toggle(page).click();
    // No waiting: the form must already be settled when the click returns.
    const state = await page.evaluate(() => {
      const form = document.getElementById("bookmark-form");
      return {
        hidden: form.hidden,
        height: form.style.height,
        overflow: form.style.overflow,
        offsetHeight: form.offsetHeight,
        scrollHeight: form.scrollHeight,
      };
    });
    expect(state.hidden).toBe(false);
    expect(state.height).toBe("");
    expect(state.overflow).toBe("");
    expect(state.offsetHeight).toBeGreaterThan(0);
    expect(state.offsetHeight).toBe(state.scrollHeight);
  });

  test("the toggle has a single .roll-text span, with no leftover rolling text", async ({
    page,
  }) => {
    await toggle(page).click();
    await expect(page.locator("#form-toggle-label .roll-text")).toHaveCount(1);
    await expect(page.locator("#form-toggle-label .roll-text")).toHaveText(
      "Close form",
    );
    await toggle(page).click();
    await expect(page.locator("#form-toggle-label .roll-text")).toHaveCount(1);
    await expect(page.locator("#form-toggle-label .roll-text")).toHaveText(
      "Add a bookmark",
    );
  });

  test("the + icon has no transition", async ({ page }) => {
    const duration = await page.evaluate(
      () =>
        getComputedStyle(document.querySelector(".form-toggle-icon"))
          .transitionDuration,
    );
    expect(duration).toBe("0s");
  });
});

test.describe("Saved message", () => {
  test('shows "Saved" when a search hides the new bookmark', async ({
    page,
  }) => {
    await page.goto("/");
    await addBookmark(page, EXAMPLE);
    await page.getByLabel("Search bookmarks").fill("pasta");
    await addBookmark(page, {
      url: "https://example.com/soup",
      title: "Soup",
      notes: "",
      tags: "",
    });
    await expect(page.locator("#save-status")).toHaveText("Saved");
    await expect(page.locator(".bookmark")).toHaveCount(1);
  });

  test("shows nothing when no search or filter is active", async ({ page }) => {
    await page.goto("/");
    await addBookmark(page, EXAMPLE);
    await expect(page.locator("#save-status")).toBeHidden();
  });
});

test.describe("R1 malformed stored data", () => {
  const good = {
    id: "1",
    url: "https://a.com",
    title: "Good",
    notes: "n",
    tags: ["x"],
    createdAt: "2026-10-01T00:00:00.000Z",
  };

  /** Seed storage before the app script runs, then load the page. */
  async function seed(page, raw) {
    await page.addInitScript((value) => {
      if (!sessionStorage.getItem("seeded")) {
        localStorage.setItem("bookmarks", value);
        sessionStorage.setItem("seeded", "1");
      }
    }, raw);
    await page.goto("/");
  }

  const cases = {
    "invalid JSON": "{not json",
    "missing tags/notes": JSON.stringify([
      { id: "1", url: "https://a.com", title: "Good", createdAt: good.createdAt },
    ]),
    "null entry": JSON.stringify([good, null]),
    "javascript: URL": JSON.stringify([
      good,
      { ...good, id: "2", url: "javascript:alert(1)", title: "Bad" },
    ]),
  };

  for (const [name, raw] of Object.entries(cases)) {
    test(`${name} does not break the app`, async ({ page }) => {
      const errors = [];
      page.on("pageerror", (e) => errors.push(e.message));
      await seed(page, raw);
      if (name === "invalid JSON") {
        await expect(page.getByText("No bookmarks yet. Add your first one above.")).toBeVisible();
      } else {
        await expect(page.locator(".bookmark-title")).toHaveText(["Good"]);
      }
      expect(errors).toEqual([]);
      const hrefs = await page
        .locator("a")
        .evaluateAll((as) => as.map((a) => a.getAttribute("href") ?? ""));
      expect(hrefs.some((h) => h.startsWith("javascript:"))).toBe(false);
    });
  }
});

// ---------------------------------------------------------------------------
// Guide deviations: 05-first-build.md Layer 6 asks for "dark background with
// light text, dark mode", so the page is dark whatever the OS setting is.
// ---------------------------------------------------------------------------

test.describe("Guide: dark colour scheme", () => {
  /** Relative luminance of an "rgb(r, g, b)" string, 0 (black) to 1 (white). */
  const luminance = (css) => {
    const [r, g, b] = css
      .match(/[\d.]+/g)
      .slice(0, 3)
      .map((v) => {
        const c = Number(v) / 255;
        return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
      });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  };

  for (const scheme of ["light", "dark", "no-preference"]) {
    test(`${scheme} OS setting: dark background with light text`, async ({
      page,
    }) => {
      await page.emulateMedia({ colorScheme: scheme });
      await page.goto("/");
      const colours = await page.evaluate(() => {
        const body = getComputedStyle(document.body);
        const panel = getComputedStyle(document.querySelector(".panel"));
        return {
          bodyBg: body.backgroundColor,
          panelBg: panel.backgroundColor,
          text: body.color,
          scheme: getComputedStyle(document.documentElement).colorScheme,
        };
      });
      expect(luminance(colours.bodyBg)).toBeLessThan(0.05);
      expect(luminance(colours.panelBg)).toBeLessThan(0.05);
      expect(luminance(colours.text)).toBeGreaterThan(0.6);
      expect(colours.scheme).toBe("dark");
    });
  }
});

// ---------------------------------------------------------------------------
// Guide deviations: 05-first-build.md Step 3 asks for "Plain HTML, CSS, and
// JavaScript in a single file called index.html".
// ---------------------------------------------------------------------------

test.describe("Guide: single index.html", () => {
  test("the page loads no separate stylesheet or script", async ({ page }) => {
    /** @type {string[]} */
    const requested = [];
    page.on("request", (request) => {
      const url = new URL(request.url());
      if (url.origin === "http://localhost:3000" && url.pathname !== "/favicon.ico")
        requested.push(url.pathname);
    });
    await page.goto("/");
    await expect(
      page.getByText("No bookmarks yet. Add your first one above."),
    ).toBeVisible();

    await expect(page.locator('link[rel="stylesheet"], script[src]')).toHaveCount(0);
    expect(requested).toEqual(["/"]);
  });

  test("opening index.html straight from disk works, styled", async ({
    page,
  }) => {
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto(`file://${path.join(__dirname, "..", "index.html")}`);

    await expect(
      page.getByText("No bookmarks yet. Add your first one above."),
    ).toBeVisible();
    await openForm(page);
    await expect(page.getByLabel("URL")).toBeFocused();
    const background = await page.evaluate(
      () => getComputedStyle(document.body).backgroundColor,
    );
    expect(background).toBe("rgb(15, 23, 42)"); // --bg from the inline styles
    expect(errors).toEqual([]);
  });
});
