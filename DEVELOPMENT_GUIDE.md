# Development Guide for Kanban Backend

## Table of Contents

1. [SQLx Database Query Guidelines](#sqlx-database-query-guidelines)
2. [Docker Build Best Practices](#docker-build-best-practices)
3. [CI/CD Guidelines](#cicd-guidelines)
4. [Code Generation with AI](#code-generation-with-ai)
5. [Error Handling Patterns](#error-handling-patterns)

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
