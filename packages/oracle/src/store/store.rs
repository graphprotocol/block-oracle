use super::models;
use models::{Caip2ChainId, WithId};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

type NetworkRow = (i32, String, Option<i64>, Option<Vec<u8>>, Option<i64>, i32);

const INITIAL_NONCE: u64 = 1;

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
            name: Caip2ChainId::from_str(chain_name.as_str()).unwrap(),
            latest_block_number: latest_block_number.map(|x| x.try_into().unwrap()),
            latest_block_hash,
            latest_block_delta: latest_block_delta.map(|x| x.try_into().unwrap()),
            introduced_with: models::Id::try_from(introduced_with).unwrap(),
        },
    }
}

#[derive(Debug, Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub async fn new(db_url: &str) -> sqlx::Result<Self> {
        let pool = SqlitePoolOptions::new().connect(db_url).await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
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
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
INSERT INTO encoding_versions (id, introduced_with)
VALUES (?1, ?2)"#,
        )
        .bind(i32::try_from(version).unwrap())
        .bind(i32::try_from(data_edge_call_id).unwrap())
        .fetch_one(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn sync_networks(&self, networks: Vec<Caip2ChainId>) -> sqlx::Result<()> {
        let networks: HashSet<Caip2ChainId> = networks.into_iter().collect();
        let current_networks: HashMap<Caip2ChainId, models::Id> = self
            .networks()
            .await?
            .into_iter()
            .map(|n| (n.data.name, n.id))
            .collect();

        let networks_to_delete: Vec<Caip2ChainId> = current_networks
            .iter()
            .filter(|n| !networks.contains(&n.0))
            .map(|x| x.0.clone())
            .collect();
        let networks_to_insert: Vec<Caip2ChainId> = networks
            .iter()
            .filter(|chain_id| !current_networks.contains_key(*chain_id))
            .cloned()
            .collect();

        Ok(())
    }

    pub async fn delete_network(&self, network_id: models::Id) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM networks WHERE id = ?1")
            .bind(i32::try_from(network_id).unwrap())
            .fetch_one(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn insert_network(
        &self,
        network: WithId<models::Network>,
    ) -> sqlx::Result<models::Id> {
        let row: (i32,) = sqlx::query_as(
            r#"
INSERT INTO networks (
    id,
    caip2_chain_id,
    latest_block_number,
    latest_block_hash,
    latest_block_delta,
    introduced_with
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6)
RETURNING id"#,
        )
        .bind(i32::try_from(network.id).unwrap())
        .bind(network.data.name.as_str())
        .bind(
            network
                .data
                .latest_block_number
                .map(|x| i64::try_from(x).unwrap()),
        )
        .bind(network.data.latest_block_hash)
        .bind(network.data.latest_block_delta)
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
SET latest_block_number = ?1, latest_block_delta = ?1 - latest_block_number
where id = ?2"#,
        )
        .bind(i32::try_from(latest_block_number).unwrap())
        .bind(i32::try_from(id).unwrap())
        .fetch_one(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_network_block_info(
        &self,
        caip2: &Caip2ChainId,
        latest_block_number: u64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
UPDATE networks
SET latest_block_number = ?1, latest_block_delta = ?1 - latest_block_number
where caip2_chain_id = ?2"#,
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
WHERE id = ?1"#,
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
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
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
    num_confirmations = ?1,
    num_confirmations_last_checked_at = ?2
WHERE id = ?3
            "#,
        )
        .bind(num_confirmations)
        .bind(now)
        .bind(call_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn block_number_of_last_epoch(&self) -> sqlx::Result<Option<u64>> {
        let row: Option<(i64,)> = sqlx::query_as(
            r#"
SELECT block_number
FROM data_edge_calls
ORDER BY id DESC
LIMIT 1
"#,
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((block_number,)) => Ok(Some(block_number.try_into().unwrap())),
            None => Ok(None),
        }
    }

    pub async fn last_nonce(&self) -> sqlx::Result<Option<models::Nonce>> {
        let row: Option<(i32,)> = sqlx::query_as(
            r#"
SELECT nonce
FROM data_edge_calls
ORDER BY id DESC
LIMIT 1"#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|x| x.0.try_into().unwrap()))
    }

    pub async fn next_nonce(&self) -> sqlx::Result<models::Nonce> {
        let initial_nonce = INITIAL_NONCE;
        Ok(self
            .last_nonce()
            .await?
            .map(|nonce| nonce + 1)
            .unwrap_or(initial_nonce))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::DataEdgeCall;
    use sqlx::{types::chrono::DateTime, Transaction};

    async fn test_store() -> Store {
        Store::new(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_connect() {
        test_store().await;
    }

    #[tokio::test]
    async fn next_nonce_empty_db() {
        let store = test_store().await;
        assert_eq!(store.next_nonce().await.unwrap(), INITIAL_NONCE);
    }

    #[tokio::test]
    async fn next_nonce_is_last_plus_one() {
        let store = test_store().await;
        store
            .insert_data_edge_call(DataEdgeCall {
                tx_hash: &[],
                nonce: 3,
                num_confirmations: 0,
                num_confirmations_last_checked_at: sqlx::types::chrono::Utc::now(),
                block_number: 0,
                block_hash: &[],
                payload: "0x0".into(),
            })
            .await
            .unwrap();
        assert_eq!(
            store.next_nonce().await.unwrap(),
            store.last_nonce().await.unwrap().unwrap() + 1
        );
    }

    #[tokio::test]
    async fn last_nonce_empty_db() {
        let store = test_store().await;
        assert_eq!(store.last_nonce().await.unwrap(), None);
    }

    #[tokio::test]
    async fn encoding_version() {
        let store = test_store().await;
        let call_id = store
            .insert_data_edge_call(DataEdgeCall {
                tx_hash: &[1],
                nonce: 3,
                num_confirmations: 0,
                num_confirmations_last_checked_at: sqlx::types::chrono::Utc::now(),
                block_number: 0,
                block_hash: &[],
                payload: "0x0".into(),
            })
            .await
            .unwrap();
        store.insert_encoding_version(4, call_id).await.unwrap();
        let encoding_version_id = store.last_encoding_version().await.unwrap().unwrap();
        assert_eq!(encoding_version_id, 4);
    }

    #[tokio::test]
    async fn last_nonce() {
        let store = test_store().await;
        store
            .insert_data_edge_call(DataEdgeCall {
                tx_hash: &[1],
                nonce: 3,
                num_confirmations: 0,
                num_confirmations_last_checked_at: sqlx::types::chrono::Utc::now(),
                block_number: 0,
                block_hash: &[],
                payload: "0x0".into(),
            })
            .await
            .unwrap();
        store
            .insert_data_edge_call(DataEdgeCall {
                tx_hash: &[],
                nonce: 1,
                num_confirmations: 0,
                num_confirmations_last_checked_at: sqlx::types::chrono::Utc::now(),
                block_number: 0,
                block_hash: &[1],
                payload: "0x1".into(),
            })
            .await
            .unwrap();
        assert_eq!(store.last_nonce().await.unwrap(), Some(1));
    }

    #[tokio::test]
    async fn test_insert_network() {
        let store = test_store().await;
        let call_id = store
            .insert_data_edge_call(DataEdgeCall {
                tx_hash: &[],
                nonce: 42,
                num_confirmations: 0,
                num_confirmations_last_checked_at: sqlx::types::chrono::Utc::now(),
                block_number: 0,
                block_hash: &[],
                payload: "0x0".into(),
            })
            .await
            .unwrap();

        let network_id = store
            .insert_network(WithId {
                id: 1,
                data: models::Network {
                    name: Caip2ChainId::ethereum_mainnet(),
                    introduced_with: call_id,
                    latest_block_hash: None,
                    latest_block_number: Some(1337),
                    latest_block_delta: None,
                },
            })
            .await
            .unwrap();
        assert_eq!(network_id, 1);

        let network = store.network_by_id(network_id).await.unwrap().unwrap();
        assert_eq!(network.id, 1);
        assert_eq!(network.data.latest_block_number, Some(1337));
        assert_eq!(network.data.name, Caip2ChainId::ethereum_mainnet());
    }
}
