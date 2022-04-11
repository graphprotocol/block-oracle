use super::models;
use sqlx::postgres::PgPoolOptions;

type PgPool = sqlx::Pool<sqlx::Postgres>;

pub struct Store {
    pool: PgPool,
}

impl Store {
    pub async fn new(db_url: &str) -> sqlx::Result<Self> {
        let pool = PgPoolOptions::new().connect(db_url).await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn insert_network(&self, chain_id: models::Caip2ChainId) -> sqlx::Result<()> {
        sqlx::query_with(
            "INSERT INTO networks (chain_id) VALUES ($1)",
            [chain_id.as_str()],
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
