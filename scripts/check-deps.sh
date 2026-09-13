#!/usr/bin/env bash
# Verify the workspace dependency rule from docs/ARCHITECTURE.md.
# Run by just check-rust, the pre-commit hook, and CI.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

command -v jq >/dev/null || { echo "check-deps: jq is required"; exit 1; }

# Allowed in-repo dependencies, one edge per line. Any other edge is a violation.
ALLOWED=$(cat <<'EOF'
piramid-core -> piramid-hardware
piramid-database -> piramid-core
piramid-database -> piramid-hardware
piramid-model -> piramid-core
piramid-model -> piramid-hardware
piramid-serving -> piramid-core
piramid-serving -> piramid-hardware
piramid-serving -> piramid-database
piramid-serving -> piramid-model
piramid -> piramid-core
piramid -> piramid-hardware
piramid -> piramid-database
piramid -> piramid-model
piramid -> piramid-serving
EOF
)

# Every in-repo edge Cargo actually sees (path dependencies between workspace members).
ACTUAL=$(cargo metadata --format-version 1 --no-deps \
  | jq -r '.packages[] | .name as $from | .dependencies[]
           | select(.path != null)
           | "\($from) -> \(.name)"' \
  | sort -u)

status=0

while IFS= read -r edge; do
  [ -z "$edge" ] && continue
  if ! grep -Fxq "$edge" <<<"$ALLOWED"; then
    echo "FAIL undeclared dependency: $edge"
    echo "     if this edge is intended, add it to scripts/check-deps.sh and docs/ARCHITECTURE.md"
    status=1
  fi
done <<<"$ACTUAL"

# Leaf crates depend on no workspace crate.
for leaf in piramid-hardware; do
  if grep -q "^$leaf -> " <<<"$ACTUAL"; then
    echo "FAIL $leaf must be a leaf crate but depends on:"
    grep "^$leaf -> " <<<"$ACTUAL" | sed 's/^/       /'
    status=1
  fi
done

# piramid-model must not depend on piramid-database.
if grep -q "^piramid-model -> piramid-database\$" <<<"$ACTUAL"; then
  echo "FAIL piramid-model must not depend on piramid-database"
  echo "     a hook implementation belongs in its own crate depending on both"
  status=1
fi

if [ $status -eq 0 ]; then
  echo "dependency direction ok ($(wc -l <<<"$ACTUAL") in-repo edges)"
fi
exit $status
