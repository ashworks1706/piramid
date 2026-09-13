#!/usr/bin/env python3
"""Copy the CLI's ASCII animation frames into the website.

Regenerates apps/website/lib/cli-frames.ts from apps/cli/assets/ascii_motion_frames.rs_inc.
Rows and columns blank in every frame are trimmed, so all frames keep the same dimensions.
"""
import json
import pathlib
import re

root = pathlib.Path(__file__).resolve().parent.parent
src = root / "apps/cli/assets/ascii_motion_frames.rs_inc"
dst = root / "apps/website/lib/cli-frames.ts"

raw = [f.split("\n") for f in re.findall(r'r#"(.*?)"#', src.read_text(), re.S)]

height = max(len(f) for f in raw)
frames = [f + [""] * (height - len(f)) for f in raw]

# Rows and columns that are blank in every frame can go.
def row_used(i):
    return any(i < len(f) and f[i].strip() for f in frames)

def col_used(c):
    return any(c < len(line) and line[c] != " " for f in frames for line in f)

top = next((i for i in range(height) if row_used(i)), 0)
bottom = next((i for i in range(height - 1, -1, -1) if row_used(i)), height - 1)

width = max(len(line) for f in frames for line in f)
left = next((c for c in range(width) if col_used(c)), 0)
right = next((c for c in range(width - 1, -1, -1) if col_used(c)), width - 1)

out = []
for f in frames:
    rows = []
    for line in f[top : bottom + 1]:
        padded = line.ljust(right + 1)
        rows.append(padded[left : right + 1].rstrip())
    out.append("\n".join(rows))

dst.write_text(
    "// Generated from apps/cli/assets/ascii_motion_frames.rs_inc, the same frames the CLI plays.\n"
    "// Regenerate with scripts/sync-ascii-frames.py after changing that file.\n\n"
    "export const CLI_FRAMES: string[] = "
    + json.dumps(out, ensure_ascii=False, indent=2)
    + ";\n"
)
print(f"wrote {len(out)} frames, {bottom - top + 1} rows x {right - left + 1} cols")
