import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./app/**/*.{ts,tsx,mdx}",
    "./components/**/*.{ts,tsx}",
    "./mdx-components.tsx",
    "../docs/**/*.{md,mdx}",
  ],
};

export default config;
