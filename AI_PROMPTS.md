# AI Code Generation Templates

## For Database Operations

### Template for AI Prompts

````
Create a [function description] for the POS backend system.

CRITICAL REQUIREMENTS:
- Use sqlx::query() or sqlx::query_as() - NEVER use sqlx::query!()
- Use .bind() for all parameters
- Use proper error handling with ServiceError enum
- Include proper imports: use sqlx::{Row, FromRow}; use sqlx::postgres::PgRow;
- For type safety, create FromRow structs when needed
- Follow the existing patterns in the codebase

Example pattern to follow:
```rust
let result = sqlx::query("SELECT id, name FROM table WHERE condition = $1")
    .bind(parameter)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        error!("Database error: {}", e);
        ServiceError::DatabaseError(e.to_string())
    })?;
````

The function should be compatible with Docker builds (no compile-time database verification).

```

### Template for Complex Queries
```

Create a [function description] with the following requirements:

DATABASE REQUIREMENTS:

- Use runtime queries only (sqlx::query, NOT sqlx::query!)
- Create appropriate FromRow structs for type safety
- Handle pagination with LIMIT and OFFSET
- Include proper error handling with ServiceError
- Support optional search parameters
- Use dynamic query building when needed

DOCKER COMPATIBILITY:

- Must build successfully without database connection
- No compile-time query verification
- Use .bind() for all parameters

Follow the patterns established in the existing codebase.

```

## Quick Copy-Paste Instructions for AI

### Standard Database Function Request
```

IMPORTANT: Use sqlx::query() not sqlx::query!() for Docker compatibility. Use .bind() for parameters and proper ServiceError handling.

```

### For Search/Filter Functions
```

IMPORTANT: Use dynamic query building with sqlx::query(), not sqlx::query!(). Build query string dynamically and use .bind() for parameters. Must work in Docker builds.

```

### For Complex Joins
```

IMPORTANT: Use sqlx::query_as() with FromRow struct, not sqlx::query!(). Include all necessary imports and proper error handling. Must be Docker-compatible.

```

```
