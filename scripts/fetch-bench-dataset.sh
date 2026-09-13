#!/usr/bin/env bash
# Download the HotpotQA dev distractor set and convert a subset of it into the question file the
# end-to-end benchmark reads (apps/engine/serving/benches/rag_e2e.rs).
#
# Usage: scripts/fetch-bench-dataset.sh [QUESTIONS] [OUT]
#   QUESTIONS  questions kept from the start of the file, default 500
#   OUT        output path, default target/bench/hotpotqa-dev-distractor.jsonl
#
# Each output line is one question:
#   id, question, answers (a list holding the answer), passages (a list of id, text, gold)
# A passage id is its paragraph title; gold marks the titles named by supporting_facts.
#
# The download is checked against EXPECTED_SHA256 and the script stops on a mismatch.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

URL="http://curtis.ml.cmu.edu/datasets/hotpot/hotpot_dev_distractor_v1.json"
# All-zero placeholder; the check fails until it holds the sha256 of hotpot_dev_distractor_v1.json.
EXPECTED_SHA256="0000000000000000000000000000000000000000000000000000000000000000"

questions="${1:-500}"
out="${2:-target/bench/hotpotqa-dev-distractor.jsonl}"
cache="target/bench/hotpot_dev_distractor_v1.json"

command -v curl >/dev/null || { echo "curl not found" >&2; exit 1; }
command -v python3 >/dev/null || { echo "python3 not found" >&2; exit 1; }
command -v sha256sum >/dev/null || { echo "sha256sum not found" >&2; exit 1; }
[[ "$questions" =~ ^[1-9][0-9]*$ ]] || { echo "QUESTIONS must be a positive integer, got $questions" >&2; exit 1; }

mkdir -p "$(dirname "$cache")" "$(dirname "$out")"
if [ ! -f "$cache" ]; then
  echo "downloading $URL"
  curl --fail --location --silent --show-error --output "$cache.partial" "$URL"
  mv "$cache.partial" "$cache"
fi

actual="$(sha256sum "$cache" | cut -d' ' -f1)"
if [ "$actual" != "$EXPECTED_SHA256" ]; then
  {
    echo "sha256 mismatch for $cache"
    echo "  expected $EXPECTED_SHA256"
    echo "  actual   $actual"
    echo "If EXPECTED_SHA256 is still the all-zero placeholder, verify the file and pin its hash in"
    echo "scripts/fetch-bench-dataset.sh. Otherwise delete $cache and download again."
  } >&2
  exit 1
fi

python3 - "$cache" "$out" "$questions" <<'PY'
import json
import sys

source, out, limit = sys.argv[1], sys.argv[2], int(sys.argv[3])
with open(source, encoding="utf-8") as f:
    records = json.load(f)

written = 0
with open(out, "w", encoding="utf-8") as f:
    for record in records:
        if written >= limit:
            break
        answer = record["answer"].strip()
        if not answer:
            continue
        gold_titles = {title for title, _ in record["supporting_facts"]}
        passages = []
        seen = set()
        for title, sentences in record["context"]:
            if title in seen:
                continue
            seen.add(title)
            passages.append({
                "id": title,
                "text": f"{title}: {''.join(sentences).strip()}",
                "gold": title in gold_titles,
            })
        if not any(p["gold"] for p in passages):
            continue
        f.write(json.dumps({
            "id": record["_id"],
            "question": record["question"],
            "answers": [answer],
            "passages": passages,
        }, ensure_ascii=False) + "\n")
        written += 1

print(f"wrote {written} questions to {out}")
PY
