---
name: read-results
description: Read a louped run's results and report them honestly. Use after a run finishes, or when the person asks what a run or experiment found.
---

1. Find the run: call `runs` (filter by experiment), then `run <id>` for its metrics, parameters
   and files.
2. Read the files the run wrote:
   - Read `report.md` first, then the per-item records.
   - For an eval, `samples` and `sample` give each transcript and score.
   - The person changed the files named in the run's `louped.edited` tag after the run. When you
     quote them, say so.
3. Before any number, say what it rests on: the denominator, which items, which model revision.
   A rate on fewer than ~30 items is a pilot; say so.
4. Compare two runs only with `compare <a> <b>`. Report the difference with its paired interval
   and the run ids. Do not compare two runs on different items.
   - Within one run, `cohort` gives each condition's paired difference from the reference on any
     items. These can be the items the person picked (`ui_selection`'s `items/row/<id>` parts).
   - A run tagged `louped.cohort` ran on a cohort, not on every item. Say which.
   - An eval sample's `degenerate` lists what is wrong with its reply: `repeat` (an earlier
     assistant turn said again), `echo` (system prompt or template text), `loop`. Report the
     share of each before a score: a degenerate model can score well by chance.
   - A training run tagged `louped.alarm` looked degenerate: its data's flags, or a loss or DPO
     accuracy that was done too early. Say so before any eval of the model it trained.
     `louped data verify --src <set.jsonl>` flags a training set's pairs.
5. Read examples, not only rates. From the records, name at least one item where the behavior
   happened and one where it did not. Check the scoring on them.
   - A score's `read_by` and `matched` say which rule read the verdict from the reply. `none`
     means no rule matched. Check that the rule read the reply as a person would.
   - In `samples`, `disagree` marks the samples where two readers gave different values. Read
     those first: a misread answer shows there.
   - A `louped/` model's `sample` has `inputs`: the exact text the model read, special tokens
     included. Check the chat template there before you blame the model.
6. Separate what the result shows from what it suggests. Do not call a pilot result a finding.
   Do not infer intent from an answer.
7. Point the person to the run page (Items tab) for the rest. Write any report with
   `write-report`.
