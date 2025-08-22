use sqlx::{PgPool, Row};
use anyhow::{Result, Context};

pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        log::info!("🔗 Connecting to database...");
        
        let pool = PgPool::connect(database_url)
            .await
            .context("Failed to connect to the database")?;

        log::info!("✅ Database connection established");

        Ok(Database { pool })
    }

    pub async fn health_check(&self) -> Result<()> {
        log::info!("🔍 Running database health check...");
        
        let row = sqlx::query("SELECT 1 as health_check")
            .fetch_one(&self.pool)
            .await
            .context("Failed to execute health check query")?;

        let result: i32 = row.get("health_check");
        
        if result == 1 {
            log::info!("✅ Database health check passed");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Database health check failed"))
        }
    }

    pub async fn check_tables(&self) -> Result<()> {
        log::info!("📋 Checking database tables...");

        let tables = sqlx::query(
            r#"
            SELECT table_name 
            FROM information_schema.tables 
            WHERE table_schema = 'public' 
            AND table_name IN ('users', 'teams', 'tasks', 'task_teams', 'task_attachments')
            ORDER BY table_name
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to check database tables")?;

        let expected_tables = vec!["task_attachments", "task_teams", "tasks", "teams", "users"];
        let found_tables: Vec<String> = tables
            .iter()
            .map(|row| row.get::<String, _>("table_name"))
            .collect();

        log::info!("📊 Found tables: {:?}", found_tables);

        if found_tables.len() == expected_tables.len() {
            log::info!("✅ All required tables exist");
        } else {
            log::warn!("⚠️  Some tables may be missing. Expected: {:?}", expected_tables);
            log::warn!("   Run the kanban_db.sql script in your Neon database if tables are missing");
        }

        Ok(())
    }

    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let stats = sqlx::query(
            r#"
            SELECT 
                (SELECT COUNT(*) FROM users) as user_count,
                (SELECT COUNT(*) FROM teams) as team_count,
                (SELECT COUNT(*) FROM tasks) as task_count,
                (SELECT COUNT(*) FROM task_attachments) as attachment_count
            "#
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to get database statistics")?;

        Ok(DatabaseStats {
            users: stats.get::<i64, _>("user_count"),
            teams: stats.get::<i64, _>("team_count"),
            tasks: stats.get::<i64, _>("task_count"),
            attachments: stats.get::<i64, _>("attachment_count"),
        })
    }
}

#[derive(Debug)]
pub struct DatabaseStats {
    pub users: i64,
    pub teams: i64,
    pub tasks: i64,
    pub attachments: i64,
}

impl DatabaseStats {
    pub fn log_stats(&self) {
        log::info!("📈 Database Statistics:");
        log::info!("   👥 Users: {}", self.users);
        log::info!("   🏢 Teams: {}", self.teams);
        log::info!("   📋 Tasks: {}", self.tasks);
        log::info!("   📎 Attachments: {}", self.attachments);
    }
}
