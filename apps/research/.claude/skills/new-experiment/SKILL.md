---
name: new-experiment
description: Start a research experiment in a louped project. Use when the person wants to test an idea, run a study, or reproduce a paper.
---

1. Read what exists first: use the `experiments` MCP tool, or read `experiments/`. If an
   experiment already asks the question, extend it. Do not start another.
2. Pick the domain. Use one that the project's `louped.toml` lists. If it lists none, use louped's
   own (mechanisms, honesty, conditioning, agents; context, inference, specialisation;
   reproduction).
3. Create it with the `new_experiment` MCP tool (or `louped new <name> --domain <domain>`). Name it
   for the question, in lowercase letters, digits and `-`. It starts `active`.
4. Fill the README before the code, one or two sentences a section:
   - **Question**: comes out yes or no, or as a number.
   - **Observation**: what was seen, apart from what it is thought to mean.
   - **Hypotheses**: including the ones that compete with the person's.
   - **Baseline**: the nearest existing method, and the simplest thing that might already work.
   - **Test**: the conditions, the metric, the seeds. Correctness and behavior metrics are named
     apart and never averaged.
   - **Stop if**: the result that would weaken the idea.
5. Write `run.py`:
   1. Build it from existing tools (transformers, nnsight, Inspect, TRL), and from `louped` where it
      has the piece. `Args` is the Launch form.
   2. Inside `start_run`, log numbers with `mlflow.log_metrics`.
   3. Inside `start_run`, write per-item records as `raw/<condition>.jsonl`: one file per
      condition, an id field in every row. Thus the run page lines them up.
   4. Inside `start_run`, write a `report.md` for people.
   5. Put the item itself in every row (its question or claim, its gold answer). Thus an item
      opens on what it is.
   6. Write `raw/fields.json`, `{"field": "what it means"}`, for each field's ?.
   7. Give `Args` a `cohort: str | None = None` option that keeps only
      `louped.tracking.cohort_ids(EXPERIMENT, args.cohort)` (ids as text). Thus items picked on the
      run page can be run again alone.

   A paper's harness with its own pins runs in its own environment, not this one.
6. Run a small check here first (`launch` then `job`, or the app's Launch page).
7. Read the check with `read-results`.
8. Do the real run. If the model is large, run it on a cluster (`run-elsewhere`).
9. Write the Result (model, date, numbers with denominators) and Next.
10. Set `status: answered` only when a real model's result answers the question.
