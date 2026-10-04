"use client";

import { useState } from "react";

/** One way to install, with the command it copies. */
type Method = { id: string; label: string; command: string };

const METHODS: Method[] = [
  {
    id: "curl",
    label: "curl",
    command: "curl -fsSL https://piramiddb.com/install.sh | sh",
  },
  {
    id: "cargo",
    label: "cargo",
    command: "cargo install piramid",
  },
  { id: "python", label: "python client", command: "pip install piramid" },
];

/** The install command, one tab per method, with a copy button. */
export function InstallTabs() {
  const [active, setActive] = useState(METHODS[0]);
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    await navigator.clipboard.writeText(active.command);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="install">
      <div className="install-tabs" role="tablist" aria-label="Install method">
        {METHODS.map((method) => (
          <button
            key={method.id}
            type="button"
            role="tab"
            aria-selected={method.id === active.id}
            className="install-tab"
            onClick={() => {
              setActive(method);
              setCopied(false);
            }}
          >
            {method.label}
          </button>
        ))}
      </div>
      <div className="install-line">
        <code>{active.command}</code>
        <button type="button" className="install-copy" onClick={copy}>
          {copied ? "copied" : "copy"}
        </button>
      </div>
    </div>
  );
}
