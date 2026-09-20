import { PIXEL_LOGO } from "../lib/pixel-logo";

type Block = { row: number; column: number; seed: number };

/** How wide the grid is, in cells. */
const COLUMNS = Math.max(...PIXEL_LOGO.map((line) => line.length));

/** How tall it is. */
const ROWS = PIXEL_LOGO.length;

/**
 * A value in [0, 1) for one block. Integer operations only, so the server and the browser agree
 * and the markup they each produce is the same.
 */
function noise(row: number, column: number) {
  let hash = (row * 73856093) ^ (column * 19349663);
  hash = Math.imul(hash ^ (hash >>> 15), 2246822519);
  hash = Math.imul(hash ^ (hash >>> 13), 3266489917);
  return ((hash ^ (hash >>> 16)) >>> 0) / 4294967296;
}

/** One block per filled cell of the logo. */
const BLOCKS: Block[] = PIXEL_LOGO.flatMap((line, row) =>
  [...line].flatMap((cell, column) =>
    cell === " " ? [] : [{ row, column, seed: noise(row, column) }],
  ),
);

/**
 * The logo, as the blocks it pixelates to.
 *
 * The grid comes from public/logo_dark.png through scripts/pixelate-logo.mjs, so the mark here is
 * the mark everywhere else. Blocks build from the base up and then keep their own beat, which
 * leaves the shape formed and reading as the logo the whole time.
 *
 * Decoration, so it is aria-hidden and holds still under reduced motion. Which block dims when
 * comes from a hash of its position rather than a random number, so the markup is the same on the
 * server and in the browser.
 */
export function PixelPyramid() {
  return (
    <div className="pyramid" aria-hidden="true">
      <div
        className="pyramid-grid"
        style={{ "--columns": COLUMNS } as React.CSSProperties}
      >
        {BLOCKS.map(({ row, column, seed }) => (
          <span
            key={`${row}-${column}`}
            className="pyramid-block"
            style={
              {
                gridRow: row + 1,
                gridColumn: column + 1,
                "--seed": seed,
                "--rise": ROWS - row,
              } as React.CSSProperties
            }
          />
        ))}
      </div>
    </div>
  );
}
