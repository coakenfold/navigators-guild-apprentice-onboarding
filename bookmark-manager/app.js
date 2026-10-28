const STORAGE_KEY = "bookmarks";

const form = document.getElementById("bookmark-form");
const formError = document.getElementById("form-error");
const list = document.getElementById("bookmark-list");
const emptyMessage = document.getElementById("empty-message");

let bookmarks = loadBookmarks();

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

  bookmarks.push({
    id: Date.now().toString(),
    url,
    title,
    notes: form.notes.value.trim(),
    tags: parseTags(form.tags.value),
    createdAt: new Date().toISOString(),
  });

  saveBookmarks();
  clearError();
  form.reset();
  render();
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

  header.append(link, date);

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
      const pill = document.createElement("span");
      pill.className = "tag";
      pill.textContent = tag;
      tags.append(pill);
    }
    item.append(tags);
  }

  return item;
}

function render() {
  list.innerHTML = "";

  const newestFirst = [...bookmarks].sort((a, b) =>
    b.createdAt.localeCompare(a.createdAt),
  );

  for (const bookmark of newestFirst) {
    list.append(createBookmarkItem(bookmark));
  }

  emptyMessage.hidden = bookmarks.length > 0;
}

form.addEventListener("submit", handleSubmit);
render();
