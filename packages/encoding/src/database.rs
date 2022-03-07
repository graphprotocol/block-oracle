use async_trait::async_trait;
use std::collections::HashMap;
use std::future::Future;

use crate::NetworkId;

#[async_trait]
pub trait Connection {
    type Database: Database;
    async fn transaction<'a, F, T, Fut>(
        &'a self,
        f: F,
    ) -> Result<T, <<Self as Connection>::Database as Database>::Error>
    where
        F: Send + FnOnce(&'a mut Self::Database) -> Fut,
        Fut: 'a
            + Send
            + Future<Output = Result<T, <<Self as Connection>::Database as Database>::Error>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Network {
    pub block_number: u64,
    pub block_delta: i64,
}

#[async_trait]
pub trait Database {
    type Error;
    async fn get_next_nonce(&self) -> Result<u64, Self::Error>;
    async fn set_next_nonce(&mut self, nonce: u64) -> Result<(), Self::Error>;
    async fn get_network_ids(&self) -> Result<HashMap<String, NetworkId>, Self::Error>;
    async fn get_network(&self, id: NetworkId) -> Result<Option<Network>, Self::Error>;
    async fn set_network(&mut self, id: NetworkId, network: Network) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod mocks {
    use {super::*, never::Never, std::sync::Arc, tokio::sync::Mutex};

    pub struct MockConnection {
        db: Arc<Mutex<MockDB>>,
    }

    #[async_trait]
    impl Connection for MockConnection {
        type Database = MockDB;

        async fn transaction<'a, F, T, Fut>(
            &'a self,
            f: F,
        ) -> Result<T, <<Self as Connection>::Database as Database>::Error>
        where
            F: Send + FnOnce(&'a mut Self::Database) -> Fut,
            Fut: 'a
                + Send
                + Future<Output = Result<T, <<Self as Connection>::Database as Database>::Error>>,
        {
            let mut db = self.db.lock().await;
            f(&mut db).await
        }
    }

    impl MockConnection {
        pub fn new() -> Self {
            Self {
                db: Arc::new(Mutex::new(MockDB::new())),
            }
        }
    }

    pub struct MockDB {
        next_nonce: u64,
        network_ids: HashMap<String, NetworkId>,
        networks: HashMap<NetworkId, Network>,
    }

    impl MockDB {
        pub fn new() -> Self {
            Self {
                next_nonce: 0,
                network_ids: HashMap::new(),
                networks: HashMap::new(),
            }
        }
    }

    #[async_trait]
    impl Database for MockDB {
        type Error = Never;
        async fn get_next_nonce(&self) -> Result<u64, Self::Error> {
            Ok(self.next_nonce)
        }
        async fn set_next_nonce(&mut self, nonce: u64) -> Result<(), Self::Error> {
            self.next_nonce = nonce;
            Ok(())
        }
        async fn get_network_ids(&self) -> Result<HashMap<String, NetworkId>, Self::Error> {
            Ok(self.network_ids.clone())
        }
        async fn get_network(&self, id: NetworkId) -> Result<Option<Network>, Self::Error> {
            Ok(self.networks.get(&id).cloned())
        }
        async fn set_network(
            &mut self,
            id: NetworkId,
            network: Network,
        ) -> Result<(), Self::Error> {
            self.networks.insert(id, network);
            Ok(())
        }
    }
}
