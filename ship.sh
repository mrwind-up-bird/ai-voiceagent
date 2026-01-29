#!/bin/bash
echo "🧹 cleaning..."
rm -rf out
rm -rf src-tauri/target/release/bundle

echo "🏗️  build frontend"
pnpm build

echo "🚀 build aurus application"
pnpm tauri build

echo "✅ Ready! Installer found in src-tauri/target/release/bundle/"