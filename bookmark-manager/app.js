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

let bookmarks = loadBookmarks();
let editingId = null;
let selectedTag = null;

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
  let parsed;
  try {
    parsed = new URL(text);
  } catch {
    return "Enter a valid URL, for example https://example.com.";
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    return "URL must start with http:// or https://";
  }
  if (!parsed.hostname) {
    return "URL must include a website name, for example https://example.com.";
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

  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  form.hidden = false;
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
    const height = startPoint + (endPoint - startPoint) * easeInOutCubic(progress);
    form.style.height = `${height}px`;
    form.style.marginTop = `${12 * (height / fullHeight)}px`;
    for (const row of rows) {
      const threshold = row.bottom * scale;
      const opacity = Math.max(0, Math.min(1, (height - threshold) / fadeRange));
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
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduceMotion) {
    current.replaceWith(next);
    return;
  }

  // Drop any text still rolling out from an interrupted roll.
  for (const stale of formToggleLabel.querySelectorAll(".roll-out")) stale.remove();
  current.classList.add("roll-out");
  current.setAttribute("aria-hidden", "true");
  formToggleLabel.append(next);

  const options = { duration: 350, easing: "ease-in-out", fill: "both" };
  const exit = down ? "100%" : "-100%";
  const enter = down ? "-100%" : "100%";
  current.animate(
    [{ transform: "translateY(0)", opacity: 1 }, { transform: `translateY(${exit})`, opacity: 0 }],
    options,
  ).onfinish = () => current.remove();
  next.animate(
    [{ transform: `translateY(${enter})`, opacity: 0 }, { transform: "translateY(0)", opacity: 1 }],
    options,
  );
}

// Show or hide the add form; the toggle button always reflects the state.
function setFormOpen(open) {
  if (open !== formOpen) animateForm(open);
  formOpen = open;
  formToggle.setAttribute("aria-expanded", open);
  rollToggleLabel(
    open
      ? editingId === null
        ? "Close form"
        : "Editing bookmark"
      : "Add a bookmark",
    open,
  );
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

// Return the form to add mode and clear it.
function resetForm() {
  editingId = null;
  submitButton.textContent = "Save";
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
  submitButton.textContent = "Update";
  setFormOpen(true);
  form.url.focus();
}

function deleteBookmark(id) {
  if (!confirm("Are you sure?")) return;

  bookmarks = bookmarks.filter((bookmark) => bookmark.id !== id);
  saveBookmarks();
  if (id === editingId) {
    resetForm();
    setFormOpen(false);
  }
  render();
}

// "https://www.example.com/a/b" -> "example.com"
function getDomain(url) {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return "";
  }
}

// Trash can icon; decorative, the button's aria-label names the action.
function createTrashIcon() {
  const svgNs = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(svgNs, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", "18");
  svg.setAttribute("height", "18");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "2");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("focusable", "false");

  const paths = [
    "M3 6h18",
    "M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2",
    "M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6",
    "M10 11v6",
    "M14 11v6",
  ];
  for (const d of paths) {
    const path = document.createElementNS(svgNs, "path");
    path.setAttribute("d", d);
    svg.append(path);
  }
  return svg;
}

// Pencil icon; decorative, the button's aria-label names the action.
function createEditIcon() {
  const svgNs = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(svgNs, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", "18");
  svg.setAttribute("height", "18");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "2");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("focusable", "false");

  const paths = ["M12 20h9", "M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z"];
  for (const d of paths) {
    const path = document.createElementNS(svgNs, "path");
    path.setAttribute("d", d);
    svg.append(path);
  }
  return svg;
}

function createBookmarkItem(bookmark) {
  const item = document.createElement("li");
  item.className = "bookmark";

  const header = document.createElement("div");
  header.className = "bookmark-header";

  const link = document.createElement("a");
  link.className = "bookmark-title";
  link.href = bookmark.url;
  link.target = "_blank";
  link.rel = "noopener noreferrer";
  link.textContent = bookmark.title;

  const date = document.createElement("span");
  date.className = "bookmark-date";
  date.textContent = formatDate(bookmark.createdAt);

  const domain = document.createElement("span");
  domain.className = "bookmark-domain";
  domain.textContent = getDomain(bookmark.url);

  const heading = document.createElement("div");
  heading.className = "bookmark-heading";
  heading.append(link, domain);

  header.append(heading, date);

  const url = document.createElement("div");
  url.className = "bookmark-url";
  url.textContent = bookmark.url;

  item.append(header, url);

  if (bookmark.notes) {
    const notes = document.createElement("p");
    notes.className = "bookmark-notes";
    notes.textContent = bookmark.notes;
    item.append(notes);
  }

  if (bookmark.tags.length > 0) {
    const tags = document.createElement("div");
    tags.className = "tags";
    for (const tag of bookmark.tags) {
      const pill = document.createElement("button");
      pill.type = "button";
      pill.className = "tag";
      pill.textContent = tag;
      pill.addEventListener("click", () => toggleTag(tag));
      tags.append(pill);
    }
    item.append(tags);
  }

  const actions = document.createElement("div");
  actions.className = "bookmark-actions";

  const editButton = document.createElement("button");
  editButton.type = "button";
  editButton.className = "edit-button";
  editButton.title = "Edit";
  editButton.setAttribute("aria-label", `Edit ${bookmark.title}`);
  editButton.append(createEditIcon());
  editButton.addEventListener("click", () => startEdit(bookmark));

  const deleteButton = document.createElement("button");
  deleteButton.type = "button";
  deleteButton.className = "delete-button";
  deleteButton.title = "Delete";
  deleteButton.setAttribute("aria-label", `Delete ${bookmark.title}`);
  deleteButton.append(createTrashIcon());
  deleteButton.addEventListener("click", () => deleteBookmark(bookmark.id));

  actions.append(editButton, deleteButton);
  item.append(actions);

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

function render() {
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
    list.append(createBookmarkItem(bookmark));
  }

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
