use crate::recorder::PlatformType;

use super::Database;
use super::DatabaseError;
use chrono::Utc;
use rand::Rng;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AccountRow {
    pub platform: String,
    pub uid: u64,
    pub name: String,
    pub avatar: String,
    pub csrf: String,
    pub cookies: String,
    pub created_at: String,
}

// accounts
impl Database {
    // CREATE TABLE accounts (uid INTEGER PRIMARY KEY, name TEXT, avatar TEXT, csrf TEXT, cookies TEXT, created_at TEXT);
    pub async fn add_account(&self, platform: &str, cookies: &str) -> Result<AccountRow, DatabaseError> {
        let lock = self.db.read().await.clone().unwrap();
        let platform = PlatformType::from_str(platform).unwrap();

        let csrf = if platform == PlatformType::Douyin {
            Some("".to_string())
        } else {
            // parse cookies
            cookies
                .split(';')
                .map(|cookie| cookie.trim())
                .find_map(|cookie| -> Option<String> {
                    match cookie.starts_with("bili_jct=") {
                        true => {
                            let var_name = &"bili_jct=";
                            Some(cookie[var_name.len()..].to_string())
                        }
                        false => None,
                    }
                })
        };

        if csrf.is_none() {
            return Err(DatabaseError::InvalidCookiesError);
        }

        // parse uid
        let uid = if platform == PlatformType::BiliBili {
            cookies
                .split("DedeUserID=")
                .collect::<Vec<&str>>()
                .get(1)
                .unwrap()
                .split(";")
                .collect::<Vec<&str>>()
                .first()
                .unwrap()
                .to_string()
                .parse::<u64>()
                .map_err(|_| DatabaseError::InvalidCookiesError)?
        } else {
            // generate a random uid
            rand::thread_rng().gen_range(10000..=i32::MAX) as u64
        };

        let account = AccountRow {
            platform: platform.as_str().to_string(),
            uid,
            name: "".into(),
            avatar: "".into(),
            csrf: csrf.unwrap(),
            cookies: cookies.into(),
            created_at: Utc::now().to_rfc3339(),
        };

        sqlx::query("INSERT INTO accounts (uid, platform, name, avatar, csrf, cookies, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)").bind(account.uid as i64).bind(&account.platform).bind(&account.name).bind(&account.avatar).bind(&account.csrf).bind(&account.cookies).bind(&account.created_at).execute(&lock).await?;

        Ok(account)
    }

    pub async fn remove_account(&self, platform: &str, uid: u64) -> Result<(), DatabaseError> {
        let lock = self.db.read().await.clone().unwrap();
        let sql = sqlx::query("DELETE FROM accounts WHERE uid = $1 and platform = $2")
            .bind(uid as i64)
            .bind(platform)
            .execute(&lock)
            .await?;
        if sql.rows_affected() != 1 {
            return Err(DatabaseError::NotFoundError);
        }
        Ok(())
    }

    pub async fn update_account(
        &self,
        platform: &str,
        uid: u64,
        name: &str,
        avatar: &str,
    ) -> Result<(), DatabaseError> {
        let lock = self.db.read().await.clone().unwrap();
        let sql = sqlx::query("UPDATE accounts SET name = $1, avatar = $2 WHERE uid = $3 and platform = $4")
            .bind(name)
            .bind(avatar)
            .bind(uid as i64)
            .bind(platform)
            .execute(&lock)
            .await?;
        if sql.rows_affected() != 1 {
            return Err(DatabaseError::NotFoundError);
        }
        Ok(())
    }

    pub async fn get_accounts(&self) -> Result<Vec<AccountRow>, DatabaseError> {
        let lock = self.db.read().await.clone().unwrap();
        Ok(sqlx::query_as::<_, AccountRow>("SELECT * FROM accounts")
            .fetch_all(&lock)
            .await?)
    }

    pub async fn get_account(&self, platform: &str, uid: u64) -> Result<AccountRow, DatabaseError> {
        let lock = self.db.read().await.clone().unwrap();
        Ok(
            sqlx::query_as::<_, AccountRow>("SELECT * FROM accounts WHERE uid = $1 and platform = $2")
                .bind(uid as i64)
                .bind(platform)
                .fetch_one(&lock)
                .await?,
        )
    }

    pub async fn get_account_by_platform(&self, platform: &str) -> Result<AccountRow, DatabaseError> {
        let lock = self.db.read().await.clone().unwrap();
        Ok(sqlx::query_as::<_, AccountRow>("SELECT * FROM accounts WHERE platform = $1")
            .bind(platform)
            .fetch_one(&lock)
            .await?)
    }
}
