#!/bin/bash
set -e

echo "Building benchmark tool..."
cd tests/bench
cargo build --release
cd ../..

echo "------------------------------------------------"
echo "Running Pub/Sub Benchmark (20k reqs, 100 conc, 50 subs)"
echo "------------------------------------------------"
./tests/bench/target/release/bench --requests 20000 --concurrency 100 --subscribers 50 pub --topic bench-test

echo ""
echo "------------------------------------------------"
echo "Running Queue Benchmark (20k reqs, 100 conc, 50 consumers)"
echo "------------------------------------------------"
./tests/bench/target/release/bench --requests 20000 --concurrency 100 --consumers 50 queue --name bench-queue

echo ""
echo "Done!"
