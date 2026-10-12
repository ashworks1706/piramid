---
name: write-report
description: Write or revise a louped run's report.md or an experiment README's Result, short and with every claim traced to a run. Use whenever you write up results for the person.
---

A report is read by someone deciding what to do next. Give them the answer, the evidence and its
limits, and nothing else.

## Every claim is checked

1. Every number comes from a file or tool output you read in this session: `run`, `samples`,
   `compare`, the run's records. Next to it, put its ref: `run:<id>`, `run:<id>/<file>`, or the
   figure it is read from. If you cannot point to a source, leave the number out. In Markdown,
   write a run's metric as a live ref, `{{run:<id> <metric>}}` or `{{run:<id> <metric> :.1%}}`,
   not as a copied number, and a figure as `{{run:<id>/views/<name>.json}}` on its own line.
   The app reads them from the run, so they never go stale.
2. Give a rate with its count and denominator: "12/40 (30%)", not "30%". Under ~30 items, call it
   a pilot.
3. State a difference between conditions only from `compare` on the same items, with its paired
   interval. Otherwise say the runs are not comparable.
4. Cite a paper only if you read it in this session from `sources/` and pinned the passage.
   Cite it as `[@<key> p<page>]` (the ground-claims skill). Do not cite from memory or invent a
   reference. If a comparison to prior work needs a paper you have not read, write "not checked".
5. Quote examples from the records exactly, with their item id. Show one item where the effect
   happened and one where it did not. Do not pick only the striking ones.

## Say only what the evidence carries

- Separate what was measured from what it suggests. Give the interpretation its own sentence,
  starting with "This suggests" or "One reading is".
- Claim exactly as much as was tested: this model, this revision, these items, this setting. An
  effect on one model is not "LLMs do X".
- Do not infer intent or beliefs from outputs. Write "the model changed its answer", not "the
  model gave in" or "it knew".
- Put limitations in the report: cohort size, scoring checks, confounds, and anything you could
  not run.
- A null or negative result is a result. Report it at the same length as a positive one.

## Structure

Lead with the answer, then the evidence for each claim, then the limits, then what to run next.
Keep to the headings the template gives (an experiment README's Result and Next). A run's
`report.md` is:

```
# <the question, as asked>
<one or two sentences: the answer, with its headline number and denominator>

## Evidence
<one short paragraph or table per claim, each number with its source>

## Limits
<bullets>

## Next
<the one run that would change the conclusion most>
```

At most three claims. Put the rest in the records, the run page or a table.

## Plain, short sentences

- Use one idea per sentence and active voice, with a specific verb.
- Define each term once and keep using that term for the same thing.
- Begin each paragraph with its point.
- Use a table when you compare numbers, a list when you give steps or options, and prose for the
  reasoning that connects them.
- Cut anything that does not change what the reader concludes or does: restatements of the
  question, summaries of the summary, praise of the method, and "future work" lists.
- Avoid filler and hype words: delve, crucial, pivotal, robust (unless measured), comprehensive,
  notably, interestingly, it is worth noting, plays a key role, landscape, showcase, underscore,
  seamless, novel (unless checked against prior work).
- Avoid filler patterns: groups of three for rhythm, "not just X but Y", rhetorical questions,
  bold scattered through prose, emoji, and closing paragraphs that restate the report.

## Decks and documents

When the person asks for slides or a document, write it into `reports/` with python-pptx or
python-docx, from the run's files. First export each figure with `export_figure`. Then place the
file it returns. Put each number's ref in the slide's speaker notes, or in the document's
paragraph (above a table, for the table). Keep the deck to the same claims as the report.

File each report under the experiment it reports on, so the Reports page and the experiment's
page find it: put it in `reports/<experiment>/`, or start a Markdown file with front matter
`experiment: <name>`. Notes on papers and other files go in a folder named for what they are,
such as `reports/paper-notes/`. Live refs do not render in a deck or document: put the value
there, with its ref in the notes or paragraph.

## Before handing it over

Re-read it as a reviewer would:

- Run `check` on it and fix every issue.
- Check each number against its source.
- For each claim, ask what result would have contradicted it.
- Delete any sentence whose removal loses nothing.
- Then point the person to the run page for the items behind the numbers.

Sources: Google's Technical Writing One (developers.google.com/tech-writing/one), and Neel Nanda's
"Highly Opinionated Advice on How to Write ML Papers" (alignmentforum.org, 2025).
