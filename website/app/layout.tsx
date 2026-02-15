import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Piramid – Vector Database for Agentic Applications",
  description: "Piramid is a Rust vector database built for agentic workloads: mmap + WAL, HNSW/IVF/Flat indexes, filter-aware search, embeddings (OpenAI/local), and a roadmap to GPU co-location with Zipy.",
  keywords: ["vector database", "rust", "low latency", "HNSW", "IVF", "flat index", "embeddings", "RAG", "agentic", "similarity search"],
  authors: [{ name: "ashworks1706" }],
  creator: "ashworks1706",
  publisher: "ashworks1706",
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      'max-video-preview': -1,
      'max-image-preview': 'large',
      'max-snippet': -1,
    },
  },
  openGraph: {
    type: "website",
    locale: "en_US",
    url: "https://piramiddb.com",
    title: "Piramid - Hybrid Vector Database",
    description: "Open-source hybrid vector database combining graph-based and traditional vector search capabilities.",
    siteName: "Piramid",
    images: [
      {
        url: "../public/logo_dark.png",
        width: 1200,
        height: 630,
        alt: "Piramid Vector Database",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Piramid - Hybrid Vector Database",
    description: "Open-source hybrid vector database combining graph-based and traditional vector search capabilities.",
    images: ["../public/logo_dark.png"],
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
    <html lang="en" className="dark">
      <body className="antialiased">
        {children}
      </body>
    </html>
  );
}
