# Kanban Backend

A kanban board backend system built with Rust and Actix-web

## 📚 Important Documentation

- **[DEVELOPMENT_GUIDE.md](./DEVELOPMENT_GUIDE.md)** - Essential reading for database queries and Docker compatibility

## 🗄️ Database Setup

### New Installation

Run the `kanban_db.sql` script to create all tables and sample data.

## Features

- REST API built with Actix-web
- JWT token-based authentication
- PostgreSQL database integration with connection pooling
- Structured error handling with custom error types
- Health check endpoints
- OpenAPI/Swagger documentation (available only in development mode)
- CORS support for frontend integration
- Logging and monitoring capabilities

## Required GitHub Secrets/Variables

### Secrets:

- DOCKER_USERNAME
- DOCKER_PASSWORD
- DROPLET_PASSWORD
- DATABASE_URL
- JWT_SECRET
- CLOUDINARY_CLOUD_NAME
- CLOUDINARY_API_KEY
- CLOUDINARY_API_SECRET

### Variables

- DROPLET_IP
- FRONTEND_URLS
