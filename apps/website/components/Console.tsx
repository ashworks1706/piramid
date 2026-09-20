"use client";

import Link from "next/link";
import { useEffect, useRef, useState } from "react";
import type { Entry, File } from "../lib/console";

const REPO = "https://github.com/ashworks1706/piramid";

/** What the banner suggests trying, in the order it suggests them. */
const EXAMPLES = ["help", "ls", "cat readme", "cat blogs/history"];

/** How long between two lines of the opening, in milliseconds. */
const BEAT = 95;

/** One block in the transcript: what was asked, and what came back. */
type Block = {
  input: string;
  output: string[];
  page?: { title: string; html: string };
};

/**
 * A console that answers out of the repository.
 *
 * It is not a shell and it is not the terminal UI the binary opens: it reads a fixed set of
 * answers assembled at build time, so nothing it says can drift from what the repository
 * actually contains.
 */
export function Console({
  entries,
  files,
  children,
}: {
  entries: Entry[];
  files: File[];
  children?: React.ReactNode;
}) {
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [input, setInput] = useState("");
  const [recall, setRecall] = useState(-1);
  /** How much of the opening has printed. It prints itself; nothing has to be typed for it. */
  const [shown, setShown] = useState(0);
  const field = useRef<HTMLInputElement>(null);
  const scroller = useRef<HTMLDivElement>(null);

  /** The opening: what the page says before anyone touches it. */
  const opening = [
    "inference runtime for retrieval systems",
    "",
    ...(entries.find((entry) => entry.name === "about")?.lines ?? []),
    "",
    "$ cargo install piramid",
  ];
  const done = shown >= opening.length;

  useEffect(() => {
    if (done) {
      return;
    }
    // Reduced motion gets the whole opening at once, but still from the callback: a setState in
    // the body of an effect cascades a render.
    const reduce = window.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    const timer = window.setTimeout(
      () => setShown((at) => (reduce ? opening.length : at + 1)),
      reduce ? 0 : BEAT,
    );
    return () => window.clearTimeout(timer);
  }, [shown, done, opening.length]);

  const typed = blocks.map((block) => block.input).filter(Boolean);
  const names = [
    ...entries.map((entry) => entry.name),
    ...files.map((file) => `cat ${file.name}`),
    "help",
    "clear",
    "ls",
    "cat",
    "open",
  ];

  useEffect(() => {
    const box = scroller.current;
    // The last block, not the last child: the input line is always last.
    const last = box?.querySelector(":scope > div:last-of-type");
    if (!box || !last) {
      return;
    }
    // A long file should start at its first line, not its last, so the newest command is put at
    // the top of the view rather than the bottom. Measured against the box rather than read off
    // offsetTop, which is relative to whichever ancestor happens to be positioned.
    const delta =
      last.getBoundingClientRect().top - box.getBoundingClientRect().top;
    box.scrollTop = Math.min(
      box.scrollTop + delta,
      box.scrollHeight - box.clientHeight,
    );
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
        `  ${"ls".padEnd(14)}list the files here`,
        `  ${"cat <file>".padEnd(14)}print one of them`,
        `  ${"open <file>".padEnd(14)}open it on GitHub instead`,
        `  ${"clear".padEnd(14)}empty the transcript`,
      ];
    }
    if (command === "clear") {
      return [];
    }
    if (command === "ls") {
      const width = Math.max(...files.map((file) => file.name.length)) + 2;
      const group = (of: File[]) =>
        of.map((file) => `  ${file.name.padEnd(width)}${file.path}`);
      const posts = files.filter((file) => file.name.startsWith("blogs/"));
      return [
        ...group(files.filter((file) => !file.name.startsWith("blogs/"))),
        ...(posts.length ? ["", "blog", ...group(posts)] : []),
      ];
    }
    if (command === "cat" || command === "open") {
      const file = files.find((candidate) => candidate.name === argument);
      if (!file) {
        return [
          argument
            ? `${argument}: no such file. try ls.`
            : `${command}: which file? try ls.`,
        ];
      }
      if (command === "open") {
        // A post has a page of its own; a repository file does not, so that one goes to GitHub.
        const href = file.name.startsWith("blogs/")
          ? file.path
          : `${REPO}/blob/main/${file.path}`;
        window.open(href, "_blank", "noopener,noreferrer");
        return [`opening ${href}`];
      }
      // cat is answered by submit, which attaches the rendered page to the block.
      return [];
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
      setInput("");
      setRecall(-1);
      return;
    }
    const [command, argument] = line.trim().split(/\s+/, 2);
    // cat shows the page itself rather than the text of the file it came from.
    const file =
      command === "cat"
        ? files.find((candidate) => candidate.name === argument)
        : undefined;
    setBlocks((current) => [
      ...current,
      file
        ? {
            input: line,
            output: [],
            page: { title: file.title, html: file.html },
          }
        : { input: line, output: answer(line) },
    ]);
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
        <span className="readme-name">/piramid</span>
        <span className="console-bar-links">
          <a href={REPO}>github</a>
          <Link href="/blogs">blog</Link>
        </span>
      </header>
      {/* The whole surface focuses the field, the way clicking a terminal does. */}
      <div
        ref={scroller}
        className="console-body"
        onClick={() => field.current?.focus()}
        role="presentation"
      >
        {children ? <div className="console-logo">{children}</div> : null}
        <div className="console-opening">
          {opening.slice(0, shown).map((line, at) => (
            <p
              key={`${line}-${at}`}
              className={`console-banner${line.startsWith("$ ") ? " console-install select-all" : ""}`}
            >
              {line || "\u00a0"}
            </p>
          ))}
        </div>
        <p
          className={`console-banner console-hint${done ? "" : " is-waiting"}`}
        >
          {"try "}
          {EXAMPLES.map((example, at) => (
            <span key={example}>
              {at ? ", " : ""}
              <button
                type="button"
                className="console-example"
                onClick={() => submit(example)}
              >
                {example}
              </button>
            </span>
          ))}
          {", or type below."}
        </p>
        {blocks.map((block, index) => (
          <div key={`${block.input}-${index}`}>
            <p className="console-line">
              <span className="console-prompt">&gt;</span> {block.input}
            </p>
            {block.output.length ? (
              <pre className="console-out">{block.output.join("\n")}</pre>
            ) : null}
            {block.page ? (
              <article
                className="console-page readme-prose"
                // Rendered at build time from a file in this repository, never from a request.
                dangerouslySetInnerHTML={{ __html: block.page.html }}
              />
            ) : null}
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
