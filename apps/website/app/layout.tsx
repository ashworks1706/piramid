import type { Metadata } from "next";
import { JetBrains_Mono } from "next/font/google";
import { DESCRIPTION, TAGLINE } from "../lib/site";
import "./globals.css";

// Self-hosted by next/font.
const mono = JetBrains_Mono({
  subsets: ["latin"],
  display: "swap",
  variable: "--font-mono",
});

export const metadata: Metadata = {
  title: {
    template: "%s | Piramid",
    default: `Piramid – ${TAGLINE}`,
  },
  description: DESCRIPTION,
  keywords: [
    "inference runtime",
    "vector database",
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
    title: `Piramid – ${TAGLINE}`,
    description: DESCRIPTION,
    siteName: "Piramid",
    images: [
      {
        url: "/logo_dark.png",
        width: 732,
        height: 732,
        alt: "Piramid",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: `Piramid – ${TAGLINE}`,
    description: DESCRIPTION,
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
    <html
      lang="en"
      className={`dark ${mono.variable}`}
      suppressHydrationWarning
    >
      <body className="antialiased">{children}</body>
    </html>
  );
}

