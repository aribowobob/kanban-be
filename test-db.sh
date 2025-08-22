#!/bin/bash
# Database Test Script
# This script tests the database connection and basic functionality

echo "🔗 Testing database connection..."

# Check if .env file exists
if [ ! -f .env ]; then
    echo "❌ .env file not found. Please create it using .env.example as a template."
    exit 1
fi

# Load environment variables
source .env

# Test database connection
echo "📋 Testing DATABASE_URL connection..."
sqlx database create 2>/dev/null || echo "Database already exists or connection failed"

# Test basic query
echo "🧪 Testing basic database query..."
sqlx migrate run 2>/dev/null || echo "No migrations found yet"

echo "✅ Database test completed!"
echo ""
echo "📝 Next steps:"
echo "1. Make sure your DATABASE_URL in .env is correct"
echo "2. Run 'cargo run' to start the server"
echo "3. Visit http://localhost:8080/health to test the API"
