import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: 'export',
  // Proxy API requests to the Rust backend during development (npm run dev).
  // In production, the static export is served by the Rust backend directly,
  // so these rewrites are not needed (and are ignored by `next build`).
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: 'http://localhost:8080/api/:path*',
      },
    ];
  },
};

export default nextConfig;
