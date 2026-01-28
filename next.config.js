/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  images: {
    unoptimized: true,
  },
  // Tauri expects a static export
  distDir: 'out',
};

module.exports = nextConfig;
