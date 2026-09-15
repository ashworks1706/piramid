"use client";

import { useEffect, useRef, useState } from "react";
import { CLI_FRAMES } from "../lib/cli-frames";

/** Milliseconds per frame. */
const FRAME_MS = 55;
const LAST = CLI_FRAMES.length - 1;

/** Largest the logo is ever drawn, in px. */
const MAX_FONT_PX = 16;

/** Smallest the logo is drawn before it stops shrinking, in px. */
const MIN_FONT_PX = 4;

/** Share of the viewport height the logo may take, leaving room for the tagline and the links. */
const HEIGHT_SHARE = 0.46;

/** Size the probe is fixed at in the stylesheet. Any size works; it yields the ratio used. */
const PROBE_PX = 100;

/** Columns and rows of the widest and tallest frame, which is not always the one it settles on. */
const COLUMNS = Math.max(
  ...CLI_FRAMES.map((frame) =>
    Math.max(...frame.split("\n").map((line) => line.length)),
  ),
);
const ROWS = Math.max(...CLI_FRAMES.map((frame) => frame.split("\n").length));

/** A block that bounds every frame, so no frame of the animation can overflow the fit. */
export const PROBE_TEXT = Array.from({ length: ROWS }, () =>
  "█".repeat(COLUMNS),
).join("\n");

/**
 * The font size at which the animation fits the space, from its measured size rather than an
 * assumed character advance. A monospace fallback with a wider advance than the webfont clips an
 * animation sized by arithmetic; measuring the glyphs that actually rendered cannot.
 */
export function fittedSize(
  probe: HTMLElement,
  availableWidth: number,
  availableHeight: number,
) {
  const widthPerPx = probe.scrollWidth / PROBE_PX;
  const heightPerPx = probe.scrollHeight / PROBE_PX;
  if (widthPerPx <= 0 || heightPerPx <= 0) {
    return MAX_FONT_PX;
  }
  const fits = Math.min(
    availableWidth / widthPerPx,
    availableHeight / heightPerPx,
  );
  return Math.max(MIN_FONT_PX, Math.min(MAX_FONT_PX, fits));
}

/**
 * Plays the ASCII animation the piramid binary prints on startup, as preformatted text, and holds
 * on the final frame. Each frame is chosen from elapsed wall time.
 */
export function CliAnimation() {
  const [frame, setFrame] = useState(0);
  const settled = frame >= LAST;
  const stage = useRef<HTMLDivElement>(null);
  const probe = useRef<HTMLPreElement>(null);
  const shown = useRef<HTMLPreElement>(null);

  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      // With reduced motion the final frame is shown after one tick.
      const id = window.setTimeout(() => setFrame(LAST), 0);
      return () => window.clearTimeout(id);
    }

    const started = Date.now();
    let timer = 0;

    const step = () => {
      const next = Math.min(
        Math.floor((Date.now() - started) / FRAME_MS),
        LAST,
      );
      setFrame(next);
      if (next < LAST) {
        const due = started + (next + 1) * FRAME_MS - Date.now();
        timer = window.setTimeout(step, Math.max(due, 0));
      }
    };

    timer = window.setTimeout(step, FRAME_MS);
    return () => window.clearTimeout(timer);
  }, []);

  useEffect(() => {
    const fit = () => {
      const box = stage.current;
      const measured = probe.current;
      const pre = shown.current;
      if (!box || !measured || !pre) {
        return;
      }
      pre.style.fontSize = `${fittedSize(measured, box.clientWidth, window.innerHeight * HEIGHT_SHARE)}px`;
    };

    // A webfont swapping in changes the glyph advance and so the size that fits, and the swap
    // lands after the promise resolves, so measure on the frame after it.
    let frameId = 0;
    const refit = () => {
      cancelAnimationFrame(frameId);
      frameId = requestAnimationFrame(() => {
        frameId = requestAnimationFrame(fit);
      });
    };

    fit();
    refit();
    const observer = new ResizeObserver(fit);
    if (stage.current) {
      observer.observe(stage.current);
    }
    window.addEventListener("resize", fit);
    window.addEventListener("orientationchange", fit);
    const fonts = document.fonts;
    fonts?.ready.then(refit).catch(() => undefined);
    fonts?.addEventListener?.("loadingdone", refit);
    return () => {
      cancelAnimationFrame(frameId);
      observer.disconnect();
      window.removeEventListener("resize", fit);
      window.removeEventListener("orientationchange", fit);
      fonts?.removeEventListener?.("loadingdone", refit);
    };
  }, []);

  return (
    <div ref={stage} className="cli-stage">
      <pre ref={probe} className="cli-animation cli-probe" aria-hidden="true">
        {PROBE_TEXT}
      </pre>
      <pre
        ref={shown}
        aria-label="Piramid"
        className={`cli-animation cli-shown select-none${settled ? " is-settled" : ""}`}
      >
        {CLI_FRAMES[frame]}
      </pre>
    </div>
  );
}
