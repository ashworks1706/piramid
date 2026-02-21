"use client";
/* eslint-disable @next/next/no-img-element */

import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

type Props = {
  src?: string;
  alt?: string;
};

// Zoom levels cycled through on each click inside the lightbox
const ZOOM_LEVELS = [1, 1.8, 3];

export function BlogImage({ src, alt }: Props) {
  const [open, setOpen] = useState(false);
  const [zoomIdx, setZoomIdx] = useState(0);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const dragging = useRef(false);
  const dragStart = useRef({ x: 0, y: 0 });
  const offsetAtDrag = useRef({ x: 0, y: 0 });
  const imgRef = useRef<HTMLImageElement>(null);

  const zoom = ZOOM_LEVELS[zoomIdx];
  const isZoomed = zoomIdx > 0;

  const close = useCallback(() => {
    setOpen(false);
    setZoomIdx(0);
    setOffset({ x: 0, y: 0 });
  }, []);

  const cycleZoom = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setZoomIdx((i) => {
      const next = (i + 1) % ZOOM_LEVELS.length;
      if (next === 0) setOffset({ x: 0, y: 0 });
      return next;
    });
  }, []);

  // Wheel to zoom
  const onWheel = useCallback((e: React.WheelEvent) => {
    e.stopPropagation();
    setZoomIdx((i) => {
      if (e.deltaY < 0) return Math.min(i + 1, ZOOM_LEVELS.length - 1);
      const next = Math.max(i - 1, 0);
      if (next === 0) setOffset({ x: 0, y: 0 });
      return next;
    });
  }, []);

  // Drag to pan while zoomed
  const onMouseDown = useCallback((e: React.MouseEvent) => {
    if (!isZoomed) return;
    dragging.current = true;
    setIsDragging(true);
    dragStart.current = { x: e.clientX, y: e.clientY };
    offsetAtDrag.current = offset;
    e.preventDefault();
  }, [isZoomed, offset]);

  const onMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragging.current) return;
    setOffset({
      x: offsetAtDrag.current.x + (e.clientX - dragStart.current.x),
      y: offsetAtDrag.current.y + (e.clientY - dragStart.current.y),
    });
  }, []);

  const onMouseUp = useCallback(() => { dragging.current = false; setIsDragging(false); }, []);

  // Touch drag
  const touchStart = useRef({ x: 0, y: 0 });
  const onTouchStart = useCallback((e: React.TouchEvent) => {
    if (!isZoomed) return;
    touchStart.current = { x: e.touches[0].clientX, y: e.touches[0].clientY };
    offsetAtDrag.current = offset;
  }, [isZoomed, offset]);

  const onTouchMove = useCallback((e: React.TouchEvent) => {
    setOffset({
      x: offsetAtDrag.current.x + (e.touches[0].clientX - touchStart.current.x),
      y: offsetAtDrag.current.y + (e.touches[0].clientY - touchStart.current.y),
    });
  }, []);

  // ESC to close
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => { if (e.key === "Escape") close(); };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, close]);

  // Prevent body scroll when lightbox open
  useEffect(() => {
    if (open) { document.body.style.overflow = "hidden"; }
    else { document.body.style.overflow = ""; }
    return () => { document.body.style.overflow = ""; };
  }, [open]);

  const lightbox = open
    ? createPortal(
        <div
          role="dialog"
          aria-modal="true"
          aria-label={alt ?? "Image preview"}
          className="fixed inset-0 z-9999 flex flex-col items-center justify-center"
          style={{ background: "rgba(5, 7, 13, 0.92)", backdropFilter: "blur(12px)" }}
          onClick={close}
          onMouseMove={onMouseMove}
          onMouseUp={onMouseUp}
          onMouseLeave={onMouseUp}
        >
          {/* Close button */}
          <button
            onClick={close}
            className="absolute top-4 right-4 z-10 flex h-9 w-9 items-center justify-center rounded-full border border-white/10 bg-white/10 text-white backdrop-blur transition hover:bg-white/20"
            aria-label="Close"
          >
            ✕
          </button>

          {/* Zoom hint */}
          <div className="absolute top-4 left-1/2 -translate-x-1/2 flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs text-slate-400 backdrop-blur select-none pointer-events-none">
            <span>scroll or click to zoom</span>
            {isZoomed && <span className="text-indigo-300">{Math.round(zoom * 100)}%</span>}
          </div>

          {/* Image wrapper — stop propagation so clicking the image doesn't close */}
          <div
            className="relative flex items-center justify-center"
            style={{
              width: "100%",
              height: "100%",
              overflow: isZoomed ? "visible" : "hidden",
            }}
            onClick={(e) => e.stopPropagation()}
            onWheel={onWheel}
          >
            <img
              ref={imgRef}
              src={src}
              alt={alt ?? ""}
              draggable={false}
              onClick={cycleZoom}
              onMouseDown={onMouseDown}
              onTouchStart={onTouchStart}
              onTouchMove={onTouchMove}
              style={{
                transform: `scale(${zoom}) translate(${offset.x / zoom}px, ${offset.y / zoom}px)`,
                transition: isDragging ? "none" : "transform 0.25s cubic-bezier(0.4,0,0.2,1)",
                maxWidth: "90vw",
                maxHeight: "82vh",
                width: "auto",
                height: "auto",
                borderRadius: "0.75rem",
                border: "1px solid rgba(255,255,255,0.1)",
                boxShadow: "0 24px 80px rgba(0,0,0,0.6)",
                cursor: isZoomed ? (isDragging ? "grabbing" : "grab") : "zoom-in",
                userSelect: "none",
              }}
            />
          </div>

          {/* Caption */}
          {alt && (
            <div
              className="absolute bottom-5 left-1/2 -translate-x-1/2 max-w-lg text-center text-sm text-slate-300 px-4 py-2 rounded-xl border border-white/10 bg-black/50 backdrop-blur pointer-events-none"
              onClick={(e) => e.stopPropagation()}
            >
              {alt}
            </div>
          )}
        </div>,
        document.body
      )
    : null;

  return (
    <>
      <span className="group block my-6 text-center">
        <img
          src={src}
          alt={alt ?? ""}
          loading="lazy"
          onClick={() => { setOpen(true); setZoomIdx(0); setOffset({ x: 0, y: 0 }); }}
          className="inline-block max-w-full h-auto rounded-xl border border-white/10 shadow-lg shadow-slate-900/40 cursor-zoom-in transition-all duration-200 group-hover:brightness-110 group-hover:border-indigo-400/40 group-hover:shadow-indigo-900/30"
        />
        {alt && (
          <span className="mt-2 block text-xs text-slate-500 italic">{alt}</span>
        )}
      </span>
      {lightbox}
    </>
  );
}
