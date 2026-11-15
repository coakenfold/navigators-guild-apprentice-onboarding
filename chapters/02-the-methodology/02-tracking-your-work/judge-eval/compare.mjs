// Compare saved results as markdown tables.
// Usage: node compare.mjs [model ...]   (no models: every file in results/)
import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";

const dir = new URL("./results/", import.meta.url);
const wanted = process.argv.slice(2);
let runs = readdirSync(dir).filter((f) => f.endsWith(".json")).map((f) => JSON.parse(readFileSync(new URL(f, dir))));
if (wanted.length) runs = runs.filter((r) => wanted.includes(r.summary.model));
if (!runs.length) { console.error("no matching results; run: node run.mjs <model>"); process.exit(1); }

// Wrong passes are the costliest mistake, so they rank first; then correct answers; then fewer holds.
runs.sort((a, b) => a.summary.wrong_pass - b.summary.wrong_pass || b.summary.correct - a.summary.correct || a.summary.unsure - b.summary.unsure);

const current = createHash("sha256").update(readFileSync(new URL("./cases.mjs", import.meta.url))).digest("hex").slice(0, 12);
const stale = runs.filter((r) => r.summary.cases_hash !== current).map((r) => r.summary.model);
if (stale.length) console.log(`> Warning: run on a different case set than the current cases.mjs, so not directly comparable: ${stale.join(", ")}. Re-run them.\n`);

console.log("| Rank | Model | Wrong pass | Correct | Unsure (held) | Wrong fail | Error | Cost (list price) | Median time | Run on |");
console.log("|---|---|---|---|---|---|---|---|---|---|");
runs.forEach(({ summary: s }, i) => {
  console.log(`| ${i + 1} | \`${s.model}\` | ${s.wrong_pass} | ${s.correct}/${s.cases} | ${s.unsure} | ${s.wrong_fail} | ${s.error} | $${s.cost_usd.toFixed(3)} | ${s.median_secs.toFixed(1)}s | ${s.date?.slice(0, 10) ?? "?"} |`);
});

const groups = [...new Set(runs[0].results.map((r) => r.group))];
console.log(`\n| Group (correct / cases) | ${runs.map((r) => `\`${r.summary.model}\``).join(" | ")} |`);
console.log(`|---|${runs.map(() => "---").join("|")}|`);
for (const g of groups) {
  const cells = runs.map(({ results }) => {
    const rs = results.filter((r) => r.group === g);
    const bad = rs.filter((r) => r.outcome === "WRONG PASS").length;
    return `${rs.filter((r) => r.outcome === "correct").length}/${rs.length}${bad ? ` (**${bad} wrong pass**)` : ""}`;
  });
  console.log(`| ${g} | ${cells.join(" | ")} |`);
}

const ids = runs[0].results.map((r) => r.id);
const label = { correct: "ok", unsure: "unsure", "WRONG PASS": "**WRONG PASS**", "wrong fail": "wrong fail", error: "error" };
console.log(`\nCases that any model got wrong or was unsure about (the answer shown):\n`);
console.log(`| Case | Expect | ${runs.map((r) => `\`${r.summary.model}\``).join(" | ")} |`);
console.log(`|---|---|${runs.map(() => "---").join("|")}|`);
for (const id of ids) {
  const row = runs.map(({ results }) => results.find((r) => r.id === id));
  if (row.every((r) => r?.outcome === "correct")) continue;
  const cells = row.map((r) => (r ? `${label[r.outcome]}${r.answer ? ` (${r.answer})` : ""}` : "–"));
  console.log(`| ${id} | ${row[0].expect} | ${cells.join(" | ")} |`);
}
