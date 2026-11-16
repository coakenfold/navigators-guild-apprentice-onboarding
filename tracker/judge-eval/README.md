# Judge model evaluation

Compares Claude models as the `tracker` judge (DESIGN.md §2.3.1) on 48 labelled cases: J1 explanation checks, injection attempts, error-message criteria, J3 maintainability and J2 design fit.

## Run

The `claude` CLI must be installed and logged in. Each question is one `claude -p` call, so a run is 48 calls.

```sh
node run.mjs <model>          # a full model name, e.g. node run.mjs claude-haiku-5-5
node compare.mjs              # every saved result
node compare.mjs <model> ...  # just these
```

`adapters/claude.mjs` turns a judge request into `claude -p` calls, the same way `tracker-judge` does: no tools, `--safe-mode` (no CLAUDE.md, hooks, skills or MCP servers), the judge's own system prompt, an empty working folder, and `--json-schema` so every reply is a one-sentence reason and `yes`, `no` or `unsure`. It fails if a different model answered than the one asked for.

## Results

Each run writes `results/<model>.json` with a summary (date, adapter, case-set fingerprint, cost at list price) and one row per case, including the model's reason. Re-running a model replaces its file. Results for models you've stopped using stay, so new candidates can be compared against them.

The CLI has no temperature setting, so a re-run can give a few different answers. Treat a difference of one or two cases between models as noise.

`compare.mjs` ranks models by wrong passes first, because a wrong pass is the mistake the tracker exists to prevent, then by correct answers, then by fewer unsure answers. It warns when results come from an older version of `cases.mjs`.

`results/ollama/` holds the 2026-11-13 runs of local Ollama models, from before judges moved to Claude. They use the old format (P(yes) from log probabilities), so `compare.mjs` doesn't read them.

## Outcomes

- **correct**: a yes or no on the right side
- **unsure**: the model said the material didn't show enough to decide, so the tracker would hold the item for a human (safe, but costs attention)
- **WRONG PASS**: bad work passed. The worst outcome.
- **wrong fail**: good work refused. Costs a dispute or a retry.
- **error**: no usable answer, such as `claude` not installed or logged in, a usage limit, or a different model answering

## Adding cases

Add to `cases.mjs`. Write the state as the judge would see it: comments already stripped, small excerpts. Keep yes and no answers roughly balanced, and re-run every model afterwards (the fingerprint changes, and `compare.mjs` will flag older results).
