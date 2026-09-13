"use client";

import { useEffect, useState } from "react";
import { CLI_FRAMES } from "../lib/cli-frames";

/** Milliseconds per frame. */
const FRAME_MS = 55;
const LAST = CLI_FRAMES.length - 1;

/**
 * Plays the ASCII animation the piramid binary prints on startup, as preformatted text, and holds
 * on the final frame. Each frame is chosen from elapsed wall time.
 */
export function CliAnimation() {
  const [frame, setFrame] = useState(0);
  const settled = frame >= LAST;

  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      // With reduced motion the final frame is shown after one tick.
      const id = window.setTimeout(() => setFrame(LAST), 0);
      return () => window.clearTimeout(id);
    }

    const started = Date.now();
    let timer = 0;

    const step = () => {
      const next = Math.min(Math.floor((Date.now() - started) / FRAME_MS), LAST);
      setFrame(next);
      if (next < LAST) {
        const due = started + (next + 1) * FRAME_MS - Date.now();
        timer = window.setTimeout(step, Math.max(due, 0));
      }
    };

    timer = window.setTimeout(step, FRAME_MS);
    return () => window.clearTimeout(timer);
  }, []);

  return (
    <pre
      aria-label="Piramid"
      className={`cli-animation select-none${settled ? " is-settled" : ""}`}
    >
      {CLI_FRAMES[frame]}
    </pre>
  );
}
