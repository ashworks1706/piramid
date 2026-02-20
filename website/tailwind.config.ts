import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./app/**/*.{ts,tsx,mdx}",
    "./components/**/*.{ts,tsx}",
    "./mdx-components.tsx",
    "../blogs/**/*.{md,mdx}",
  ],
};

export default config;
