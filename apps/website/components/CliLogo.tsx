import { CLI_FRAMES } from "../lib/cli-frames";

/** The frame the CLI settles on. */
const LOGO = CLI_FRAMES[CLI_FRAMES.length - 1];

/**
 * A character cell, in the proportion a terminal draws one.
 *
 * JetBrains Mono advances 0.6em against a line box of 1em, so the cell is 12 by 20 rather than
 * square. Getting this wrong does not corrupt the wordmark, it squashes it.
 */
const W = 12;
const H = 20;

/** Stroke thickness, and where the two strokes of a double line sit inside the cell. */
const T = 2;
const VX = [3, 7];
const HY = [7, 11];

type Rect = [number, number, number, number];

/** A full cell. */
const SOLID = "█";

/** A cell of dark shade. The mark beside the wordmark is drawn in these, and only it is. */
const SHADE = "▓";

/**
 * The double box-drawing glyphs, as the strokes they are made of.
 *
 * A corner is two nested right angles: the outer stroke turns at the outer corner and the inner
 * stroke at the inner one, which is what gives the wordmark its outline.
 */
const BOX: Record<string, Rect[]> = {
  "═": [
    [0, HY[0], W, T],
    [0, HY[1], W, T],
  ],
  "║": [
    [VX[0], 0, T, H],
    [VX[1], 0, T, H],
  ],
  "╔": [
    [VX[0], HY[0], W - VX[0], T],
    [VX[0], HY[0], T, H - HY[0]],
    [VX[1], HY[1], W - VX[1], T],
    [VX[1], HY[1], T, H - HY[1]],
  ],
  "╗": [
    [0, HY[0], VX[1] + T, T],
    [VX[1], HY[0], T, H - HY[0]],
    [0, HY[1], VX[0] + T, T],
    [VX[0], HY[1], T, H - HY[1]],
  ],
  "╚": [
    [VX[0], HY[1], W - VX[0], T],
    [VX[0], 0, T, HY[1] + T],
    [VX[1], HY[0], W - VX[1], T],
    [VX[1], 0, T, HY[0] + T],
  ],
  "╝": [
    [0, HY[1], VX[1] + T, T],
    [VX[1], 0, T, HY[1] + T],
    [0, HY[0], VX[0] + T, T],
    [VX[0], 0, T, HY[0] + T],
  ],
};

/**
 * The wordmark on its own.
 *
 * The frame carries the mark beside the letters, drawn in shade and nothing else, so dropping
 * every shade cell leaves the letters. What is then blank on every side is trimmed, which is what
 * lets the wordmark sit flush against the edge it is aligned to.
 */
const LINES = (() => {
  const rows = LOGO.split("\n").map((line) =>
    [...line].map((glyph) => (glyph === SHADE ? " " : glyph)).join(""),
  );
  while (rows.length && !rows[0].trim()) rows.shift();
  while (rows.length && !rows[rows.length - 1].trim()) rows.pop();
  const first = Math.min(
    ...rows.filter((row) => row.trim()).map((row) => row.search(/\S/)),
  );
  return rows.map((row) => row.slice(first).trimEnd());
})();

const COLUMNS = Math.max(...LINES.map((line) => line.length));

const lit: Rect[] = [];

LINES.forEach((line, y) => {
  let x = 0;
  while (x < line.length) {
    const glyph = line[x];
    if (glyph === SOLID) {
      const start = x;
      while (x < line.length && line[x] === glyph) {
        x += 1;
      }
      // One rectangle per unbroken run, rather than one per cell.
      lit.push([start * W, y * H, (x - start) * W, H]);
      continue;
    }
    for (const [rx, ry, rw, rh] of BOX[glyph] ?? []) {
      lit.push([x * W + rx, y * H + ry, rw, rh]);
    }
    x += 1;
  }
});

function draw(rects: Rect[]) {
  return rects.map(([x, y, width, height]) => (
    <rect
      key={`${x}-${y}-${width}`}
      x={x}
      y={y}
      width={width}
      height={height}
    />
  ));
}

/**
 * The wordmark, as the shape the CLI draws rather than as the text it draws it with.
 *
 * The CLI has one font and a fixed cell, so blocks, box-drawing glyphs and spaces line up there.
 * A browser has neither: the blocks and the box come from one font and the spaces from another,
 * and the moment their advances disagree, or one of them arrives late, every row shifts by a
 * different amount and the letters come apart. Drawing the frame as rectangles removes the
 * question, and an SVG with a viewBox takes whatever size the stylesheet gives it.
 */
export function CliLogo() {
  return (
    <svg
      className="cli-logo"
      viewBox={`0 0 ${COLUMNS * W} ${LINES.length * H}`}
      role="img"
      aria-label="Piramid"
      shapeRendering="crispEdges"
    >
      <g className="cli-logo-lit">{draw(lit)}</g>
    </svg>
  );
}
