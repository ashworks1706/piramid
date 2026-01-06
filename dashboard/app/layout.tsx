import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Piramid Dashboard",
  description: "Admin console for Piramid vector database",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="antialiased">
        {children}
      </body>
    </html>
  );
}
