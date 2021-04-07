#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_mut_passed)]

use sp_std::prelude::*;
// use codec::{Codec, Decode, Encode};
use codec::Codec;

// Here we declare the runtime API. It is implemented it the `impl` block in
// runtime amalgamator file (the `runtime/src/lib.rs`)
sp_api::decl_runtime_apis! {
    pub trait GluonApi<AccountGenerationDataWithoutP3> where
        AccountGenerationDataWithoutP3: Codec,
     {
        fn get_delegates(start: u32, count: u32) -> Vec<[u8; 32]>;

        fn encode_account_generation_without_p3(
            key_type: Vec<u8>, n: u32, k: u32, delegator_nonce_hash: Vec<u8>,
            delegator_nonce_rsa: Vec<u8>, p1: Vec<u8>) -> Vec<u8>;

        fn encode_task1(task: AccountGenerationDataWithoutP3) -> Vec<u8>;
    }
}
