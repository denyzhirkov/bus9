#!/bin/bash
set -e

echo "Building frontend..."
cd front
npm install
npm run build
cd ..

echo "Building and running backend..."
cargo run --release
