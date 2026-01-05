#!/bin/bash
set -e

echo "Building benchmark tool..."
cd tests/bench
cargo build --release
cd ../..

echo "------------------------------------------------"
echo "Running Pub/Sub Benchmark (20k reqs, 100 conc)"
echo "------------------------------------------------"
./tests/bench/target/release/bench pub --topic bench-test --requests 20000 --concurrency 100

echo ""
echo "------------------------------------------------"
echo "Running Queue Benchmark (20k reqs, 100 conc)"
echo "------------------------------------------------"
./tests/bench/target/release/bench queue --name bench-queue --requests 20000 --concurrency 100

echo ""
echo "Done!"
