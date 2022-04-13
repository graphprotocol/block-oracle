use super::models;
use async_trait::async_trait;
use epoch_encoding as ee;
use models::WithId;
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashMap, str::FromStr};

type PgPool = sqlx::Pool<sqlx::Postgres>;
type NetworkRow = (i64, String, Option<i64>, Option<Vec<u8>>, Option<i64>, i64);

fn network_row_to_model(row: NetworkRow) -> WithId<models::Network> {
    let (
        id,
        chain_name,
        latest_block_number,
        latest_block_hash,
        latest_block_delta,
        introduced_with,
    ) = row;
    WithId {
        id: models::Id::try_from(id).unwrap(),
        data: models::Network {
            name: models::Caip2ChainId::from_str(chain_name.as_str()).unwrap(),
            latest_block_number: latest_block_number.map(|x| x.try_into().unwrap()),
            latest_block_hash,
            latest_block_delta: latest_block_delta.map(|x| x.try_into().unwrap()),
            introduced_with: models::Id::try_from(introduced_with).unwrap(),
        },
    }
}

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
        let row: (i64,) = sqlx::query_as(
            r#"
INSERT INTO encoding_versions (chain_id, introduced_with)
VALUES ($1, $2)
RETURNING id"#,
        )
        .bind(i64::try_from(version).unwrap())
        .bind(i64::try_from(data_edge_call_id).unwrap())
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0.try_into().unwrap())
    }

    pub async fn insert_network(
        &self,
        network: WithId<models::Network>,
    ) -> sqlx::Result<models::Id> {
        let row: (i64,) = sqlx::query_as(
            r#"
INSERT INTO networks (id, chain_id, introduced_with)
VALUES ($1, $2, $3)
RETURNING id"#,
        )
        .bind(i64::try_from(network.id).unwrap())
        .bind(network.data.name.as_str())
        .bind(i64::try_from(network.data.introduced_with).unwrap())
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0.try_into().unwrap())
    }

    pub async fn update_network_block_info(
        &self,
        id: models::Id,
        latest_block_number: u64,
        latest_block_delta: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
UPDATE networks
SET latest_block_number = $1, latest_block_delta = $2
where id = $3"#,
        )
        .bind(i64::try_from(latest_block_number).unwrap())
        .bind(latest_block_delta)
        .bind(i64::try_from(id).unwrap())
        .fetch_one(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn network_by_id(
        &self,
        id: models::Id,
    ) -> sqlx::Result<Option<WithId<models::Network>>> {
        let row: Option<NetworkRow> = sqlx::query_as(
            r#"
SELECT (id, chain_id, latest_block_number, latest_block_hash, latest_block_delta, introduced_with)
FROM networks
WHERE id = $1"#,
        )
        .bind(i64::try_from(id).unwrap())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(network_row_to_model))
    }

    pub async fn networks(&self) -> sqlx::Result<Vec<WithId<models::Network>>> {
        let rows: Vec<NetworkRow> = sqlx::query_as(
            r#"
SELECT (id, chain_id, latest_block_number, latest_block_hash, latest_block_delta, introduced_with)
FROM networks
ORDER BY id ASC"#,
        )
        .fetch_all(&self.pool)
        .await?;
        let networks = rows.into_iter().map(network_row_to_model).collect();
        Ok(networks)
    }

    pub async fn insert_data_edge_call(
        &self,
        call: models::DataEdgeCall,
    ) -> sqlx::Result<models::Id> {
        let row: (i64,) = sqlx::query_as(
            r#"
INSERT INTO data_edge_calls (
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
        .bind(i64::try_from(call.nonce).unwrap())
        .bind(i64::try_from(call.num_confirmations).unwrap())
        .bind(call.num_confirmations_last_checked_at)
        .bind(i64::try_from(call.block_number).unwrap())
        .bind(call.block_hash)
        .bind(call.payload)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0.try_into().unwrap())
    }

    pub async fn write_num_confirmations(
        &self,
        call_id: models::Id,
        num_confirmations: u64,
    ) -> sqlx::Result<()> {
        let call_id = i64::try_from(call_id).unwrap();
        let num_confirmations = i64::try_from(num_confirmations).unwrap();
        let now = sqlx::types::chrono::Utc::now();

        sqlx::query(
            r#"
UPDATE data_edge_calls
SET
    num_confirmations = $1,
    num_confirmations_last_checked_at = $2
WHERE id = $3
            "#,
        )
        .bind(num_confirmations)
        .bind(now)
        .bind(call_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn last_nonce(&self) -> sqlx::Result<models::Nonce> {
        let row: (i64,) = sqlx::query_as(
            r#"
SELECT nonce
FROM data_edge_calls
ORDER BY id DESC
LIMIT 1"#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0.try_into().unwrap())
    }
}

#[async_trait]
impl epoch_encoding::Database for Store {
    type Error = sqlx::Error;

    async fn get_next_nonce(&self) -> Result<u64, Self::Error> {
        self.last_nonce().await.map(|nonce| nonce as u64 + 1)
    }

    async fn set_next_nonce(&mut self, nonce: u64) -> Result<(), Self::Error> {
        let last_nonce = self.last_nonce().await?;
        assert_eq!(nonce, last_nonce + 1);
        Ok(())
    }

    async fn get_network_ids(&self) -> Result<HashMap<String, ee::NetworkId>, Self::Error> {
        let networks = self.networks().await?;
        Ok(networks
            .into_iter()
            .map(|n| (n.data.name.into_string(), n.id.try_into().unwrap()))
            .collect())
    }

    async fn set_network_ids(
        &mut self,
        ids: HashMap<String, ee::NetworkId>,
    ) -> Result<(), Self::Error> {
        for (name, id) in ids {
            let network = WithId {
                id,
                data: models::Network {
                    name: models::Caip2ChainId::from_str(name.as_str()).unwrap(),
                    introduced_with: 0,
                    latest_block_hash: None,
                    latest_block_number: None,
                    latest_block_delta: None,
                },
            };
            self.insert_network(network).await?;
        }
        Ok(())
    }

    async fn get_network(&self, id: ee::NetworkId) -> Result<Option<ee::Network>, Self::Error> {
        if let Some(network) = self.network_by_id(id).await? {
            match (
                network.data.latest_block_delta,
                network.data.latest_block_number,
            ) {
                (Some(delta), Some(num)) => Ok(Some(ee::Network {
                    block_delta: delta,
                    block_number: num,
                })),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn set_network(
        &mut self,
        id: ee::NetworkId,
        network: ee::Network,
    ) -> Result<(), Self::Error> {
        self.update_network_block_info(id, network.block_number, network.block_delta)
            .await?;
        Ok(())
    }
}
