use async_trait::async_trait;
use std::collections::HashMap;
use std::future::Future;

use crate::NetworkId;

#[async_trait]
pub trait Connection {
    type Database: Database;
    async fn transaction<F, T, Fut>(
        &self,
        f: F,
    ) -> Result<T, <<Self as Connection>::Database as Database>::Error>
    where
        F: Send + FnOnce(Self::Database) -> Fut,
        Fut: Send + Future<Output = Result<T, <<Self as Connection>::Database as Database>::Error>>;
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
    async fn set_network_ids(&mut self, ids: HashMap<String, NetworkId>)
        -> Result<(), Self::Error>;
    async fn get_network(&self, id: NetworkId) -> Result<Option<Network>, Self::Error>;
    async fn set_network(&mut self, id: NetworkId, network: Network) -> Result<(), Self::Error>;
}

#[cfg(test)]
pub(crate) mod mocks {

    use {super::*, never::Never, std::ops::Deref, std::sync::Arc, tokio::sync::Mutex};

    pub struct MockConnection {
        db: Arc<Mutex<MockDBState>>,
    }

    pub struct MockDB {
        orig: Arc<Mutex<MockDBState>>,
        new: MockDBState,
    }

    #[async_trait]
    impl Connection for MockConnection {
        type Database = MockDB;

        async fn transaction<F, T, Fut>(
            &self,
            f: F,
        ) -> Result<T, <<Self as Connection>::Database as Database>::Error>
        where
            F: Send + FnOnce(Self::Database) -> Fut,
            Fut: Send
                + Future<Output = Result<T, <<Self as Connection>::Database as Database>::Error>>,
        {
            let db = self.db.lock().await;
            let db = MockDB {
                orig: self.db.clone(),
                new: (*db.deref()).clone(),
            };

            // TODO: FIXME! Commit the result to the actual db if the function completed successfully.
            // This turns out to be annoying. For one, a nested Result is not visible in this method.
            // This means that a commit could happen even if validation failed, which we do not want.
            // Furthermore, getting the lifetime correct for the MutexGuard is hard (or maybe impossible
            // without GAT). Also, we can't seem to send &mut MockDB here because of the lifetimes.
            // So we don't have a reference to the data to commit. Achievable, but annoying. It may be
            // best to modify the trait when an actual database is available.
            f(db).await
        }
    }

    impl MockConnection {
        pub fn new() -> Self {
            Self {
                db: Arc::new(Mutex::new(MockDBState::new())),
            }
        }
    }

    #[derive(Clone)]
    pub struct MockDBState {
        next_nonce: u64,
        network_ids: HashMap<String, NetworkId>,
        networks: HashMap<NetworkId, Network>,
    }

    impl MockDBState {
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
            Ok(self.new.next_nonce)
        }
        async fn set_next_nonce(&mut self, nonce: u64) -> Result<(), Self::Error> {
            self.new.next_nonce = nonce;
            Ok(())
        }
        async fn get_network_ids(&self) -> Result<HashMap<String, NetworkId>, Self::Error> {
            Ok(self.new.network_ids.clone())
        }
        async fn set_network_ids(
            &mut self,
            ids: HashMap<String, NetworkId>,
        ) -> Result<(), Self::Error> {
            self.new.network_ids = ids;
            Ok(())
        }
        async fn get_network(&self, id: NetworkId) -> Result<Option<Network>, Self::Error> {
            Ok(self.new.networks.get(&id).cloned())
        }
        async fn set_network(
            &mut self,
            id: NetworkId,
            network: Network,
        ) -> Result<(), Self::Error> {
            self.new.networks.insert(id, network);
            Ok(())
        }
    }
}
