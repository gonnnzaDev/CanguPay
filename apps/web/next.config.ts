import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // La imagen de docker copia solo .next/standalone, que lleva un server.js con
  // el minimo de dependencias y su propio runtime. Sin esto la imagen final
  // necesitaria node_modules entero.
  output: "standalone",
};

export default nextConfig;
