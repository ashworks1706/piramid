import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: 'export',  // Static export for embedding in Rust server
  trailingSlash: true,
};

export default nextConfig;
