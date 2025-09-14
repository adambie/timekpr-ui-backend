#!/bin/bash
# Development helper script

set -e

echo "🚀 TimKpr UI Development Helper"
echo "================================"

case "${1:-help}" in
  "build")
    echo "📦 Building in offline mode..."
    cargo build
    ;;
  "run")
    echo "🏃 Running development server..."
    cargo run
    ;;
  "fresh")
    echo "🆕 Starting with fresh database..."
    rm -f instance/timekpr.db
    touch instance/timekpr.db
    cargo run
    ;;
  "migrate")
    echo "🔄 Running migrations..."
    cargo sqlx migrate run
    ;;
  "prepare")
    echo "📋 Preparing SQLx offline data..."
    cargo sqlx prepare
    echo "✅ Don't forget to commit .sqlx/ directory!"
    ;;
  "reset-db")
    echo "⚠️  Resetting database..."
    cargo sqlx database reset
    ;;
  "check")
    echo "✅ Checking code..."
    cargo check
    cargo fmt --check
    cargo clippy -- -D warnings
    ;;
  "help"|*)
    echo "Available commands:"
    echo "  build     - Build the project (offline mode)"
    echo "  run       - Run development server"
    echo "  fresh     - Run with fresh database"
    echo "  migrate   - Run migrations"
    echo "  prepare   - Prepare SQLx offline data"
    echo "  reset-db  - Reset database"
    echo "  check     - Check code (format, clippy, build)"
    ;;
esac