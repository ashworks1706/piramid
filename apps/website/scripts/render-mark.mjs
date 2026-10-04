// Renders the pyramid mark from the last CLI frame into the site's icon and logo images.
// Run with: node scripts/render-mark.mjs. Needs sharp, which next installs.

import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SITE = path.join(HERE, "..");

/** The shade glyph the CLI draws the mark in. */
const SHADE = "▓";

/** A character cell, the same 12 by 20 the page draws. */
const W = 12;
const H = 20;

/** The block lattice that stands in for the shade glyph: two columns and three rows per cell. */
const FINE = { x: W / 2, y: H / 3, gap: 1 };

/** The same lattice at one block per half cell, for icons drawn too small to show the fine one. */
const COARSE = { x: W, y: H / 2, gap: 2 };

const ACCENT = "#22d3ee";
const ACCENT_DIM = "#0e7490";
const BG = "#05070c";

const source = readFileSync(path.join(SITE, "lib", "cli-frames.ts"), "utf8");
const frames = JSON.parse(
  source.slice(source.indexOf("= [") + 2, source.lastIndexOf("]") + 1),
);
const rows = frames[frames.length - 1]
  .split("\n")
  .map((row) =>
    [...row].map((glyph) => (glyph === SHADE ? "#" : " ")).join(""),
  );
while (rows.length && !rows[0].trim()) rows.shift();
while (rows.length && !rows[rows.length - 1].trim()) rows.pop();
const left = Math.min(
  ...rows.filter((row) => row.trim()).map((row) => row.search(/#/)),
);
const mark = rows.map((row) => row.slice(left).trimEnd());

const width = Math.max(...mark.map((row) => row.length)) * W;
const height = mark.length * H;

/** The mark as a square SVG, each run of shade cells filled with the dot lattice. */
function svg(fill, background, pad, lattice) {
  const side = Math.max(width, height) + pad * 2;
  const dx = (side - width) / 2;
  const dy = (side - height) / 2;
  const runs = [];
  mark.forEach((row, y) => {
    let x = 0;
    while (x < row.length) {
      if (row[x] !== "#") {
        x += 1;
        continue;
      }
      const start = x;
      while (x < row.length && row[x] === "#") x += 1;
      runs.push(
        `<rect x="${dx + start * W}" y="${dy + y * H}" width="${(x - start) * W}" height="${H}"/>`,
      );
    }
  });
  const ground = background
    ? `<rect width="${side}" height="${side}" fill="${background}"/>`
    : "";
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${side} ${side}" shape-rendering="crispEdges">
<defs><pattern id="shade" width="${lattice.x}" height="${lattice.y}" patternUnits="userSpaceOnUse" x="${dx}" y="${dy}"><rect width="${lattice.x - lattice.gap}" height="${lattice.y - lattice.gap}" fill="${fill}"/></pattern></defs>
${ground}<g fill="url(#shade)">${runs.join("")}</g>
</svg>
`;
}

const write = (name, text) => writeFileSync(path.join(SITE, name), text);
const png = (text, size, name) =>
  sharp(Buffer.from(text), { density: 300 })
    .resize(size, size)
    .png()
    .toFile(path.join(SITE, name));

write("app/icon.svg", svg(ACCENT, BG, 24, COARSE));
await png(svg(ACCENT, BG, 24, COARSE), 180, "app/apple-icon.png");
await png(svg(ACCENT, null, 24, FINE), 732, "public/logo_dark.png");
await png(svg(ACCENT_DIM, null, 24, FINE), 732, "public/logo_light.png");
