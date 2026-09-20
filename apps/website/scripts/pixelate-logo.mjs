#!/usr/bin/env node
// Pixelate the logo into the grid the site animates.
//
// Regenerates lib/pixel-logo.ts from public/logo_dark.png. Needs sharp, which next installs.
// Run it after changing that file:
//   cd apps/website && node scripts/pixelate-logo.mjs
//
// The grid is committed, so nothing in the build reads the PNG or needs sharp.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SOURCE = path.join(HERE, "..", "public", "logo_dark.png");
const TARGET = path.join(HERE, "..", "lib", "pixel-logo.ts");

// Chosen so the two bands across the base survive as their own rows. Finer grids alias the
// sloped edges of the bands into the rows above and below them.
const COLUMNS = 26;
const ROWS = 28;

async function main() {
  const { data, info } = await sharp(SOURCE)
    .flatten({ background: "#000" })
    .greyscale()
    .resize(COLUMNS, ROWS, { fit: "fill", kernel: "cubic" })
    .raw()
    .toBuffer({ resolveWithObject: true });

  const rows = [];
  for (let row = 0; row < ROWS; row++) {
    let line = "";
    for (let column = 0; column < COLUMNS; column++) {
      line += data[(row * COLUMNS + column) * info.channels] > 127 ? "#" : " ";
    }
    rows.push(line);
  }

  // Rows and columns the mark never reaches are dropped, so the grid is the mark and nothing else.
  while (rows.length && !rows[0].trim()) rows.shift();
  while (rows.length && !rows[rows.length - 1].trim()) rows.pop();
  let first = COLUMNS;
  let last = 0;
  for (const line of rows) {
    const start = line.indexOf("#");
    if (start === -1) continue;
    first = Math.min(first, start);
    last = Math.max(last, line.lastIndexOf("#"));
  }
  const cropped = rows.map((line) =>
    line.slice(first, last + 1).padEnd(last + 1 - first),
  );

  const body = cropped.map((line) => `  ${JSON.stringify(line)},`).join("\n");
  fs.writeFileSync(
    TARGET,
    [
      "// Generated from public/logo_dark.png by scripts/pixelate-logo.mjs. Do not edit by hand.",
      "",
      "/** The logo as a grid of cells, one character each: a block where the mark is, a space where it is not. */",
      "export const PIXEL_LOGO: string[] = [",
      body,
      "];",
      "",
    ].join("\n"),
  );
  process.stdout.write(cropped.join("\n") + "\n");
}

main();
