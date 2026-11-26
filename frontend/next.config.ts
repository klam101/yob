import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Self-contained server bundle for the Docker image.
  output: "standalone",
  cacheComponents: true,
  // The Documents page became Resume.
  async redirects() {
    return [
      { source: "/documents", destination: "/resume", permanent: true },
      // Resumes used to open at /resume/{id}; the id pattern keeps
      // /resume/edit itself from matching.
      {
        source: "/resume/:id([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})",
        destination: "/resume/edit/:id",
        permanent: true,
      },
    ];
  },
  partialPrefetching: true,
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
