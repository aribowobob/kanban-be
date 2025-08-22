#!/bin/bash

# Test script for the Task API endpoints
BASE_URL="http://localhost:8080"

echo "Testing Kanban Backend API..."
echo "==============================="

# Test health endpoint first
echo "1. Testing health endpoint..."
curl -s "$BASE_URL/health" | jq '.' || echo "Health endpoint failed"

echo -e "\n2. Testing login endpoint..."
# Test login with the default admin user
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}')

echo "$LOGIN_RESPONSE" | jq '.' || echo "Login failed"

# Extract token from login response
TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.data.token // empty')

if [ -z "$TOKEN" ]; then
    echo "Failed to get authentication token. Cannot continue with authenticated tests."
    exit 1
fi

echo -e "\n3. Testing GET /api/tasks endpoint..."
curl -s -H "Authorization: Bearer $TOKEN" "$BASE_URL/api/tasks" | jq '.' || echo "GET /api/tasks failed"

echo -e "\n4. Testing GET /api/tasks/1 endpoint..."
curl -s -H "Authorization: Bearer $TOKEN" "$BASE_URL/api/tasks/1" | jq '.' || echo "GET /api/tasks/1 failed"

echo -e "\nAPI tests completed!"
