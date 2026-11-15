// Run every case against one Claude model and save the results to results/<model>.json.
// Usage: node run.mjs <model>   (a full model name, such as claude-haiku-5-5)
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { cases } from "./cases.mjs";

const [model] = process.argv.slice(2);
if (!model) { console.error("usage: node run.mjs <model>   (a full model name, such as claude-haiku-5-5)"); process.exit(1); }
const adapterPath = new URL("./adapters/claude.mjs", import.meta.url).pathname;

const casesHash = createHash("sha256").update(readFileSync(new URL("./cases.mjs", import.meta.url))).digest("hex").slice(0, 12);
console.log(`${model}, ${cases.length} cases (case set ${casesHash})\n`);

const results = [];
let adapter = null;
for (const c of cases) {
  const passWhen = c.pass_when ?? "yes";
  const question = { instructions: c.question, ...(c.criteria && { criteria: c.criteria }) };
  const request = { model, state: c.state, questions: { q: question } };
  const started = Date.now();
  let answer = null, reason = null, cost = null, error = null;
  try {
    const out = JSON.parse(execFileSync("node", [adapterPath], { input: JSON.stringify(request), timeout: 120_000, stdio: ["pipe", "pipe", "pipe"] }));
    ({ answer, reason } = out.answers.q);
    cost = out.cost_usd;
    adapter = out.adapter;
  } catch (e) {
    error = (e.stderr?.toString() || e.message).trim();
  }
  const secs = (Date.now() - started) / 1000;

  // Outcome as the tracker would see it: an unsure answer holds; otherwise the answer passes or fails the work.
  let outcome;
  if (answer === null) outcome = "error";
  else if (answer === "unsure") outcome = "unsure";
  else {
    const passes = answer === passWhen;
    outcome = passes === (c.expect === passWhen) ? "correct" : passes ? "WRONG PASS" : "wrong fail";
  }
  results.push({ id: c.id, group: c.group, expect: c.expect, pass_when: passWhen, answer, outcome, reason, secs, cost_usd: cost, error, note: c.note });
  console.log(`${outcome.padEnd(10)} ${(answer ?? "-").padEnd(6)} expect ${c.expect.padEnd(3)} ${secs.toFixed(1).padStart(5)}s  ${c.group.padEnd(18)} ${c.id}${error ? `  (${error})` : ""}`);
}

const count = (o) => results.filter((r) => r.outcome === o).length;
const summary = {
  model, adapter, date: new Date().toISOString(), cases_hash: casesHash, cases: results.length,
  correct: count("correct"), unsure: count("unsure"), wrong_pass: count("WRONG PASS"), wrong_fail: count("wrong fail"), error: count("error"),
  cost_usd: results.reduce((s, r) => s + (r.cost_usd ?? 0), 0),
  median_secs: results.map((r) => r.secs).sort((a, b) => a - b)[Math.floor(results.length / 2)],
};
console.log(`\n${model}: ${summary.correct}/${summary.cases} correct, ${summary.unsure} unsure, ${summary.wrong_pass} wrong pass, ${summary.wrong_fail} wrong fail, ${summary.error} error; $${summary.cost_usd.toFixed(3)}; median ${summary.median_secs.toFixed(1)}s`);
const file = new URL(`./results/${model.replace(/[:/]/g, "_")}.json`, import.meta.url);
writeFileSync(file, JSON.stringify({ summary, results }, null, 2) + "\n");
console.log(`saved ${file.pathname}`);
