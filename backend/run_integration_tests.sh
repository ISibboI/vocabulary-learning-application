#!/usr/bin/env bash

echo "Building backend"
nix build --out-link debugBinary .#debugBinary
echo "Building tests"
nix build --out-link integrationTestsBinary .#integrationTestsBinary

echo "Applying database migrations"
debugBinary/bin/rvoc-backend apply-migrations

echo "Starting backend in background"
RUST_BACKTRACE=1 debugBinary/bin/rvoc-backend web &
BACKEND_PID=$!

echo "Waiting 5 seconds for backend to start"
sleep 5

echo "Running integration tests"
RUST_BACKTRACE=1 integrationTestsBinary/bin/integration-tests

echo "Terminating backend"
kill -SIGINT $BACKEND_PID
wait $BACKEND_PID