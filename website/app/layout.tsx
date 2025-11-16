import type { Metadata } from "next";
import { Open_Sans } from "next/font/google";
import "./globals.css";

const openSans = Open_Sans({
  variable: "--font-open-sans",
  subsets: ["latin"],
  weight: ["300", "400", "600", "700"],
});

export const metadata: Metadata = {
  title: "Piramid - Hybrid Vector Database | Graph-Based & Traditional Vector Search",
  description: "Piramid is an open-source hybrid vector database written in Rust, combining graph-based and traditional vector search capabilities for customizable, high-performance AI applications.",
  keywords: ["vector database", "graph database", "hybrid database", "rust", "open source", "vector search", "AI", "machine learning", "embeddings", "similarity search"],
  authors: [{ name: "Piramid Team" }],
  creator: "Piramid",
  publisher: "Piramid",
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
    url: "https://piramid.dev",
    title: "Piramid - Hybrid Vector Database",
    description: "Open-source hybrid vector database combining graph-based and traditional vector search capabilities.",
    siteName: "Piramid",
    images: [
      {
        url: "/logo.png",
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
    images: ["/logo.png"],
    creator: "@piramiddb",
  },
  metadataBase: new URL("https://piramid.dev"),
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
      <body
        className={`${openSans.variable} antialiased`}
      >
        {children}
      </body>
    </html>
  );
}
