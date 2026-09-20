/** Rows of the pyramid, widest last. */
const ROWS = 9;

/** How many blocks the base is wide. Each row above loses two. */
const BASE = ROWS * 2 - 1;

type Block = { row: number; column: number; seed: number };

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

/** The blocks of the pyramid, centred row by row. */
const BLOCKS: Block[] = Array.from({ length: ROWS }, (_, row) => {
  const width = row * 2 + 1;
  const start = (BASE - width) / 2;
  return Array.from({ length: width }, (_, at) => ({
    row,
    column: start + at,
    seed: noise(row, start + at),
  }));
}).flat();

/**
 * The mark, as blocks that settle in and then keep moving.
 *
 * Decoration, so it is aria-hidden and holds still under reduced motion. Which block lights when
 * comes from a hash of its position rather than a random number, so the markup is the same on the
 * server and in the browser.
 */
export function PixelPyramid() {
  return (
    <div className="pyramid" aria-hidden="true">
      <div className="pyramid-grid">
        {BLOCKS.map(({ row, column, seed }) => (
          <span
            key={`${row}-${column}`}
            className="pyramid-block"
            style={
              {
                gridRow: row + 1,
                gridColumn: column + 1,
                "--seed": seed,
                "--row": row,
              } as React.CSSProperties
            }
          />
        ))}
      </div>
    </div>
  );
}
