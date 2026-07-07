use std::str::FromStr;

use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{FromRow, Row, SqlitePool};

use crate::game::player_info::PlayerInfo;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[derive(Debug, FromRow)]
pub struct Account {
    pub uid: i64,
}

#[derive(Debug, FromRow)]
pub struct SdkAccount {
    pub uid: i64,
    pub token: String,
    pub user_name: String,
    pub nick_name: String,
    pub created_at: i64,
    pub is_new: i64,
}

#[derive(Debug, FromRow)]
pub struct PlayerRow {
    pub is_banned: i64,
    pub data: String,
}

impl Database {
    pub async fn connect(url: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS accounts (
                uid INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL UNIQUE,
                token TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        self.ensure_account_column("user_name", "user_name TEXT")
            .await?;
        self.ensure_account_column("nick_name", "nick_name TEXT")
            .await?;
        self.ensure_account_column("created_at", "created_at INTEGER NOT NULL DEFAULT 0")
            .await?;
        self.ensure_account_column("is_new", "is_new INTEGER NOT NULL DEFAULT 0")
            .await?;

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_accounts_user_name ON accounts(user_name) WHERE user_name IS NOT NULL",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS players (
                uid INTEGER PRIMARY KEY,
                is_banned INTEGER NOT NULL DEFAULT 0,
                data TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn ensure_account_column(&self, name: &str, ddl: &str) -> Result<()> {
        let exists = sqlx::query("PRAGMA table_info(accounts)")
            .fetch_all(&self.pool)
            .await?
            .iter()
            .any(|row| row.get::<String, _>("name") == name);

        if !exists {
            sqlx::query(&format!("ALTER TABLE accounts ADD COLUMN {ddl}"))
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    pub async fn get_account(&self, uid: u32) -> Result<Option<Account>> {
        Ok(sqlx::query_as("SELECT uid FROM accounts WHERE uid = ?")
            .bind(uid as i64)
            .fetch_optional(&self.pool)
            .await?)
    }

    pub async fn create_sdk_account(
        &self,
        user_name: &str,
        token: &str,
        created_at: i64,
    ) -> Result<SdkAccount> {
        let device_id = format!("sdk:{user_name}");
        let result = sqlx::query(
            r#"
            INSERT INTO accounts (device_id, token, user_name, nick_name, created_at, is_new)
            VALUES (?, ?, ?, ?, ?, 1)
            "#,
        )
        .bind(device_id)
        .bind(token)
        .bind(user_name)
        .bind(user_name)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(SdkAccount {
            uid: result.last_insert_rowid(),
            token: token.to_string(),
            user_name: user_name.to_string(),
            nick_name: user_name.to_string(),
            created_at,
            is_new: 1,
        })
    }

    pub async fn get_sdk_account_by_user_name(
        &self,
        user_name: &str,
    ) -> Result<Option<SdkAccount>> {
        Ok(sqlx::query_as(
            r#"
            SELECT uid, token, user_name, COALESCE(nick_name, user_name) AS nick_name, created_at, is_new
            FROM accounts
            WHERE user_name = ?
            "#,
        )
        .bind(user_name)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn get_sdk_account(&self, uid: u32) -> Result<Option<SdkAccount>> {
        Ok(sqlx::query_as(
            r#"
            SELECT uid, token, user_name, COALESCE(nick_name, user_name) AS nick_name, created_at, is_new
            FROM accounts
            WHERE uid = ? AND user_name IS NOT NULL
            "#,
        )
        .bind(uid as i64)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn clear_sdk_new_flag(&self, uid: u32) -> Result<()> {
        sqlx::query("UPDATE accounts SET is_new = 0 WHERE uid = ?")
            .bind(uid as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_sdk_nickname(&self, uid: u32, nick_name: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE accounts SET nick_name = ? WHERE uid = ? AND user_name IS NOT NULL",
        )
        .bind(nick_name)
        .bind(uid as i64)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() != 0)
    }

    pub async fn get_player_row(&self, uid: u32) -> Result<Option<PlayerRow>> {
        Ok(
            sqlx::query_as("SELECT is_banned, data FROM players WHERE uid = ?")
                .bind(uid as i64)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    pub async fn load_or_create_player(&self, uid: u32) -> Result<PlayerInfo> {
        if let Some(row) = self.get_player_row(uid).await? {
            if let Ok(info) = serde_json::from_str(&row.data) {
                return Ok(info);
            }
        }

        let info = PlayerInfo::default();
        self.save_player(uid, &info).await?;
        Ok(info)
    }

    pub async fn save_player(&self, uid: u32, info: &PlayerInfo) -> Result<()> {
        let data = serde_json::to_string(info)?;
        sqlx::query(
            r#"
            INSERT INTO players (uid, is_banned, data)
            VALUES (?, COALESCE((SELECT is_banned FROM players WHERE uid = ?), 0), ?)
            ON CONFLICT(uid) DO UPDATE SET data = excluded.data
            "#,
        )
        .bind(uid as i64)
        .bind(uid as i64)
        .bind(data)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlite_account_and_player_roundtrip() {
        let db = Database::connect("sqlite::memory:").await.unwrap();

        let sdk = db
            .create_sdk_account("user@example.com", "token", 123)
            .await
            .unwrap();
        assert_eq!(sdk.user_name, "user@example.com");
        assert_eq!(sdk.nick_name, "user@example.com");
        assert_eq!(sdk.is_new, 1);

        let uid = sdk.uid as u32;
        let account = db.get_account(uid).await.unwrap().unwrap();
        assert_eq!(account.uid as u32, uid);

        let mut info = PlayerInfo::default();
        info.uid = uid;
        info.nick_name = "tester".to_string();
        db.save_player(uid, &info).await.unwrap();

        let loaded = db.load_or_create_player(uid).await.unwrap();
        assert_eq!(loaded.uid, uid);
        assert_eq!(loaded.nick_name, "tester");

        db.clear_sdk_new_flag(sdk.uid as u32).await.unwrap();
        assert!(db
            .update_sdk_nickname(sdk.uid as u32, "akiko97")
            .await
            .unwrap());

        let sdk = db.get_sdk_account(sdk.uid as u32).await.unwrap().unwrap();
        assert_eq!(sdk.is_new, 0);
        assert_eq!(sdk.nick_name, "akiko97");
    }
}
