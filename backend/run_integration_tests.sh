#!/usr/bin/env bash

set -e

echo "Building backend"
nix build --out-link debugBinary .#debugBinary
echo "Building tests"
nix build --out-link integrationTestsBinary .#integrationTestsBinary

echo "Creating database if not exists"
set +e
psql --dbname postgres -c "CREATE DATABASE rvoc_dev;"
set -e

echo "Clearing database"
diesel migration redo --all --locked-schema

echo "Starting backend in background"
RUST_BACKTRACE=1 debugBinary/bin/rvoc-backend web 2>&1 > >(tee rvoc-backend.log) &
BACKEND_PID=$!

set +e

echo "Waiting 5 seconds for backend to start"
sleep 5

echo "Running integration tests"
RUST_BACKTRACE=1 integrationTestsBinary/bin/integration-tests 2>&1 | tee integration-tests.log

echo "Terminating backend"
kill -SIGINT $BACKEND_PID
wait $BACKEND_PID
