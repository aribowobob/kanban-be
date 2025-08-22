-- Kanban Board Database Schema
-- PostgreSQL Database Schema for Neon.com
-- Run this script in your Neon console to set up the database

-- Enable UUID extension (if not already enabled)
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 1. Users table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    password VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 2. Teams table (with predefined data)
CREATE TABLE teams (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Insert predefined teams
INSERT INTO teams (name) VALUES 
    ('DESIGN'), 
    ('BACKEND'), 
    ('FRONTEND');

-- 3. Tasks table
CREATE TABLE tasks (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    status VARCHAR(20) NOT NULL CHECK (status IN ('TO_DO', 'DOING', 'DONE')),
    external_link TEXT, -- For Google Docs/Forms URLs
    created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 4. Task-Teams junction table (many-to-many relationship)
CREATE TABLE task_teams (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(task_id, team_id) -- Prevent duplicate team assignments
);

-- 5. Task attachments (for Cloudinary storage)
CREATE TABLE task_attachments (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    file_name VARCHAR(255) NOT NULL,
    file_size BIGINT NOT NULL, -- Size in bytes
    mime_type VARCHAR(100) NOT NULL,
    cloudinary_public_id VARCHAR(255) NOT NULL, -- Cloudinary public ID
    cloudinary_url TEXT NOT NULL, -- Full Cloudinary URL
    cloudinary_secure_url TEXT NOT NULL, -- HTTPS Cloudinary URL
    uploaded_by INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for better query performance
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_tasks_created_by ON tasks(created_by);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_created_at ON tasks(created_at);
CREATE INDEX idx_task_teams_task_id ON task_teams(task_id);
CREATE INDEX idx_task_teams_team_id ON task_teams(team_id);
CREATE INDEX idx_task_attachments_task_id ON task_attachments(task_id);
CREATE INDEX idx_task_attachments_cloudinary_public_id ON task_attachments(cloudinary_public_id);

-- Function to automatically update the updated_at column
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger to automatically update updated_at on tasks table
CREATE TRIGGER update_tasks_updated_at 
    BEFORE UPDATE ON tasks
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

-- Insert a default admin user for testing (password: 'admin123')
-- Note: This is a bcrypt hash of 'admin123' - change this in production!
INSERT INTO users (username, password, name) VALUES 
    ('admin', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj8LhQnE.K6W', 'Administrator');

-- Insert some sample tasks for testing (optional - remove in production)
-- Uncomment the following lines if you want sample data:

/*
INSERT INTO tasks (name, description, status, external_link, created_by) VALUES 
    ('Setup Database Schema', 'Create and configure PostgreSQL database schema for the kanban application', 'DONE', 'https://docs.google.com/document/d/sample1', 1),
    ('Implement Authentication API', 'Create login, logout, and user verification endpoints', 'DOING', NULL, 1),
    ('Design Task Management UI', 'Create wireframes and mockups for task management interface', 'TO_DO', 'https://www.figma.com/sample', 1);

-- Assign teams to sample tasks
INSERT INTO task_teams (task_id, team_id) VALUES 
    (1, 2), -- Setup Database Schema -> BACKEND
    (2, 2), -- Implement Authentication API -> BACKEND  
    (3, 1), -- Design Task Management UI -> DESIGN
    (3, 3); -- Design Task Management UI -> FRONTEND
*/

-- Verify the schema creation
SELECT 'Database schema created successfully!' as status;

-- Show table information
SELECT 
    table_name, 
    column_name, 
    data_type, 
    is_nullable,
    column_default
FROM information_schema.columns 
WHERE table_schema = 'public' 
    AND table_name IN ('users', 'teams', 'tasks', 'task_teams', 'task_attachments')
ORDER BY table_name, ordinal_position;
