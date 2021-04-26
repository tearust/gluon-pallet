//! RPC interface for the transaction payment module.
use gluon_runtime_api::GluonApi as GluonRuntimeApi;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
use codec::Codec;

#[allow(clippy::too_many_arguments)]

#[rpc]
pub trait GluonApi<BlockHash, AccountGenerationDataWithoutP3> {
    #[rpc(name = "gluon_getDelegates")]
    fn get_delegates(&self, start: u32, count: u32, at: Option<BlockHash>)
        -> Result<Vec<[u8; 32]>>;

    #[rpc(name = "gluon_encodeAccountGenerationWithoutP3")]
    fn encode_account_generation_without_p3(&self,
        key_ype: Vec<u8>, n: u32, k: u32, delegator_nonce_hash: Vec<u8>,
        delegator_nonce_rsa: Vec<u8>, p1: Vec<u8>, at: Option<BlockHash>) -> Result<Vec<u8>>;

    #[rpc(name = "gluon_encodeTask")]
    fn encode_task1(&self, task: AccountGenerationDataWithoutP3, at: Option<BlockHash>) -> Result<Vec<u8>>;
}

/// A struct that implements the `SumStorageApi`.
pub struct Gluon<C, M> {
    // If you have more generics, no need to SumStorage<C, M, N, P, ...>
    // just use a tuple like SumStorage<C, (M, N, P, ...)>
    client: Arc<C>,
    _marker: std::marker::PhantomData<M>,
}

impl<C, M> Gluon<C, M> {
    /// Create new `gluon` instance with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, AccountGenerationDataWithoutP3>
    GluonApi<<Block as BlockT>::Hash, AccountGenerationDataWithoutP3> for Gluon<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static,
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block>,
    C::Api: GluonRuntimeApi<Block, AccountGenerationDataWithoutP3>,
    AccountGenerationDataWithoutP3: Codec,
{
    fn get_delegates(
        &self,
        start: u32,
        count: u32,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<[u8; 32]>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

        let runtime_api_result = api.get_delegates(&at, start, count);
        runtime_api_result.map_err(|e| RpcError {
            code: ErrorCode::ServerError(9876), // No real reason for this value
            message: "Something wrong".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }

    fn encode_account_generation_without_p3(
        &self,
        key_type: Vec<u8>,
        n: u32,
        k: u32,
        delegator_nonce_hash: Vec<u8>,
        delegator_nonce_rsa: Vec<u8>,
        p1: Vec<u8>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
        let runtime_api_result = api.encode_account_generation_without_p3(
            &at, key_type, n, k, delegator_nonce_hash, delegator_nonce_rsa, p1);
        runtime_api_result.map_err(|e| RpcError {
            code: ErrorCode::ServerError(9876), // No real reason for this value
            message: "Something wrong".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }

    fn encode_task1(&self, task: AccountGenerationDataWithoutP3,
                    at: Option<<Block as BlockT>::Hash>) -> Result<Vec<u8>>  {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
        let runtime_api_result = api.encode_task1(
            &at, task);
        runtime_api_result.map_err(|e| RpcError {
            code: ErrorCode::ServerError(9876), // No real reason for this value
            message: "Something wrong".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }
}
