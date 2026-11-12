(() => {
  "use strict";
  const STORAGE_KEY = "bookmarks";

  const form = document.getElementById("bookmark-form");
  const formError = document.getElementById("form-error");
  const list = document.getElementById("bookmark-list");
  const emptyMessage = document.getElementById("empty-message");
  const tagFilter = document.getElementById("tag-filter");
  const searchInput = document.getElementById("search");

  const NO_BOOKMARKS_TEXT = "No bookmarks yet. Add your first one above.";
  const NO_MATCH_TEXT = "No bookmarks match your search.";

  const formToggle = document.getElementById("form-toggle");
  const formToggleLabel = document.getElementById("form-toggle-label");
  const formCancel = document.getElementById("form-cancel");

  const submitButton = form.querySelector('button[type="submit"]');
  const formHeading = document.getElementById("form-heading");

  let bookmarks = loadBookmarks();
  let editingId = null;
  let selectedTag = null;
  // Ids shown by the last render; null until the first one, so nothing fades in on load.
  let renderedIds = null;

  // Read bookmarks from localStorage, falling back to an empty array.
  function loadBookmarks() {
    try {
      const data = JSON.parse(localStorage.getItem(STORAGE_KEY));
      return Array.isArray(data) ? data : [];
    } catch {
      return [];
    }
  }

  function saveBookmarks() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(bookmarks));
  }

  // "JavaScript, reference , javascript" -> ["javascript", "reference"]
  function parseTags(text) {
    const tags = text
      .split(",")
      .map((tag) => tag.trim().toLowerCase())
      .filter((tag) => tag !== "");
    return [...new Set(tags)];
  }

  function formatDate(isoString) {
    return new Date(isoString).toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  }

  // Returns an error message for an unusable URL, or '' if it is valid.
  function validateUrl(text) {
    if (!/^https?:\/\//i.test(text)) {
      return "URL must start with http:// or https://";
    }
    try {
      new URL(text);
    } catch {
      return "Enter a valid URL, for example https://example.com.";
    }
    if (/\s/.test(text)) {
      return "URL can't contain spaces. Enter it like https://example.com.";
    }
    return "";
  }

  // Show the message in the role="alert" region and tie it to the field.
  function showError(message, field) {
    formError.textContent = message;
    formError.hidden = false;
    if (field) {
      field.setAttribute("aria-invalid", "true");
      field.setAttribute("aria-describedby", formError.id);
      field.focus();
    }
  }

  function clearError() {
    formError.textContent = "";
    formError.hidden = true;
    for (const field of form.elements) {
      field.removeAttribute("aria-invalid");
      field.removeAttribute("aria-describedby");
    }
  }

  function handleSubmit(event) {
    event.preventDefault();

    const url = form.url.value.trim();
    const title = form.title.value.trim();

    clearError();

    if (!url || !title) {
      showError(
        "Please fill in both URL and Title.",
        url ? form.title : form.url,
      );
      return;
    }
    const urlError = validateUrl(url);
    if (urlError) {
      showError(urlError, form.url);
      return;
    }

    const fields = {
      url,
      title,
      notes: form.notes.value.trim(),
      tags: parseTags(form.tags.value),
    };

    const existing = bookmarks.find((bookmark) => bookmark.id === editingId);
    if (existing) {
      Object.assign(existing, fields);
    } else {
      bookmarks.push({
        id: Date.now().toString(),
        ...fields,
        createdAt: new Date().toISOString(),
      });
    }

    saveBookmarks();
    resetForm();
    setFormOpen(false);
    render();
    formToggle.focus();
  }

  const FORM_DURATION = 450;
  const FORM_FADE_RANGE = 28;
  let formOpen = false;
  let formAnimation = null;

  function easeInOutCubic(t) {
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
  }

  // Group the visible form children into rows and record where each row ends.
  function measureFormRows() {
    const rows = [];
    for (const child of form.children) {
      if (child.hidden) continue;
      const top = child.offsetTop - form.offsetTop;
      const bottom = top + child.offsetHeight;
      const row = rows.find((r) => r.top === top);
      if (row) {
        row.bottom = Math.max(row.bottom, bottom);
        row.elements.push(child);
      } else {
        rows.push({ top, bottom, elements: [child] });
      }
    }
    return rows;
  }

  // Slide the drawer to `open`. A row fades in once the drawer bottom passes the
  // row's bottom edge, and has fully faded out before the drawer bottom crosses it.
  function animateForm(open) {
    // Resume from the current height if interrupted mid-animation.
    const interruptedHeight = formAnimation ? formAnimation.height : null;
    if (formAnimation) cancelAnimationFrame(formAnimation.frame);
    formAnimation = null;

    const reduceMotion = window.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    form.hidden = false;
    // Closing fields leave the tab order at once, not when the animation ends.
    form.inert = !open;
    form.style.height = "";
    form.style.overflow = "";
    const rows = measureFormRows();
    const fullHeight = form.offsetHeight;
    const fadeRange = Math.min(FORM_FADE_RANGE, fullHeight);
    const scale = (fullHeight - fadeRange) / fullHeight;

    const finish = () => {
      formAnimation = null;
      form.style.height = "";
      form.style.overflow = "";
      form.style.marginTop = "";
      for (const row of rows) {
        for (const el of row.elements) el.style.opacity = "";
      }
      form.hidden = !open;
      // Add-mode names wait until the drawer is shut so the closing rows don't change.
      if (!open && editingId === null) setFormMode();
    };

    if (reduceMotion) {
      finish();
      return;
    }

    const startPoint = interruptedHeight ?? (open ? 0 : fullHeight);
    const endPoint = open ? fullHeight : 0;
    const startTime = performance.now();
    form.style.overflow = "hidden";

    const step = (now) => {
      const progress = Math.min((now - startTime) / FORM_DURATION, 1);
      const height =
        startPoint + (endPoint - startPoint) * easeInOutCubic(progress);
      form.style.height = `${height}px`;
      form.style.marginTop = `${12 * (height / fullHeight)}px`;
      for (const row of rows) {
        const threshold = row.bottom * scale;
        const opacity = Math.max(
          0,
          Math.min(1, (height - threshold) / fadeRange),
        );
        for (const el of row.elements) el.style.opacity = opacity;
      }
      if (progress < 1) {
        formAnimation = { frame: requestAnimationFrame(step), height };
      } else {
        finish();
      }
    };
    formAnimation = { frame: requestAnimationFrame(step), height: startPoint };
  }

  // Roll the toggle label like an alarm-clock drum: opening rolls the old text
  // down and out with the new text coming in from the top; closing reverses it.
  function rollToggleLabel(text, down) {
    const current = formToggleLabel.lastElementChild;
    if (current.textContent === text) return;

    const next = document.createElement("span");
    next.className = "roll-text";
    next.textContent = text;
    const reduceMotion = window.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    if (reduceMotion) {
      current.replaceWith(next);
      return;
    }

    // Drop any text still rolling out from an interrupted roll.
    for (const stale of formToggleLabel.querySelectorAll(".roll-out"))
      stale.remove();
    current.classList.add("roll-out");
    current.setAttribute("aria-hidden", "true");
    formToggleLabel.append(next);

    const options = { duration: 350, easing: "ease-in-out", fill: "both" };
    const exit = down ? "100%" : "-100%";
    const enter = down ? "-100%" : "100%";
    current.animate(
      [
        { transform: "translateY(0)", opacity: 1 },
        { transform: `translateY(${exit})`, opacity: 0 },
      ],
      options,
    ).onfinish = () => current.remove();
    next.animate(
      [
        { transform: `translateY(${enter})`, opacity: 0 },
        { transform: "translateY(0)", opacity: 1 },
      ],
      options,
    );
  }

  // Show or hide the add form; the toggle button always reflects the state.
  function setFormOpen(open) {
    if (open && editingId === null) setFormMode();
    if (open !== formOpen) animateForm(open);
    formOpen = open;
    formToggle.setAttribute("aria-expanded", open);
    rollToggleLabel(open ? "Close form" : "Add a bookmark", open);
    formToggle.classList.toggle("open", open);
  }

  function toggleForm() {
    const open = !formOpen;
    if (!open) resetForm();
    setFormOpen(open);
    if (open) form.url.focus({ preventScroll: true });
    else formToggle.focus();
  }

  function cancelForm() {
    resetForm();
    setFormOpen(false);
    formToggle.focus();
  }

  function handleFormKeydown(event) {
    if (event.key === "Escape") cancelForm();
  }

  // Name the form and its buttons for add mode, or for editing `title`.
  function setFormMode(title) {
    const editing = title !== undefined;
    formHeading.textContent = editing
      ? `Edit bookmark: ${title}`
      : "Add a bookmark";
    submitButton.textContent = editing ? "Save changes" : "Save";
    formCancel.textContent = editing ? "Discard changes" : "Cancel";
  }

  // Return the form to add mode and clear it. The add-mode heading and button
  // names are applied once the drawer has closed (see animateForm).
  function resetForm() {
    editingId = null;
    clearError();
    form.reset();
  }

  function startEdit(bookmark) {
    editingId = bookmark.id;
    clearError();
    form.url.value = bookmark.url;
    form.title.value = bookmark.title;
    form.notes.value = bookmark.notes;
    form.tags.value = bookmark.tags.join(", ");
    setFormMode(bookmark.title);
    setFormOpen(true);
    form.url.focus();
  }

  function deleteBookmark(id) {
    if (!confirm("Are you sure?")) return;

    const position = [...list.children].findIndex(
      (item) => item.dataset.id === id,
    );
    bookmarks = bookmarks.filter((bookmark) => bookmark.id !== id);
    saveBookmarks();
    if (id === editingId) {
      resetForm();
      setFormOpen(false);
    }
    render();

    // Focus the bookmark that took its place (or the previous one), else the toggle.
    const titles = list.querySelectorAll(".bookmark-title");
    const next = titles[Math.min(position, titles.length - 1)];
    (next ?? formToggle).focus();
  }

  // "https://www.example.com/a/b" -> "example.com"
  function getDomain(url) {
    try {
      return new URL(url).hostname.replace(/^www\./, "");
    } catch {
      return "";
    }
  }

  function createTagPill(tag, bookmarkId) {
    const pill = document.createElement("button");
    pill.type = "button";
    pill.className = "tag";
    pill.textContent = tag;
    // The name contains the visible text, as WCAG 2.5.3 (Label in Name) asks.
    pill.setAttribute("aria-label", `Filter by ${tag}`);
    pill.setAttribute("aria-pressed", selectedTag === tag);
    pill.classList.toggle("selected", selectedTag === tag);
    pill.dataset.id = bookmarkId;
    pill.dataset.tag = tag;
    pill.addEventListener("click", () => toggleTag(tag));
    return pill;
  }

  const bookmarkTemplate = document.getElementById("bookmark-template");

  function createBookmarkItem(bookmark) {
    const item = bookmarkTemplate.content.firstElementChild.cloneNode(true);
    item.dataset.id = bookmark.id;

    const link = item.querySelector(".bookmark-title");
    link.href = bookmark.url;
    link.textContent = bookmark.title;
    item.querySelector(".bookmark-domain").textContent = getDomain(
      bookmark.url,
    );
    item.querySelector(".bookmark-date").textContent = formatDate(
      bookmark.createdAt,
    );
    item.querySelector(".bookmark-url").textContent = bookmark.url;

    const notes = item.querySelector(".bookmark-notes");
    if (bookmark.notes) notes.textContent = bookmark.notes;
    else notes.remove();

    const tags = item.querySelector(".tags");
    if (bookmark.tags.length > 0) {
      tags.append(
        ...bookmark.tags.map((tag) => createTagPill(tag, bookmark.id)),
      );
    } else {
      tags.remove();
    }

    const editButton = item.querySelector(".edit-button");
    editButton.setAttribute("aria-label", `Edit ${bookmark.title}`);
    editButton.addEventListener("click", () => startEdit(bookmark));

    const deleteButton = item.querySelector(".delete-button");
    deleteButton.setAttribute("aria-label", `Delete ${bookmark.title}`);
    deleteButton.addEventListener("click", () => deleteBookmark(bookmark.id));

    return item;
  }

  // Selecting the current tag again clears it.
  function toggleTag(tag) {
    selectedTag = selectedTag === tag ? null : tag;
    render();
  }

  function createChip(label, tag) {
    const chip = document.createElement("button");
    chip.type = "button";
    chip.className = "tag chip";
    chip.textContent = label;
    chip.dataset.tag = tag ?? "";
    const active = selectedTag === tag;
    chip.classList.toggle("selected", active);
    chip.setAttribute("aria-pressed", active);
    chip.addEventListener("click", () =>
      tag === null ? clearTag() : toggleTag(tag),
    );
    return chip;
  }

  function clearTag() {
    selectedTag = null;
    render();
  }

  // Rebuild the "All" chip plus one chip per tag in use, sorted A-Z.
  function renderTagFilter() {
    const tags = [...new Set(bookmarks.flatMap((b) => b.tags))].sort();
    if (selectedTag !== null && !tags.includes(selectedTag)) selectedTag = null;

    tagFilter.innerHTML = "";
    tagFilter.append(createChip("All", null));
    for (const tag of tags) {
      tagFilter.append(createChip(tag, tag));
    }
  }

  // Case-insensitive match on title, URL and notes (not tags). query is lowercase.
  function matchesSearch(bookmark, query) {
    if (query === "") return true;
    return [bookmark.title, bookmark.url, bookmark.notes].some((text) =>
      text.toLowerCase().includes(query),
    );
  }

  // A selector for the focused chip or bookmark tag, or null for anything else.
  function focusedTagSelector() {
    const el = document.activeElement;
    if (!el.dataset || el.dataset.tag === undefined) return null;
    const tag = `[data-tag="${CSS.escape(el.dataset.tag)}"]`;
    if (tagFilter.contains(el)) return `#tag-filter ${tag}`;
    return `#bookmark-list [data-id="${CSS.escape(el.dataset.id)}"]${tag}`;
  }

  // Rebuilding the DOM drops focus, so put it back on the matching button.
  function render() {
    const refocus = focusedTagSelector();
    renderAll();
    if (refocus) document.querySelector(refocus)?.focus();
  }

  function renderAll() {
    list.innerHTML = "";
    renderTagFilter();

    const query = searchInput.value.trim().toLowerCase();
    const visible = bookmarks.filter(
      (bookmark) =>
        (selectedTag === null || bookmark.tags.includes(selectedTag)) &&
        matchesSearch(bookmark, query),
    );
    const newestFirst = [...visible].sort((a, b) =>
      b.createdAt.localeCompare(a.createdAt),
    );

    for (const bookmark of newestFirst) {
      const item = createBookmarkItem(bookmark);
      // Only bookmarks that weren't visible last time fade in.
      if (renderedIds && !renderedIds.has(bookmark.id))
        item.classList.add("entering");
      list.append(item);
    }
    renderedIds = new Set(newestFirst.map((bookmark) => bookmark.id));

    emptyMessage.textContent =
      bookmarks.length === 0 ? NO_BOOKMARKS_TEXT : NO_MATCH_TEXT;
    emptyMessage.hidden = newestFirst.length > 0;
  }

  form.addEventListener("submit", handleSubmit);
  form.addEventListener("keydown", handleFormKeydown);
  formToggle.addEventListener("click", toggleForm);
  formCancel.addEventListener("click", cancelForm);
  searchInput.addEventListener("input", render);
  render();
})();
