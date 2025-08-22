# Kanban Backend

A kanban board backend system built with Rust and Actix-web

## 📚 Important Documentation

- **[DEVELOPMENT_GUIDE.md](./DEVELOPMENT_GUIDE.md)** - Essential reading for database queries and Docker compatibility

## 🗄️ Database Setup

### New Installation

Run the `kanban_db.sql` script to create all tables and sample data.

### Existing Database Migration

If you have an existing database with the old `password_hash` column, run:

```sql
-- Run this migration to rename the column
ALTER TABLE users RENAME COLUMN password_hash TO password;
```

Or use the provided migration file: `migrate_password_column.sql`

## Features

- REST API built with Actix-web
- JWT token-based authentication
- PostgreSQL database integration with connection pooling
- Structured error handling with custom error types
- Health check endpoints
- OpenAPI/Swagger documentation (available only in development mode)
- CORS support for frontend integration
- Logging and monitoring capabilities
