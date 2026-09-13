import type { Metadata } from "next";
import { JetBrains_Mono } from "next/font/google";
import localFont from "next/font/local";
import "./globals.css";
import "katex/dist/katex.min.css";

// Self-hosted by next/font.
const mono = JetBrains_Mono({
  subsets: ["latin"],
  display: "swap",
  variable: "--font-mono",
});

// JetBrains Mono box-drawing and block glyphs, U+2500-259F, which the latin subset lacks.
const blocks = localFont({
  src: "./fonts/jetbrains-mono-blocks.woff2",
  display: "block",
  variable: "--font-blocks",
  declarations: [{ prop: "unicode-range", value: "U+2500-259F" }],
});

export const metadata: Metadata = {
  title: {
    template: "%s | Piramid",
    default: "Piramid – Inference engine for RAG",
  },
  description:
    "Piramid is an inference engine for RAG on one GPU, written in Rust. Documents, model weights and the KV cache live in one process, so retrieval runs in the same process as generation. It stores and searches your documents with exact search and metadata filters, and serves generation over HTTP with piramid serve.",
  keywords: [
    "inference engine",
    "RAG",
    "retrieval-augmented generation",
    "rust",
    "GPU",
    "KV cache",
    "embeddings",
    "LLM serving",
  ],
  authors: [{ name: "ashworks1706" }],
  creator: "ashworks1706",
  publisher: "ashworks1706",
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-video-preview": -1,
      "max-image-preview": "large",
      "max-snippet": -1,
    },
  },
  openGraph: {
    type: "website",
    locale: "en_US",
    url: "https://piramiddb.com",
    title: "Piramid – Inference engine for RAG",
    description:
      "An inference engine for RAG on one GPU, in Rust, with retrieval in the same process as generation.",
    siteName: "Piramid",
    images: [
      {
        url: "/logo_dark.png",
        width: 711,
        height: 732,
        alt: "Piramid",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Piramid – Inference engine for RAG",
    description:
      "An inference engine for RAG on one GPU, in Rust, with retrieval in the same process as generation.",
    images: ["/logo_dark.png"],
    creator: "@piramiddb",
  },
  metadataBase: new URL("https://piramiddb.com"),
  alternates: {
    canonical: "/",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    // Hydration warnings are suppressed on the html element only.
    <html lang="en" className={`dark ${mono.variable} ${blocks.variable}`} suppressHydrationWarning>
      <body className="antialiased">{children}</body>
    </html>
  );
}
