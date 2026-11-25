#!/usr/bin/env node
// Copy the HTML pages listed in a text file into ./github-pages, keeping each
// page's path from the repo root (so bookmark-manager/index.html is copied to
// github-pages/bookmark-manager/index.html).
//
// Usage: node scripts/copy-pages.mjs [list-file]   (default: github-pages.txt)
//
// The list has one repo-relative .html path per line. Blank lines and lines
// starting with # are ignored. Every entry is checked before anything is
// copied: if one is missing, outside the repo or not .html, nothing changes.
// github-pages/ is emptied first, so a page dropped from the list disappears.
// Only the listed files are copied, not any images, styles or scripts they link to.
//
// It also writes github-pages/index.html, a table of contents that links to
// every copied page, named by its <title> and grouped by top-level folder.
// So the list can't include a root-level index.html.

import { statSync } from "node:fs";
import { copyFile, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(root, "github-pages");
const listArg = process.argv[2] ?? "github-pages.txt";
const listFile = path.resolve(root, listArg);

async function main() {
  let text;
  try {
    text = await readFile(listFile, "utf8");
  } catch {
    fail([`Can't read the list file ${listArg}.`]);
  }

  const entries = [];
  const problems = [];
  text.split(/\r?\n/).forEach((raw, index) => {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) return;
    const where = `line ${index + 1} (${line})`;
    const source = path.resolve(root, line);
    const relative = path.relative(root, source);
    if (path.isAbsolute(line) || relative.startsWith("..") || relative === "") {
      problems.push(`${where}: give a path inside the repo, relative to its root.`);
    } else if (path.extname(source).toLowerCase() !== ".html") {
      problems.push(`${where}: only .html files can be listed.`);
    } else if (relative === "github-pages" || relative.startsWith(`github-pages${path.sep}`)) {
      problems.push(`${where}: that's inside the output folder.`);
    } else if (relative.toLowerCase() === "index.html") {
      problems.push(`${where}: github-pages/index.html is the generated table of contents.`);
    } else if (!statSync(source, { throwIfNoEntry: false })?.isFile()) {
      problems.push(`${where}: no such file.`);
    } else {
      entries.push({ source, relative });
    }
  });

  if (problems.length > 0) fail(problems);
  if (entries.length === 0) fail([`${listArg} lists no pages.`]);

  await rm(outDir, { recursive: true, force: true });
  const pages = [];
  for (const { source, relative } of entries) {
    if (pages.some((page) => page.relative === relative)) continue;
    const target = path.join(outDir, relative);
    await mkdir(path.dirname(target), { recursive: true });
    await copyFile(source, target);
    pages.push({ relative, title: titleOf(await readFile(source, "utf8")) });
    console.log(`copied ${relative}`);
  }
  await writeFile(path.join(outDir, "index.html"), tableOfContents(pages));
  console.log(
    `${pages.length} page${pages.length === 1 ? "" : "s"} in github-pages/, plus index.html`,
  );
}

// The text of a page's <title>, or "" if it has none.
function titleOf(html) {
  const match = /<title[^>]*>([\s\S]*?)<\/title>/i.exec(html);
  if (!match) return "";
  return match[1]
    .replace(/<[^>]*>/g, "")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, "&")
    .replace(/\s+/g, " ")
    .trim();
}

const escapeHtml = (text) =>
  text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");

// A link's href: the page's path with forward slashes, each part URL-encoded.
const hrefOf = (relative) =>
  relative.split(path.sep).map(encodeURIComponent).join("/");

// The table of contents: one section per top-level folder, in list order.
function tableOfContents(pages) {
  const groups = new Map();
  for (const page of pages) {
    const parts = page.relative.split(path.sep);
    const group = parts.length > 1 ? parts[0] : "Other pages";
    if (!groups.has(group)) groups.set(group, []);
    groups.get(group).push(page);
  }

  const sections = [...groups]
    .map(([group, items]) => {
      const links = items
        .map(({ relative, title }) => {
          const href = escapeHtml(hrefOf(relative));
          const shown = relative.split(path.sep).join("/");
          const name = escapeHtml(title || shown);
          return `        <li><a href="${href}">${name}</a> <span class="path">${escapeHtml(shown)}</span></li>`;
        })
        .join("\n");
      return `    <section>\n      <h2>${escapeHtml(group)}</h2>\n      <ul>\n${links}\n      </ul>\n    </section>`;
    })
    .join("\n");

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>guild-portfolio pages</title>
  <style>
    :root {
      color-scheme: light dark;
      --bg: #f7f7f8;
      --text: #1f2933;
      --muted: #5b6472;
      --accent: #1d4ed8;
      --rule: #dfe2e6;
    }
    @media (prefers-color-scheme: dark) {
      :root {
        --bg: #0f172a;
        --text: #e5e7eb;
        --muted: #a3acba;
        --accent: #60a5fa;
        --rule: #2d3748;
      }
    }
    body {
      margin: 0;
      font-family: system-ui, sans-serif;
      line-height: 1.5;
      color: var(--text);
      background: var(--bg);
    }
    main { max-width: 720px; margin: 0 auto; padding: 32px 16px; }
    h1 { margin: 0 0 4px; font-size: 1.8rem; }
    .intro { margin: 0 0 24px; color: var(--muted); }
    h2 { margin: 24px 0 8px; font-size: 1.1rem; border-bottom: 1px solid var(--rule); padding-bottom: 4px; }
    ul { list-style: none; margin: 0; padding: 0; }
    li { padding: 6px 0; overflow-wrap: anywhere; }
    a { color: var(--accent); font-weight: 600; }
    a:focus-visible { outline: 3px solid var(--accent); outline-offset: 2px; }
    .path { display: block; color: var(--muted); font-size: 0.85rem; font-family: ui-monospace, monospace; }
  </style>
</head>
<body>
  <main>
    <h1>guild-portfolio pages</h1>
    <p class="intro">The HTML pages published from this repository.</p>
${sections}
  </main>
</body>
</html>
`;
}

function fail(problems) {
  console.error("Nothing was copied:");
  for (const problem of problems) console.error(`  ${problem}`);
  process.exit(1);
}

await main();
