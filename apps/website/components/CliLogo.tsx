"use client";

import { useEffect, useState } from "react";
import { CLI_FRAMES } from "../lib/cli-frames";

/** Time each frame is shown, the pace the CLI plays the same frames at. */
const FRAME_MS = 45;

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
 * Every frame without the mark, split into rows and cut to the rows and columns any frame uses.
 */
const GRIDS = (() => {
  const grids = CLI_FRAMES.map((frame) =>
    frame.split("\n").map((row) => row.replaceAll(SHADE, " ")),
  );
  const height = Math.max(...grids.map((rows) => rows.length));
  const used = (row: number) =>
    grids.some((rows) => (rows[row] ?? "").trim() !== "");
  let top = 0;
  while (top < height && !used(top)) top += 1;
  let bottom = height;
  while (bottom > top && !used(bottom - 1)) bottom -= 1;
  const cells = grids.map((rows) =>
    Array.from({ length: bottom - top }, (_, y) => [...(rows[top + y] ?? "")]),
  );
  const left = Math.min(
    ...cells.flat().map((row) => {
      const at = row.findIndex((glyph) => glyph !== " ");
      return at === -1 ? Infinity : at;
    }),
  );
  return cells.map((rows) => rows.map((row) => row.slice(left)));
})();

const ROWS = GRIDS[0]?.length ?? 0;

const COLUMNS = Math.max(...GRIDS.flat().map((row) => row.length));

/** The rectangles one frame draws. */
function rectangles(rows: string[][]) {
  const lit: Rect[] = [];
  rows.forEach((row, y) => {
    let x = 0;
    while (x < row.length) {
      const glyph = row[x];
      if (glyph === SOLID) {
        const start = x;
        while (x < row.length && row[x] === glyph) {
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
  return lit;
}

const FRAMES = GRIDS.map(rectangles);

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
 * The CLI wordmark animation, played once at the CLI's pace and held on its last frame.
 *
 * The CLI has one font and a fixed cell, so blocks, box-drawing glyphs and spaces line up there.
 * A browser has neither: the blocks and the box come from one font and the spaces from another,
 * and the moment their advances disagree, or one of them arrives late, every row shifts by a
 * different amount and the letters come apart. Drawing each frame as rectangles removes the
 * question, and an SVG with a viewBox takes whatever size the stylesheet gives it.
 */
export function CliLogo() {
  const [index, setIndex] = useState(0);
  const last = FRAMES.length - 1;

  useEffect(() => {
    if (index >= last) {
      return;
    }
    const reduce = window.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    const timer = window.setTimeout(
      () => setIndex((at) => (reduce ? last : at + 1)),
      reduce ? 0 : FRAME_MS,
    );
    return () => window.clearTimeout(timer);
  }, [index, last]);

  const frame = FRAMES[Math.min(index, last)];

  return (
    <svg
      className="cli-logo"
      viewBox={`0 0 ${COLUMNS * W} ${ROWS * H}`}
      role="img"
      aria-label="Piramid"
      shapeRendering="crispEdges"
    >
      <g className="cli-logo-lit">{draw(frame)}</g>
    </svg>
  );
}
