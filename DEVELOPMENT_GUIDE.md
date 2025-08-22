# Development Guide for Kanban Backend

## Table of Contents

1. [API Response Format](#api-response-format)
2. [SQLx Database Query Guidelines](#sqlx-database-query-guidelines)
3. [Docker Build Best Practices](#docker-build-best-practices)
4. [CI/CD Guidelines](#cicd-guidelines)
5. [Code Generation with AI](#code-generation-with-ai)
6. [Error Handling Patterns](#error-handling-patterns)

## API Response Format

### Standardized Response Structure

All API endpoints must follow this standardized response format for consistency:

```json
{
  "status": "success" | "error",
  "message": "Human readable message",
  "data": Object | Array | Boolean | null
}
```

### Success Response Guidelines

#### 1. Object Response (Single Resource)

```json
{
  "status": "success",
  "message": "Successfully retrieved user data",
  "data": {
    "id": 1,
    "username": "john_doe",
    "name": "John Doe",
    "created_at": "2025-08-22T09:42:10.264443Z",
    "updated_at": "2025-08-22T09:42:10.264443Z"
  }
}
```

**When resource not found:**

```json
{
  "status": "success",
  "message": "No user found",
  "data": null
}
```

#### 2. Array Response (Multiple Resources)

```json
{
  "status": "success",
  "message": "Successfully retrieved tasks",
  "data": [
    { "id": 1, "title": "Task 1" },
    { "id": 2, "title": "Task 2" }
  ]
}
```

**When no items found:**

```json
{
  "status": "success",
  "message": "No tasks found",
  "data": []
}
```

#### 3. Boolean Response (Action Confirmation)

```json
{
  "status": "success",
  "message": "Successfully logout from the system",
  "data": true
}
```

### Error Response Guidelines

Error responses should only contain `status` and `message`:

```json
{
  "status": "error",
  "message": "Invalid credentials"
}
```

### Implementation in Rust

Use the `ApiResponse<T>` struct for all success responses:

```rust
use crate::models::auth::ApiResponse;

// For object responses
Ok(HttpResponse::Ok().json(ApiResponse::success("User retrieved", user_data)))

// For array responses
Ok(HttpResponse::Ok().json(ApiResponse::success("Tasks retrieved", tasks_list)))

// For boolean responses
Ok(HttpResponse::Ok().json(ApiResponse::success("Operation completed", true)))
```

### Current Endpoints

- **POST /api/auth/login** - Returns login data with token and user object
- **POST /api/auth/logout** - Returns boolean confirmation
- **GET /api/auth/me** - Returns user object or null if not found

## SQLx Database Query Guidelines

### ⚠️ CRITICAL: Always Use Runtime Queries for CI/CD Compatibility

**NEVER use `sqlx::query!` macro in this project.** Always use `sqlx::query` with manual type mapping.

#### ❌ DON'T DO THIS:

```rust
// This will fail in Docker builds because it requires database connection at compile time
let result = sqlx::query!(
    "SELECT id, name FROM users WHERE email = $1",
    email
)
.fetch_one(&pool)
.await?;
```

#### ✅ DO THIS INSTEAD:

```rust
// This works in Docker builds because it uses runtime verification
let result = sqlx::query(
    "SELECT id, name FROM users WHERE email = $1"
)
.bind(email)
.fetch_one(&pool)
.await?;

// For type safety, create a struct and use query_as:
#[derive(FromRow)]
struct UserResult {
    id: i32,
    name: String,
}

let result = sqlx::query_as::<_, UserResult>(
    "SELECT id, name FROM users WHERE email = $1"
)
.bind(email)
.fetch_one(&pool)
.await?;
```

### Required Imports

Always include these imports when working with SQLx:

```rust
use sqlx::{Row, FromRow};
use sqlx::postgres::PgRow;
```

### Pattern for Manual Row Mapping

When you need to manually extract values from rows:

```rust
let user = User {
    id: row.try_get::<i32, _>("id")?,
    email: row.try_get::<String, _>("email")?,
    company_id: row.try_get::<i32, _>("company_id")?,
    // ... other fields
};
```

### Pattern for Dynamic Queries

For queries with optional search parameters:

```rust
let mut query = String::from("SELECT * FROM table WHERE 1=1");
let mut params = Vec::new();
let mut param_index = 1;

if let Some(search_term) = search {
    query.push_str(&format!(" AND name ILIKE ${}", param_index));
    params.push(format!("%{}%", search_term));
    param_index += 1;
}

let mut query_builder = sqlx::query(&query);
for param in params {
    query_builder = query_builder.bind(param);
}
```

## Docker Build Best Practices

### Dockerfile Requirements

- Never set `ENV SQLX_OFFLINE=true` unless you have a complete offline setup
- Always use runtime queries instead of compile-time verification
- Keep dependencies minimal

### Environment Variables in Docker

```dockerfile
# ✅ Good - these don't require database connection
ENV RUST_LOG=info
ENV PORT=8080

# ❌ Avoid - this requires SQLx offline mode setup
# ENV SQLX_OFFLINE=true
```

## CI/CD Guidelines

### GitHub Actions Deployment

1. **Build Phase**: Should not require database connection
2. **Deploy Phase**: Database connection is available on the server
3. **Always test locally with `cargo build --release` before pushing**

### Testing Before Deployment

```bash
# Always run these before pushing:
cargo check
cargo build --release
```

## Code Generation with AI

### When Requesting AI Code Generation

#### ✅ Always Include These Instructions:

```
IMPORTANT DATABASE QUERY REQUIREMENTS:
- Use sqlx::query() instead of sqlx::query!()
- Use sqlx::query_as() with FromRow structs for type safety
- Never use compile-time verified queries (sqlx::query!())
- Always use .bind() for parameters
- Include proper error handling with ServiceError
```

#### Example AI Prompt:

```
Create a function to get users by email. IMPORTANT: Use sqlx::query() not sqlx::query!()
because we need runtime verification for Docker builds. Use proper error handling
with ServiceError and manual row mapping.
```

### Code Review Checklist for AI-Generated Code

Before accepting AI-generated database code, check:

- [ ] Uses `sqlx::query()` or `sqlx::query_as()` (NOT `sqlx::query!()`)
- [ ] Uses `.bind()` for parameters
- [ ] Has proper error handling with `ServiceError`
- [ ] Includes required imports (`sqlx::Row`, `sqlx::FromRow`)
- [ ] Uses `try_get()` for safe value extraction

## Error Handling Patterns

### Standard Error Handling for Database Operations

```rust
match sqlx::query("SELECT * FROM table")
    .fetch_one(&pool)
    .await
{
    Ok(row) => {
        // Handle success
    },
    Err(e) => {
        error!("Database error: {}", e);
        return Err(ServiceError::DatabaseError(e.to_string()));
    }
}
```

### Connection Pool Error Handling

```rust
let pool = match db_manager.get_pool().await {
    Ok(pool) => pool,
    Err(e) => {
        error!("Failed to get database connection: {:?}", e);
        return Err(ServiceError::DatabaseConnectionError);
    }
};
```

## Quick Reference

### Convert Existing sqlx::query! to sqlx::query

1. Find all `sqlx::query!` in your code
2. Replace with `sqlx::query`
3. Add `.bind()` for each parameter
4. Create a `FromRow` struct if needed for type safety
5. Test with `cargo build --release`

### Search for Problematic Patterns

```bash
# Find all potential issues:
grep -r "sqlx::query!" src/
```

## Common Mistakes to Avoid

1. **Using `sqlx::query!` anywhere in the codebase**
2. **Setting `SQLX_OFFLINE=true` without proper setup**
3. **Not testing Docker builds locally**
4. **Forgetting to use `.bind()` for parameters**
5. **Not handling database errors properly**

---

## Emergency Fix for SQLx Issues

If you encounter "set `DATABASE_URL` to use query macros online" error:

1. **Quick Fix**: Convert the problematic query to runtime query
2. **Find the error**: Look for `sqlx::query!` in the error location
3. **Replace with**: `sqlx::query` + `.bind()` + manual mapping
4. **Test**: Run `cargo build --release`
5. **Deploy**: Push the changes

Remember: **Always prefer runtime queries over compile-time queries** for this project to maintain CI/CD compatibility.

---

## 🔧 API Endpoints Documentation

All endpoints follow a standardized response format for consistency and better frontend integration.

### Response Format

All API responses follow this consistent structure:

```json
{
  "status": "success" | "error",
  "message": "Human-readable message",
  "data": T | null  // The actual response data, type varies by endpoint
}
```

#### Success Response Example:

```json
{
  "status": "success",
  "message": "User retrieved successfully",
  "data": {
    "id": 1,
    "username": "admin",
    "name": "Administrator"
  }
}
```

#### Error Response Example:

```json
{
  "status": "error",
  "message": "Invalid credentials",
  "data": null
}
```

### Authentication Endpoints

#### POST `/api/auth/login`

Login with username and password

**Request:**

```json
{
  "username": "admin",
  "password": "admin123"
}
```

**Response:**

```json
{
  "status": "success",
  "message": "Login successful",
  "data": {
    "token": "eyJ0eXAiOiJKV1Q...",
    "user": {
      "id": 2,
      "username": "admin",
      "name": "Administrator",
      "created_at": "2025-08-22T09:42:10.264443Z",
      "updated_at": "2025-08-22T09:42:10.264443Z"
    }
  }
}
```

#### POST `/api/auth/logout`

Logout (invalidate token)

**Response:**

```json
{
  "status": "success",
  "message": "Logout successful",
  "data": true
}
```

#### GET `/api/auth/me`

Get current user information

**Headers:** `Authorization: Bearer <token>`

**Response:**

```json
{
  "status": "success",
  "message": "User retrieved successfully",
  "data": {
    "id": 2,
    "username": "admin",
    "name": "Administrator",
    "created_at": "2025-08-22T09:42:10.264443Z",
    "updated_at": "2025-08-22T09:42:10.264443Z"
  }
}
```

### Task Management Endpoints

All task endpoints require authentication via Bearer token in the Authorization header.

#### GET `/api/tasks`

Get all tasks

**Headers:** `Authorization: Bearer <token>`

**Response:**

```json
{
  "status": "success",
  "message": "Tasks retrieved successfully",
  "data": [
    {
      "id": 1,
      "name": "Implement Login Form",
      "description": "Create a responsive login form with validation and error handling",
      "status": "DOING",
      "external_link": "https://github.com/project/issues/1",
      "created_by": 2,
      "teams": ["Frontend", "QA"],
      "created_at": "2025-08-22T11:21:12.440982Z",
      "updated_at": "2025-08-22T11:21:48.575477Z"
    }
  ]
}
```

#### GET `/api/tasks/{id}`

Get a specific task by ID

**Headers:** `Authorization: Bearer <token>`

**Response:**

```json
{
  "status": "success",
  "message": "Task retrieved successfully",
  "data": {
    "id": 1,
    "name": "Implement Login Form",
    "description": "Create a responsive login form with validation and error handling",
    "status": "DOING",
    "external_link": "https://github.com/project/issues/1",
    "created_by": 2,
    "teams": ["Frontend", "QA"],
    "created_at": "2025-08-22T11:21:12.440982Z",
    "updated_at": "2025-08-22T11:21:48.575477Z"
  }
}
```

#### POST `/api/tasks`

Create a new task

**Headers:** `Authorization: Bearer <token>`

**Request:**

```json
{
  "name": "Implement Login Form",
  "description": "Create a responsive login form with validation and error handling",
  "status": "TO_DO",
  "teams": ["Frontend"],
  "external_link": "https://github.com/project/issues/1"
}
```

**Response:**

```json
{
  "status": "success",
  "message": "Task created successfully",
  "data": {
    "id": 1,
    "name": "Implement Login Form",
    "description": "Create a responsive login form with validation and error handling",
    "status": "TO_DO",
    "external_link": "https://github.com/project/issues/1",
    "created_by": 2,
    "teams": ["Frontend"],
    "created_at": "2025-08-22T11:21:12.440982Z",
    "updated_at": "2025-08-22T11:21:12.440982Z"
  }
}
```

#### PUT `/api/tasks/{id}`

Update an existing task

**Headers:** `Authorization: Bearer <token>`

**Request (all fields optional):**

```json
{
  "name": "Updated Task Name",
  "description": "Updated description",
  "status": "DOING",
  "teams": ["Frontend", "QA"],
  "external_link": "https://github.com/project/issues/1"
}
```

**Response:**

```json
{
  "status": "success",
  "message": "Task updated successfully",
  "data": {
    "id": 1,
    "name": "Updated Task Name",
    "description": "Updated description",
    "status": "DOING",
    "external_link": "https://github.com/project/issues/1",
    "created_by": 2,
    "teams": ["Frontend", "QA"],
    "created_at": "2025-08-22T11:21:12.440982Z",
    "updated_at": "2025-08-22T11:21:48.575477Z"
  }
}
```

#### DELETE `/api/tasks/{id}`

Delete a task

**Headers:** `Authorization: Bearer <token>`

**Response:**

```json
{
  "status": "success",
  "message": "Task deleted successfully",
  "data": true
}
```

### Team Management Endpoints

#### GET `/api/teams`

Get all available teams

**Headers:** `Authorization: Bearer <token>`

**Response:**

```json
{
  "status": "success",
  "message": "Teams retrieved successfully",
  "data": [
    {
      "id": 1,
      "name": "Frontend",
      "created_at": "2025-08-22T11:11:18.023216Z"
    },
    {
      "id": 2,
      "name": "Backend",
      "created_at": "2025-08-22T11:11:18.023216Z"
    }
  ]
}
```

### Task Status Values

Tasks can have one of the following status values:

- `TO_DO` - Task is not started
- `DOING` - Task is in progress
- `DONE` - Task is completed

### Error Handling

All endpoints return appropriate HTTP status codes:

- `200` - Success
- `201` - Created
- `400` - Bad Request (validation errors)
- `401` - Unauthorized (invalid/missing token)
- `404` - Not Found
- `500` - Internal Server Error

Error responses follow the same format:

```json
{
  "status": "error",
  "message": "Detailed error message",
  "data": null
}
```
