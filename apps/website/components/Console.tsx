"use client";

import { useEffect, useRef, useState } from "react";
import type { Entry } from "../lib/console";

/** Where a document named by `open` lives. */
const DOCS: Record<string, string> = {
  architecture: "docs/ARCHITECTURE.md",
  roadmap: "docs/ROADMAP.md",
  setup: "docs/SETUP.md",
  decisions: "docs/decisions/",
  readme: "README.md",
};

const REPO = "https://github.com/ashworks1706/piramid";

/** One block in the transcript: what was typed, and what came back. */
type Block = { input: string; output: string[] };

const BANNER = [
  "piramid 0.2.0 - ask it about itself",
  "type help, or a command. tab completes, up and down recall.",
];

/**
 * A console that answers out of the repository.
 *
 * It is not a shell and it is not the terminal UI the binary opens: it reads a fixed set of
 * answers assembled at build time, so nothing it says can drift from what the repository
 * actually contains.
 */
export function Console({ entries }: { entries: Entry[] }) {
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [input, setInput] = useState("");
  const [recall, setRecall] = useState(-1);
  const field = useRef<HTMLInputElement>(null);
  const scroller = useRef<HTMLDivElement>(null);

  const typed = blocks.map((block) => block.input).filter(Boolean);
  const names = [
    ...entries.map((entry) => entry.name),
    "help",
    "clear",
    "open",
  ];

  useEffect(() => {
    const box = scroller.current;
    if (box) {
      box.scrollTop = box.scrollHeight;
    }
  }, [blocks]);

  function answer(line: string): string[] {
    const [command, argument] = line.trim().split(/\s+/, 2);
    if (!command) {
      return [];
    }
    if (command === "help") {
      return [
        "commands",
        ...entries.map((entry) => `  ${entry.name.padEnd(14)}${entry.blurb}`),
        `  ${"open <doc>".padEnd(14)}open a document on GitHub`,
        `  ${"clear".padEnd(14)}empty the transcript`,
      ];
    }
    if (command === "clear") {
      return [];
    }
    if (command === "open") {
      const path = DOCS[argument ?? ""];
      if (!path) {
        return [
          `no document called ${argument ?? ""}. try: ${Object.keys(DOCS).join(", ")}`,
        ];
      }
      window.open(`${REPO}/blob/main/${path}`, "_blank", "noopener,noreferrer");
      return [`opening ${path}`];
    }
    const entry = entries.find((candidate) => candidate.name === command);
    if (entry) {
      return entry.lines;
    }
    return [`${command}: not a command. type help.`];
  }

  function submit(line: string) {
    if (line.trim() === "clear") {
      setBlocks([]);
    } else {
      setBlocks((current) => [
        ...current,
        { input: line, output: answer(line) },
      ]);
    }
    setInput("");
    setRecall(-1);
  }

  function onKey(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Enter") {
      submit(input);
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      const stem = input.trim();
      const match = names.find((name) => name.startsWith(stem) && stem);
      if (match) {
        setInput(match);
      }
      return;
    }
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      if (!typed.length) {
        return;
      }
      event.preventDefault();
      const next =
        event.key === "ArrowUp"
          ? Math.min(recall + 1, typed.length - 1)
          : Math.max(recall - 1, -1);
      setRecall(next);
      setInput(next === -1 ? "" : typed[typed.length - 1 - next]);
    }
  }

  return (
    <section className="console" aria-label="Ask Piramid about itself">
      <header className="readme-bar">
        <span className="readme-dot" />
        <span className="readme-dot" />
        <span className="readme-dot" />
        <span className="readme-name">piramid</span>
      </header>
      {/* The whole surface focuses the field, the way clicking a terminal does. */}
      <div
        ref={scroller}
        className="console-body"
        onClick={() => field.current?.focus()}
        role="presentation"
      >
        {BANNER.map((line) => (
          <p key={line} className="console-banner">
            {line}
          </p>
        ))}
        {blocks.map((block, index) => (
          <div key={`${block.input}-${index}`}>
            <p className="console-line">
              <span className="console-prompt">&gt;</span> {block.input}
            </p>
            {block.output.map((line, at) => (
              <p key={`${line}-${at}`} className="console-line console-out">
                {line || " "}
              </p>
            ))}
          </div>
        ))}
        <p className="console-line">
          <label className="console-prompt" htmlFor="console-input">
            &gt;
          </label>
          <input
            id="console-input"
            ref={field}
            className="console-input"
            value={input}
            onChange={(event) => setInput(event.target.value)}
            onKeyDown={onKey}
            autoComplete="off"
            spellCheck={false}
            aria-label="command"
          />
        </p>
      </div>
    </section>
  );
}
