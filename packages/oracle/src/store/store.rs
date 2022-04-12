use super::models;
use async_trait::async_trait;
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

    pub async fn last_encoding_version(&self) -> sqlx::Result<Option<models::EncodingVersion>> {
        let row: Option<(i64,)> = sqlx::query_as(
            r#"
SELECT id,
FROM encoding_versions
ORDER BY id DESC
LIMIT 1"#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0 as models::EncodingVersion))
    }

    pub async fn insert_encoding_version(
        &self,
        version: models::EncodingVersion,
        data_edge_call_id: models::Id,
    ) -> sqlx::Result<models::Id> {
        let row: (models::Id,) = sqlx::query_as(
            r#"
INSERT INTO encoding_versions (chain_id, introduced_with)
VALUES ($1, $2)
RETURNING id"#,
        )
        .bind(version as i64)
        .bind(data_edge_call_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    pub async fn insert_network(
        &self,
        chain_id: models::Caip2ChainId,
        data_edge_call_id: models::Id,
    ) -> sqlx::Result<models::Id> {
        let row: (models::Id,) = sqlx::query_as(
            r#"
INSERT INTO networks (chain_id, introduced_with)
VALUES ($1, $2)
RETURNING id"#,
        )
        .bind(chain_id.as_str())
        .bind(data_edge_call_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    pub async fn networks(&self) -> sqlx::Result<Vec<models::Caip2ChainId>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
SELECT (chain_id)
FROM networks
ORDER BY chain_id ASC"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|id| models::Caip2ChainId::parse(id.0.as_str()).unwrap())
            .collect())
    }

    pub async fn insert_data_edge_call(
        &self,
        call: models::DataEdgeCall,
    ) -> sqlx::Result<models::Id> {
        let row: (models::Id,) = sqlx::query_as(
            r#"
INSERT INTO networks (
    tx_hash,
    nonce,
    num_confirmations,
    num_confirmations_last_checked_at,
    block_number,
    block_hash,
    payload)
VALUES ($1, $2, $3, $4, $5, $6, $7)
RETURNING id"#,
        )
        .bind(call.tx_hash)
        .bind(call.nonce as i64)
        .bind(call.num_confirmations as i64)
        .bind(call.num_confirmations_last_checked_at)
        .bind(call.block_number as i64)
        .bind(call.block_hash)
        .bind(call.payload)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    pub async fn insert_nonce(&self, nonce: i64) -> sqlx::Result<()> {
        sqlx::query(
            r#"
INSERT INTO nonces (nonce)
VALUES ($1)"#,
        )
        .bind(nonce)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn last_nonce(&self) -> sqlx::Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
SELECT nonce,
FROM nonces
ORDER BY nonce DESC
LIMIT 1"#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }
}

//#[async_trait]
//impl epoch_encoding::Database for Store {
//    type Error = sqlx::Error;
//
//    async fn get_next_nonce(&self) -> Result<u64, Self::Error> {
//        self.last_nonce().await.map(|nonce| nonce as u64 + 1)
//    }
//
//    async fn set_next_nonce(&mut self, nonce: u64) -> Result<(), Self::Error> {
//        self.insert_nonce(nonce as i64).await
//    }
//
//    async fn get_network_ids(&self) -> Result<HashMap<String, NetworkId>, Self::Error>;
//
//    async fn set_network_ids(
//        &mut self,
//        ids: HashMap<String, NetworkId>,
//    ) -> Result<(), Self::Error> {
//        let id = self.insert_network(chain_id, data_edge_call_id);
//    }
//
//    async fn get_network(&self, id: NetworkId) -> Result<Option<Network>, Self::Error>;
//
//    async fn set_network(&mut self, id: NetworkId, network: Network) -> Result<(), Self::Error>;
//}
