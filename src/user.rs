use std::str::FromStr;

use argon2::{
    password_hash::{rand_core::OsRng, Error, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use sqlx::{
    pool::PoolConnection,
    query,
    sqlite::{SqliteQueryResult, SqliteRow},
    Row, Sqlite,
};

use crate::db::{utc_from_str, utc_to_str};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct User {
    pub email: EmailAddress,
    pub password: String,
    pub roles: String,
    pub active: bool,
    #[serde(default = "Utc::now")]
    pub created: DateTime<Utc>,
}

impl User {
    /// Insert into or Replace `User` in users table in SQLite database
    ///
    /// | Name | Type | Comment | Extended Comment
    /// :--- | :--- | :--- | ---
    /// | email | TEXT | Email Address
    /// | password | TEXT |
    /// | roles | TEXT |
    /// | active | NUMERIC |
    /// | created | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        let q =
            r#"REPLACE INTO users(email, password, roles, active, created) VALUES(?, ?, ?, ?, ?)"#;
        query(q)
            .bind(self.email.to_string())
            .bind(self.password)
            .bind(self.roles)
            .bind(self.active)
            .bind(utc_to_str(self.created))
            .execute(&mut *connection)
            .await
            .unwrap()
    }
    pub fn verify_password(&self, password: &[u8]) -> Result<(), Error> {
        let parsed_hash = PasswordHash::new(&self.password)?;
        Argon2::default().verify_password(password, &parsed_hash)
    }
}

/// Hash password for storage
pub fn hash_password(password: &[u8]) -> Result<String, Error> {
    let salt = SaltString::generate(&mut OsRng);
    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();
    // Hash password to PHC string ($argon2id$v=19$...)
    Ok(argon2.hash_password(password, &salt)?.to_string())
    // TODO: Add Pepper!
}

/// Convert `SqliteRow` in `User` struct
impl From<SqliteRow> for User {
    fn from(s: SqliteRow) -> Self {
        User {
            email: EmailAddress::from_str(&s.get::<String, _>("email")).unwrap(),
            password: s.get::<String, _>("password"),
            roles: s.get::<String, _>("roles"),
            active: s.get::<bool, _>("active"),
            created: utc_from_str(&s.get::<String, _>("created")),
        }
    }
}

pub async fn get_users_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> Vec<User> {
    let stmt = if let Some(f) = filter {
        format!("SELECT * FROM users WHERE {f}")
    } else {
        "SELECT * FROM users".into()
    };
    let users = match query(&stmt).fetch_all(&mut *connection).await {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    users.into_iter().map(|s| s.into()).collect()
}

pub async fn count_rows(connection: PoolConnection<Sqlite>) -> Result<i64, sqlx::Error> {
    crate::db::count_rows("users", connection).await
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::db::{create_database, init_database};
    use tracing_subscriber::{
        fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
    };

    // #[tokio::test]
    // async fn test_users() {
    //     registry()
    //         .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
    //         .with(fmt::layer())
    //         .try_init()
    //         .unwrap_or(());

    //     let pool = create_database("sqlite::memory:").await.unwrap();

    //     init_database(&pool, None).await.unwrap();

    //     // init new testuser
    //     let password = b"test123";
    //     let hashed_pw = hash_password(password).unwrap();
    //     let new_user = User {
    //         email: EmailAddress::from_str("test@test.int").unwrap(),
    //         password: hashed_pw,
    //         roles: "".into(),
    //         active: true,
    //         created: Utc::now(),
    //     };
    //     let _i1 = new_user.insert_into_db(pool.acquire().await.unwrap()).await;
    //     assert_eq!(
    //         count_rows(pool.acquire().await.unwrap()).await.unwrap_or(0),
    //         1
    //     );

    //     // get all users
    //     let users = get_users_from_db(None, pool.acquire().await.unwrap()).await;
    //     assert_eq!(users.len(), 1);

    //     // get testuser and verify pw
    //     let users =
    //         get_users_from_db(Some("email='test@test.int'"), pool.acquire().await.unwrap()).await;
    //     let user = users.first().unwrap();
    //     assert!(user.verify_password(password).is_ok());
    // }
}
