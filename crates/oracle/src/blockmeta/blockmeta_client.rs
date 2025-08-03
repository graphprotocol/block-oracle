//! StreamingFast Blockmeta gRPC client.

use std::collections::BTreeMap;
use std::time::Duration;

use prost::bytes::Bytes;

use futures::stream::{FuturesUnordered, StreamExt};

use tonic::codegen::{Body, InterceptedService, StdError};
use tonic::transport::{Channel, Uri};

pub use self::auth::AuthInterceptor;
use self::gen::block_client::BlockClient;
pub use self::gen::BlockResp as Block;
use self::gen::Empty;
pub use self::gen::{BlockResp, NumToIdReq};
use crate::{BlockmetaProviderForChain, Caip2ChainId};

/// This file is **generated** by the `build.rs` when compiling the crate with the `proto-gen`
/// feature enabled. The `build.rs` script uses the `tonic-build` crate to generate the files.
///
/// ```shell
/// cargo build --features proto-gen --bin block-oracle
/// ```
mod gen {
    include!("sf_blockmeta_client/sf.blockmeta.v2.rs");
}

mod auth {
    use tonic::{Request, Status};

    /// The `AuthInterceptor` is a gRPC interceptor that adds an `authorization` header to the request
    /// metadata.
    ///
    /// This middleware inserts the `authorization` header into the request metadata. The header is
    /// expected to be in the format `Bearer <token>`.
    ///
    /// It is used to authenticate requests to the StreamingFast Blockmeta service.
    #[derive(Clone)]
    pub struct AuthInterceptor {
        header_value: String,
    }

    impl AuthInterceptor {
        /// Create a new `AuthInterceptor` with the given authorization token.
        pub(super) fn with_token(token: &str) -> Self {
            Self {
                header_value: format!("bearer {token}"),
            }
        }
    }

    impl tonic::service::Interceptor for AuthInterceptor {
        fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
            // The `authorization` header is expected to be in the format `Bearer <token>`
            let auth = self.header_value.parse().map_err(|err| {
                Status::new(
                    tonic::Code::Unauthenticated,
                    format!("invalid authorization token: {err}"),
                )
            })?;

            // Insert the `authorization` header into the request metadata
            request.metadata_mut().insert("authorization", auth);
            Ok(request)
        }
    }
}

/// StreamingFast Blockmeta gRPC client.
///
/// The `BlockmetaClient` is a gRPC client for the StreamingFast Blockmeta service. It provides
/// method to fetch the latest block.
#[derive(Debug, Clone)]
pub struct BlockmetaClient<T> {
    grpc_client: BlockClient<T>,
}

impl BlockmetaClient<Channel> {
    /// Create a new `BlockmetaClient` with the given gRPC endpoint.
    ///
    /// The service will connect once the first request is made. It will attempt to connect for
    /// 5 seconds before timing out.
    pub fn new(endpoint: Uri) -> Self {
        let channel = Channel::builder(endpoint)
            .tls_config(Default::default())
            .expect("failed to configure TLS")
            .connect_timeout(Duration::from_secs(5))
            .connect_lazy();
        Self {
            grpc_client: BlockClient::new(channel),
        }
    }
}

impl BlockmetaClient<InterceptedService<Channel, AuthInterceptor>> {
    /// Create a new `BlockmetaClient` with the given gRPC endpoint and authorization token.
    ///
    /// The cliient will connect to the given endpoint and authenticate requests with the given
    /// authorization token inserted into the `authorization` header by the [`AuthInterceptor`].
    ///
    /// The service will connect once the first request is made. It will attempt to connect for
    /// 5 seconds before timing out.
    pub fn new_with_auth(endpoint: Uri, auth: impl AsRef<str>) -> Self {
        let interceptor = AuthInterceptor::with_token(auth.as_ref());
        let channel = Channel::builder(endpoint)
            .tls_config(Default::default())
            .expect("failed to configure TLS")
            .connect_timeout(Duration::from_secs(5))
            .connect_lazy();

        Self {
            grpc_client: BlockClient::with_interceptor(channel, interceptor),
        }
    }
}

impl<T> BlockmetaClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<StdError>,
    T::ResponseBody: Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as Body>::Error: Into<StdError> + Send,
{
    /// Fetch the latest block from the StreamingFast Blockmeta service.
    ///
    /// Returns `None` if the block does not exist.
    pub async fn get_latest_block(&mut self) -> anyhow::Result<Option<Block>> {
        let request = Empty {};

        match self.grpc_client.head(request).await {
            Ok(res) => Ok(Some(res.into_inner())),
            Err(err) if err.code() == tonic::Code::NotFound => Ok(None),
            Err(err) => Err(anyhow::anyhow!("request failed: {}", err.message())),
        }
    }

    /// Fetch a block by its number from the StreamingFast Blockmeta service.
    ///
    /// Returns `None` if the block does not exist.
    pub async fn num_to_id(&mut self, request: NumToIdReq) -> anyhow::Result<BlockResp> {
        match self.grpc_client.num_to_id(request).await {
            Ok(res) => Ok(res.into_inner()),
            Err(err) => Err(anyhow::anyhow!("request failed: {}", err.message())),
        }
    }
}

/// Fetches the latest available block number and hash from all `chains`.
pub async fn get_latest_blockmeta_blocks<T>(
    chains: &[BlockmetaProviderForChain<T>],
) -> BTreeMap<Caip2ChainId, anyhow::Result<Block>>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<StdError>,
    T::ResponseBody: Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    T: std::clone::Clone,
{
    let mut tasks = chains
        .iter()
        .cloned()
        .map(|mut chain| async move {
            chain.client.get_latest_block().await.map(|block| {
                (
                    chain.chain_id,
                    block.ok_or_else(|| anyhow::anyhow!("Block not found")),
                )
            })
        })
        .collect::<FuturesUnordered<_>>();

    let mut block_ptr_per_chain = BTreeMap::new();
    while let Some(result) = tasks.next().await {
        match result {
            Ok((chain_id, block)) => {
                block_ptr_per_chain.insert(chain_id, block);
            }
            Err(e) => {
                println!("Error: {e:?}");
            }
        }
    }

    assert!(block_ptr_per_chain.len() == chains.len());
    block_ptr_per_chain
}
