# Working in this research project

This is a louped project. Each research question about a language model is an experiment under
`experiments/`. louped runs it, records it and shows it. The person reads and judges results in
the app (`louped serve`, http://127.0.0.1:8000). You write experiments, launch them and read their
results.

## Tools

- `louped mcp`, registered in `.mcp.json`, drives the running app. Start `louped serve` first.
  Work started through it shows in the app.
  - To read: `experiments`, `runs`, `run`, `samples`, `figures`, `compare`.
  - To act: `new_experiment`, `launch`, `job`, `submit_job`, `export_job`, `import_result`,
    `push`, `pull`.
  - `experiment` shows an experiment's gate. A launch that the gate guards is refused until the
    gate passes. Do not edit a gate to get past it. Tell the person.
  - `ui_page`, `set_part`, `set_layout` and `set_preset` change what the app's pages show.
  - The person Shift+clicks any part of a page. `ui_selection` gives you those parts with their
    data, numbered as the app's tray numbers them. Call it when they say "this", "these" or
    `@sel 2`. In Claude Code, new picks also come with their message (the hook in
    `.claude/settings.json`).
  - `ui_show` points the person at parts, with a note.
  - `cohort` reads a run's conditions on a set of items, with paired intervals. `save_cohort` saves
    them for a run.py's `--cohort`.
- Skills in `.claude/skills/`: `new-experiment`, `read-results`, `run-elsewhere`, `write-report`,
  `ground-claims`, `add-plugin`, `change-ui`.
- The CLI does the same steps: `louped new`, `louped push`, `louped pull`, `louped --help`.
- Never ask for a Hugging Face token in chat. If push or pull says one is missing, ask the person
  to press Connect on Runs or to run `louped push` in a terminal. Both ask for the token and keep
  it out of the project.

## The method

1. A question is one experiment. `experiments/<question>/README.md` opens with front matter
   (`domain`, `status`). Then come Question, Observation, Hypotheses, Baseline, Test, Stop if, Run,
   Result, Next. Fill the README before the code.
2. `run.py` is the experiment's code. It has:
   - a tyro `Args` dataclass (the app's Launch form)
   - a `main` that runs inside `louped.tracking.start_run`, which records RunMeta (versions,
     commit, seed) with every result

   Log numbers with `mlflow.log_metrics` and files with `mlflow.log_artifact`. Write per-item
   records as one JSONL file per condition in one folder. The run page lines them up item by item.
   Put Inspect evals in `task.py`.
3. Compare conditions on the same items:
   - Fix the cohort from the baseline.
   - Report counts with their denominators.
   - Report a difference only with its paired interval (`compare`).
   - Name the run ids you used.
4. Pin what moves numbers: the model's revision, the seed, greedy decoding unless sampling is the
   point.
5. Send real runs on large models to a cluster (`run-elsewhere`). Use this machine for small
   checks.
6. Write the result in the README's Result (model, date, numbers) and the decision in Next, as
   `write-report` says. Trace every number to a run. Make no claim past what was tested.
   - Take papers from `sources/` and cite them from pinned passages (`ground-claims`).
   - `check` finds what is not grounded.
   - Put decks, documents and exported figures in `reports/`.
   - Set `status: answered` when a real model's result answers the question.

## Where things are

| Path              | What                                                  |
| ----------------- | ----------------------------------------------------- |
| `louped.toml`      | the project's root and settings (domains, remote, theme) |
| `layout.json`     | what the app's pages show (`change-ui`); committed     |
| `experiments/`    | the questions: README, scripts, notebooks; committed  |
| `sources/`        | papers, docs, slides, notebooks; committed            |
| `reports/`        | write-ups, decks, documents, exported figures; committed |
| `plugins/`        | the project's own pages, routes and tools; committed  |
| `judges/`         | judges that compare two eval runs (`new_judge`); committed |
| `.louped/`         | runs, eval logs, jobs, caches; gitignored, rebuildable |

## This project

The research behind Piramid, kept in the Piramid repository at `apps/research`. Piramid builds the
system; the experiments here decide whether a change to it is better. Read `../../docs/ROADMAP.md`
for the questions in flight.

- Piramid comes in as an OpenAI-compatible endpoint: `piramid serve` answers `/v1/chat/completions`
  on `127.0.0.1:6333`. Name the commit and the `piramid.yaml` a run used.
- Retrieval-before-prefill (passages in the prompt) is the control every arm is measured against.
- `just bench-rag` in the repository root is the quick check while iterating on the engine. A
  result that goes in the roadmap or the README comes from an experiment here.
- Nothing here is built or tested by the repository's `just check`.
