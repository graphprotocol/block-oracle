use super::models;
use async_trait::async_trait;
use epoch_encoding as ee;
use models::WithId;
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashMap, str::FromStr};

type PgPool = sqlx::Pool<sqlx::Postgres>;
type NetworkRow = (i32, String, Option<i64>, Option<Vec<u8>>, Option<i64>, i32);

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

    #[cfg(test)]
    pub async fn new_clean(db_url: &str) -> sqlx::Result<Self> {
        let store = Self::new(db_url).await?;
        sqlx::query("DELETE FROM data_edge_calls;")
            .execute(&store.pool)
            .await?;
        Ok(store)
    }

    pub async fn last_encoding_version(&self) -> sqlx::Result<Option<models::Id>> {
        let row: Option<(i32,)> = sqlx::query_as(
            r#"
SELECT id,
FROM encoding_versions
ORDER BY id DESC
LIMIT 1"#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| models::Id::try_from(r.0).unwrap()))
    }

    pub async fn insert_encoding_version(
        &self,
        version: models::Id,
        data_edge_call_id: models::Id,
    ) -> sqlx::Result<models::Id> {
        let row: (i32,) = sqlx::query_as(
            r#"
INSERT INTO encoding_versions (caip2_chain_id, introduced_with)
VALUES ($1, $2)
RETURNING id"#,
        )
        .bind(i32::try_from(version).unwrap())
        .bind(i32::try_from(data_edge_call_id).unwrap())
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0.try_into().unwrap())
    }

    pub async fn insert_network(
        &self,
        network: WithId<models::Network>,
    ) -> sqlx::Result<models::Id> {
        let row: (i32,) = sqlx::query_as(
            r#"
INSERT INTO networks (id, caip2_chain_id, introduced_with)
VALUES ($1, $2, $3)
RETURNING id"#,
        )
        .bind(i32::try_from(network.id).unwrap())
        .bind(network.data.name.as_str())
        .bind(i32::try_from(network.data.introduced_with).unwrap())
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0.try_into().unwrap())
    }

    pub async fn update_network_block_info_by_id(
        &self,
        id: models::Id,
        latest_block_number: u64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
UPDATE networks
SET latest_block_number = $1, latest_block_delta = $1 - latest_block_number
where id = $2"#,
        )
        .bind(i32::try_from(latest_block_number).unwrap())
        .bind(i32::try_from(id).unwrap())
        .fetch_one(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_network_block_info(
        &self,
        caip2: &models::Caip2ChainId,
        latest_block_number: u64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
UPDATE networks
SET latest_block_number = $1, latest_block_delta = $1 - latest_block_number
where caip2_chain_id = $2"#,
        )
        .bind(i32::try_from(latest_block_number).unwrap())
        .bind(caip2.as_str())
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
SELECT id, caip2_chain_id, latest_block_number, latest_block_hash, latest_block_delta, introduced_with
FROM networks
WHERE id = $1"#,
        )
        .bind(i32::try_from(id).unwrap())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(network_row_to_model))
    }

    pub async fn networks(&self) -> sqlx::Result<Vec<WithId<models::Network>>> {
        let rows: Vec<NetworkRow> = sqlx::query_as(
            r#"
SELECT id, caip2_chain_id, latest_block_number, latest_block_hash, latest_block_delta, introduced_with
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
        let row: (i32,) = sqlx::query_as(
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
        .bind(i32::try_from(call.nonce).unwrap())
        .bind(i32::try_from(call.num_confirmations).unwrap())
        .bind(call.num_confirmations_last_checked_at)
        .bind(i32::try_from(call.block_number).unwrap())
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
        let call_id = i32::try_from(call_id).unwrap();
        let num_confirmations = i32::try_from(num_confirmations).unwrap();
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
        let row: (i32,) = sqlx::query_as(
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
                id: id as models::Id,
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
        if let Some(network) = self.network_by_id(id as models::Id).await? {
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
        self.update_network_block_info_by_id(id as models::Id, network.block_number)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::DataEdgeCall;
    use sqlx::{types::chrono::DateTime, Postgres, Transaction};

    async fn test_store() -> Store {
        let db_url = std::env::var("BLOCK_ORACLE_TEST_DATABASE_URL").unwrap();
        Store::new_clean(&db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_connect() {
        test_store().await;
    }

    #[tokio::test]
    async fn test_insert_network() {
        let store = test_store().await;
        let call_id = store
            .insert_data_edge_call(DataEdgeCall {
                tx_hash: "0x0".into(),
                nonce: 0,
                num_confirmations: 0,
                num_confirmations_last_checked_at: sqlx::types::chrono::Utc::now(),
                block_number: 0,
                block_hash: "0x0".into(),
                payload: "0x0".into(),
            })
            .await
            .unwrap();
        store
            .insert_network(WithId {
                id: 1,
                data: models::Network {
                    name: models::Caip2ChainId::ethereum_mainnet(),
                    introduced_with: call_id,
                    latest_block_hash: None,
                    latest_block_number: Some(1337),
                    latest_block_delta: None,
                },
            })
            .await
            .unwrap();
        assert_eq!(
            store
                .network_by_id(1)
                .await
                .unwrap()
                .unwrap()
                .data
                .latest_block_number,
            Some(1337)
        );
    }
}
