(() => {
  "use strict";
  const STORAGE_KEY = "bookmarks";

  const form = document.getElementById("bookmark-form");
  const formError = document.getElementById("form-error");
  const list = document.getElementById("bookmark-list");
  const emptyMessage = document.getElementById("empty-message");
  const tagFilter = document.getElementById("tag-filter");
  const searchInput = document.getElementById("search");
  const saveStatus = document.getElementById("save-status");
  let saveStatusTimer = null;

  const NO_BOOKMARKS_TEXT = "No bookmarks yet. Add your first one above.";
  const NO_MATCH_TEXT = "No bookmarks match your search.";

  const formToggle = document.getElementById("form-toggle");
  const formToggleLabel = document.getElementById("form-toggle-label");
  const formCancel = document.getElementById("form-cancel");

  let bookmarks = loadBookmarks();
  // The bookmark being edited inline, and what has been typed so far. The draft
  // lets the edit survive a re-render (search, tag filter, another delete).
  let editingId = null;
  let editTitle = "";
  let editDraft = null;
  let editError = null;
  let selectedTag = null;
  // Ids shown by the last render; null until the first one, so nothing fades in on load.
  let renderedIds = null;

  // Read bookmarks from localStorage, falling back to an empty array.
  function loadBookmarks() {
    try {
      const data = JSON.parse(localStorage.getItem(STORAGE_KEY));
      return Array.isArray(data) ? data.map(normaliseRecord).filter(Boolean) : [];
    } catch {
      return [];
    }
  }

  // Returns a safe copy of a stored record, or null if it can't be used.
  function normaliseRecord(record) {
    if (!record || typeof record !== "object" || Array.isArray(record)) return null;
    const { id, url, title } = record;
    if (typeof id !== "string" || typeof url !== "string" || typeof title !== "string") {
      return null;
    }
    if (validateUrl(url) !== "") return null;
    const created = new Date(record.createdAt);
    return {
      id,
      url,
      title,
      notes: typeof record.notes === "string" ? record.notes : "",
      tags: Array.isArray(record.tags)
        ? record.tags.filter((tag) => typeof tag === "string")
        : [],
      createdAt:
        typeof record.createdAt === "string" && !Number.isNaN(created.getTime())
          ? record.createdAt
          : new Date().toISOString(),
    };
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

  // Show the message in a form's role="alert" region and tie it to the field.
  function showError(errorEl, message, field) {
    errorEl.textContent = message;
    errorEl.hidden = false;
    field.setAttribute("aria-invalid", "true");
    field.setAttribute("aria-describedby", errorEl.id);
  }

  function clearError(formEl, errorEl) {
    errorEl.textContent = "";
    errorEl.hidden = true;
    for (const field of formEl.elements) {
      field.removeAttribute("aria-invalid");
      field.removeAttribute("aria-describedby");
    }
  }

  // Read and check the add form or an edit form.
  // Returns { fields } when it is valid, or { message, field } when it isn't.
  function readFields(formEl) {
    const { elements } = formEl;
    const url = elements.url.value.trim();
    const title = elements.title.value.trim();
    if (!url || !title) {
      return {
        message: "Please fill in both URL and Title.",
        field: url ? elements.title : elements.url,
      };
    }
    const urlError = validateUrl(url);
    if (urlError) return { message: urlError, field: elements.url };
    return {
      fields: {
        url,
        title,
        notes: elements.notes.value.trim(),
        tags: parseTags(elements.tags.value),
      },
    };
  }

  const filterActive = () =>
    searchInput.value.trim() !== "" || selectedTag !== null;

  function handleSubmit(event) {
    event.preventDefault();
    clearError(form, formError);
    const result = readFields(form);
    if (result.message) {
      showError(formError, result.message, result.field);
      result.field.focus();
      return;
    }

    bookmarks.push({
      id: Date.now().toString(),
      ...result.fields,
      createdAt: new Date().toISOString(),
    });
    saveBookmarks();
    resetForm();
    setFormOpen(false);
    render();
    formToggle.focus();
    // The saved bookmark may be hidden by the search or tag filter,
    // so confirm the save in words.
    if (filterActive()) showSaved();
  }

  function showSaved() {
    clearTimeout(saveStatusTimer);
    saveStatus.textContent = "Saved";
    saveStatus.hidden = false;
    saveStatusTimer = setTimeout(() => {
      saveStatus.hidden = true;
      saveStatus.textContent = "";
    }, 3000);
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

  // Clear the add form. It is only for adding; editing happens inline.
  function resetForm() {
    clearError(form, formError);
    form.reset();
  }

  // Open the inline edit form in this bookmark's card (guide Layer 3).
  // Editing another bookmark switches to it and drops the unsaved edit.
  function startEdit(bookmark) {
    editingId = bookmark.id;
    editTitle = bookmark.title;
    editDraft = {
      url: bookmark.url,
      title: bookmark.title,
      notes: bookmark.notes,
      tags: bookmark.tags.join(", "),
    };
    editError = null;
    render();
    document.getElementById("edit-url")?.focus();
  }

  function stopEdit() {
    editingId = null;
    editDraft = null;
    editError = null;
  }

  // Focus a bookmark's Edit button, or the toggle if the bookmark isn't shown.
  function focusEditButton(id) {
    const button = list.querySelector(
      `.bookmark[data-id="${CSS.escape(id)}"] .edit-button`,
    );
    (button ?? formToggle).focus();
  }

  function cancelEdit() {
    const id = editingId;
    stopEdit();
    render();
    focusEditButton(id);
  }

  function handleEditSubmit(event) {
    event.preventDefault();
    const editForm = event.currentTarget;
    const errorEl = editForm.querySelector(".form-error");
    clearError(editForm, errorEl);
    editError = null;
    const result = readFields(editForm);
    if (result.message) {
      editError = { message: result.message, name: result.field.name };
      showError(errorEl, result.message, result.field);
      result.field.focus();
      return;
    }

    const id = editingId;
    const existing = bookmarks.find((bookmark) => bookmark.id === id);
    if (existing) Object.assign(existing, result.fields);
    saveBookmarks();
    stopEdit();
    render();
    focusEditButton(id);
    // The edited bookmark may no longer match the search or tag filter.
    if (filterActive()) showSaved();
  }

  function deleteBookmark(id) {
    if (!confirm("Are you sure?")) return;

    const position = [...list.children].findIndex(
      (item) => item.dataset.id === id,
    );
    bookmarks = bookmarks.filter((bookmark) => bookmark.id !== id);
    saveBookmarks();
    if (id === editingId) stopEdit();
    render();

    // Focus the bookmark that took its place (or the previous one), else the toggle.
    // A card being edited has no title link, so focus its first field instead.
    const next = list.children[Math.min(position, list.children.length - 1)];
    (next?.querySelector(".bookmark-title, input") ?? formToggle).focus();
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

  const editTemplate = document.getElementById("edit-template");

  // The card for the bookmark being edited: the inline form, filled from the draft.
  function createEditItem(bookmark) {
    const item = document.createElement("li");
    item.className = "bookmark editing";
    item.dataset.id = bookmark.id;

    const editForm = editTemplate.content.firstElementChild.cloneNode(true);
    // The heading keeps the title from when the edit started.
    editForm.querySelector(".form-heading").textContent =
      `Edit bookmark: ${editTitle}`;
    for (const [name, value] of Object.entries(editDraft)) {
      editForm.elements[name].value = value;
    }
    if (editError) {
      showError(
        editForm.querySelector(".form-error"),
        editError.message,
        editForm.elements[editError.name],
      );
    }

    editForm.addEventListener("input", (event) => {
      editDraft[event.target.name] = event.target.value;
    });
    editForm.addEventListener("submit", handleEditSubmit);
    editForm.addEventListener("keydown", (event) => {
      if (event.key === "Escape") cancelEdit();
    });
    editForm
      .querySelector(".edit-cancel")
      .addEventListener("click", cancelEdit);

    item.append(editForm);
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
      const item =
        bookmark.id === editingId
          ? createEditItem(bookmark)
          : createBookmarkItem(bookmark);
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
