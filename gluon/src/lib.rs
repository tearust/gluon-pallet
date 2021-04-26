#![cfg_attr(not(feature = "std"), no_std)]

#[allow(dead_code)]

use codec::{Decode, Encode};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch, ensure,
    traits::{Currency, ExistenceRequirement},
    StorageMap, StorageValue,
};
/// A FRAME pallet template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references

/// For more guidance on Substrate FRAME, see the example pallet
/// https://github.com/paritytech/substrate/blob/master/frame/example/src/lib.rs
use frame_system::ensure_signed;
use pallet_balances as balances;
use sha2::{Digest, Sha256};
// use sp_core::sr25519;

use sp_runtime::{
	// DispatchError, ModuleId,
    // traits::{AccountIdConversion, Saturating, Zero},
    traits::Saturating,
};

use sp_std::prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// The pallet's configuration trait.
pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type Currency: Currency<Self::AccountId>;
}

pub type TeaPubKey = [u8; 32];

pub type Url = Vec<u8>;

pub type Cid = Vec<u8>;

pub type TxData = Vec<u8>;

pub type Signature = Vec<u8>;

pub type KeyType = Vec<u8>;

pub type ClientPubKey = Vec<u8>;

pub type MultiSigAccount = Vec<u8>;

const RUNTIME_ACTIVITY_THRESHOLD: u32 = 3600;
// const MIN_TRANSFER_ASSET_SIGNATURE_COUNT: usize = 2;
// const MAX_TRANSFER_ASSET_TASK_PERIOD: u32 = 100;
const TASK_TIMEOUT_PERIOD: u32 = 30;

// type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct AccountAssetData {
    pub btc: Vec<Cid>,
    pub eth: Vec<Cid>,
    pub dot: Vec<Cid>,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct TransferAssetTask<BlockNumber> {
    pub from: Cid,
    pub to: Cid,
    pub start_height: BlockNumber,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct AccountGenerationDataWithoutP3 {
    /// the key type: btc or eth.
    pub key_type: Cid,
    /// split the secret to `n` pieces
    pub n: u32,
    /// if have k (k < n) pieces the secret can be recovered
    pub k: u32,
    /// the nonce hash of delegator
    pub delegator_nonce_hash: Cid,
    /// encrypted nonce using delegator pubic key
    pub delegator_nonce_rsa: Cid,
    /// p1 public key
    pub p1: Cid,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct Asset<AccountId> {
    pub owner: AccountId,
    pub p2: Cid,
    pub deployment_ids: Vec<Cid>,
    pub web: AccountId,
    pub app: AccountId,
    pub multi_sig_account: MultiSigAccount,
    pub data_adhoc: AccountGenerationDataWithoutP3,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct SignTransactionData {
    /// the transaction to be signed
    pub data_adhoc: TxData,
    /// the hash of nonce
    pub delegator_nonce_hash: Cid,
    /// encrypted nonce using delegator pubic key
    pub delegator_nonce_rsa: Cid,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct SignTransactionTask {
    /// the task hash
    pub task_id: Cid,
    /// the address of multisig account
    pub multisig_address: Cid,
    /// the signature of p1
    pub p1_signature: TxData,
    /// the information of task
    pub task_data: SignTransactionData,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct SignTransactionResult {
    pub task_id: Cid,
    pub succeed: bool,
}

decl_storage! {
    trait Store for Module<T: Trait> as GluonModule {
        TransferAssetTasks get(fn transfer_asset_tasks):
            map hasher(twox_64_concat) Cid => TransferAssetTask<T::BlockNumber>;

        TransferAssetSignatures get(fn transfer_asset_signatures):
            map hasher(twox_64_concat) Cid => Vec<T::AccountId>;

        AccountAssets get(fn account_assets):
            map hasher(twox_64_concat) T::AccountId => AccountAssetData;

        // Register Gluon wallet account.
        // Temporary storage
        BrowserNonce get(fn browser_nonce):
            map hasher(blake2_128_concat) T::AccountId => (T::BlockNumber, Cid);
        // Permanent storage
        BrowserAppPair get(fn browser_app_pair):
            map hasher(blake2_128_concat) T::AccountId => (T::AccountId, Cid);
        AppBrowserPair get(fn app_browser_pair):
            map hasher(blake2_128_concat) T::AccountId => (T::AccountId, Cid);

        // Generate BTC 2/3 MultiSig Account
        // Temporary storage
        BrowserAccountNonce get(fn browser_account_nonce):
            map hasher(blake2_128_concat) T::AccountId => (T::BlockNumber, Cid, Cid); // value: (height, nonce hash, task hash)
        AccountGenerationTasks get(fn account_generation_tasks):
                    map hasher(blake2_128_concat) Cid => AccountGenerationDataWithoutP3; // key: task_id value: task
        // Intermediate storage to wait for task result
        AccountGenerationTaskDelegator get(fn account_generation_task_delegator):
            map hasher(blake2_128_concat) Cid => (Cid, T::AccountId); // key: task_id, value: (delegator_nonce_hash, browser)
        // Permanent storage
        BrowserMultiSigAccounts get(fn browser_multi_sig_account):
            map hasher(blake2_128_concat) T::AccountId => Vec<MultiSigAccount>;

        Assets get(fn assets):
            map hasher(blake2_128_concat) MultiSigAccount => Asset<T::AccountId>; // key: multiSigAccount value


        // Sign transaction
        // Temporary storage
        SignTransactionTasks get(fn sign_transaction_tasks):
            map hasher(blake2_128_concat) Cid => SignTransactionData;
        SignTransactionTaskSender get(fn sign_transaction_sender):
            map hasher(blake2_128_concat) Cid => (T::BlockNumber, T::AccountId);
        // Permanent storage
        SignTransactionResults get(fn sign_transaction_results):
            map hasher(blake2_128_concat) Cid => bool;

        Delegates get(fn delegates): Vec<(TeaPubKey, T::BlockNumber)>;
        // if need to use nonce to get a fixed delegator.
        // DelegatesIndex get(fn delegates_index):
        //     map hasher(blake2_128_concat) u64 => (TeaPubKey, T::BlockNumber);
    }

    add_extra_genesis {
        build(|_config: &GenesisConfig| {
        })
    }
}

impl<T: Trait> Module<T> {
    pub fn get_delegates(start: u32, count: u32) -> Vec<[u8; 32]> {
        let delegates = Delegates::<T>::get();
        let current_block_number = <frame_system::Module<T>>::block_number();
        let mut result: Vec<[u8; 32]> = vec![];
        let mut index: u32 = 0;
        for (d, b) in delegates {
            if current_block_number - b < RUNTIME_ACTIVITY_THRESHOLD.into() {
                index += 1;
                if index > start {
                    result.push(d);
                    let size = result.len();
                    let delegates_count = count as usize;
                    if size == delegates_count {
                        break;
                    }
                }
            }
        }
        result
    }

    pub fn encode_account_generation_without_p3(
        key_type: Vec<u8>,
        n: u32,
        k: u32,
        delegator_nonce_hash: Vec<u8>,
        delegator_nonce_rsa: Vec<u8>,
        p1: Vec<u8>,
    ) -> Vec<u8> {
        let task = AccountGenerationDataWithoutP3 {
            key_type: key_type,
            n: n,
            k: k,
            delegator_nonce_hash: delegator_nonce_hash,
            delegator_nonce_rsa: delegator_nonce_rsa,
            p1: p1,
        };
        let task_data = task.encode();

        let task_hash = Self::sha2_256(&task_data);
        let task_hash_str = hex::encode(task_hash);
        debug::info!("task_data:{:?}", task_data);
        debug::info!("task_hash_str:{}", task_hash_str);

        task_data.to_vec()
    }

    pub fn encode_task1(task: AccountGenerationDataWithoutP3) -> Vec<u8> {
        let task_data = task.encode();
        task_data.to_vec()
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        BlockNumber = <T as frame_system::Trait>::BlockNumber,
    {
        TransferAssetBegin(Cid, TransferAssetTask<BlockNumber>),
        TransferAssetSign(Cid, AccountId),
        TransferAssetEnd(Cid, TransferAssetTask<BlockNumber>),
        BrowserSendNonce(AccountId, Cid),
        RegistrationApplicationSucceed(AccountId, AccountId),
        BrowserAccountGeneration(AccountId, Cid, Cid),
        AccountGenerationRequested(AccountId, Cid, AccountGenerationDataWithoutP3),
        AssetGenerated(Cid, MultiSigAccount, Asset<AccountId>),
        BrowserSignTransactionRequested(AccountId, Cid, SignTransactionData),
        SignTransactionRequested(AccountId, SignTransactionTask),
        UpdateSignTransaction(Cid, bool),
        AppBrowserUnpaired(AccountId, AccountId),
    }
);



// The pallet's errors
decl_error! {
    pub enum Error for Module<T: Trait> {
        InvalidSig,
        InvalidNonceSig,
        InvalidSignatureLength,
        DelegatorNotExist,
        AccountIdConvertionError,
        InvalidToAccount,
        SenderIsNotBuildInAccount,
        SenderAlreadySigned,
        TransferAssetTaskTimeout,
        BrowserTaskALreadyExist,
        BrowserNonceAlreadyExist,
        AppBrowserPairAlreadyExist,
        NonceNotMatch,
        NonceNotExist,
        TaskNotMatch,
        TaskNotExist,
        KeyGenerationSenderAlreadyExist,
        KeyGenerationSenderNotExist,
        KeyGenerationTaskAlreadyExist,
        KeyGenerationResultExist,
        SignTransactionTaskAlreadyExist,
        SignTransactionResultExist,
        AccountGenerationTaskAlreadyExist,
        AssetAlreadyExist,
        AssetNotExist,
        InvalidAssetOwner,
        AppBrowserNotPair,
        AppBrowserPairNotExist,
        TaskTimeout,
        PairNotExist,
        InvalidKeyTypeForAccountAsset,
    }
}

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing errors
        // this includes information about your errors in the node's metadata.
        // it is needed only if you are using errors in your pallet
        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your pallet
        fn deposit_event() = default;

        #[weight = 100]
        pub fn browser_send_nonce(
            origin,
            nonce_hash: Cid,
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            // ensure!(!BrowserNonce::<T>::contains_key(&sender), Error::<T>::BrowserNonceAlreadyExist);
            ensure!(!BrowserAppPair::<T>::contains_key(&sender), Error::<T>::AppBrowserPairAlreadyExist);

             // insert into BrowserNonce and fire an event
             let current_block_number = <frame_system::Module<T>>::block_number();
             BrowserNonce::<T>::insert(sender.clone(), (current_block_number, nonce_hash.clone()));
             Self::deposit_event(RawEvent::BrowserSendNonce(sender, nonce_hash));

            Ok(())
        }

        #[weight = 100]
        pub fn send_registration_application(
            origin,
            nonce: Cid,
            browser_pk: ClientPubKey,
            metadata: Cid,
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(!AppBrowserPair::<T>::contains_key(&sender), Error::<T>::AppBrowserPairAlreadyExist);

            // check browser is not paired.
            let browser_account;
            let browser = Self::bytes_to_account(&browser_pk.as_slice());
            match browser {
                Ok(p) => {
                    browser_account = p;
                }
                Err(_e) => {
                    debug::info!("failed to parse browser pubKey");
                    return Err(Error::<T>::AccountIdConvertionError.into());
                }
            }
            ensure!(!BrowserAppPair::<T>::contains_key(&browser_account), Error::<T>::AppBrowserPairAlreadyExist);
            ensure!(BrowserNonce::<T>::contains_key(&browser_account), Error::<T>::NonceNotExist);
            // todo check task hash

            let app_nonce_hash = Self::sha2_256(&nonce.as_slice());
            let (block_number, browser_nonce_hash) = BrowserNonce::<T>::get(&browser_account);
            let current_block_number = <frame_system::Module<T>>::block_number();
            if current_block_number - block_number > TASK_TIMEOUT_PERIOD.into() {
                BrowserNonce::<T>::remove(&browser_account);
                debug::info!("browser task timeout");
                return Err(Error::<T>::TaskTimeout.into());
            }
            ensure!(browser_nonce_hash == app_nonce_hash, Error::<T>::NonceNotMatch);

            // pair succeed, record it into BrowserAppPair and AppBrowserPair and
            // remove data from AppRegistration and BrowserNonce.
            BrowserAppPair::<T>::insert(browser_account.clone(), (sender.clone(), metadata.clone()));
            AppBrowserPair::<T>::insert(sender.clone(), (browser_account.clone(), metadata));
            BrowserNonce::<T>::remove(browser_account.clone());

            // pair finished and fire an event
            Self::deposit_event(RawEvent::RegistrationApplicationSucceed(sender, browser_account));

            Ok(())
        }

        #[weight = 100]
        pub fn unpair_app_browser(
            origin,
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            if BrowserAppPair::<T>::contains_key(&sender) {
                let (app, _metadata) = BrowserAppPair::<T>::get(&sender);
                BrowserAppPair::<T>::remove(&sender);
                AppBrowserPair::<T>::remove(&app);
                Self::deposit_event(RawEvent::AppBrowserUnpaired(app, sender));
            } else if AppBrowserPair::<T>::contains_key(&sender) {
                let (browser, _metadata) = AppBrowserPair::<T>::get(&sender);
                AppBrowserPair::<T>::remove(&sender);
                BrowserAppPair::<T>::remove(&browser);
                Self::deposit_event(RawEvent::AppBrowserUnpaired(sender, browser));
            } else {
                debug::info!("failed to unpair");
                return Err(Error::<T>::PairNotExist.into());
            }

            Ok(())
        }

        #[weight = 100]
        pub fn browser_generate_account(
            origin,
            nonce_hash: Cid,
            task_hash: Cid,
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(!AccountGenerationTaskDelegator::<T>::contains_key(&task_hash), Error::<T>::BrowserTaskALreadyExist);
            // ensure!(!BrowserAccountNonce::<T>::contains_key(&sender), Error::<T>::BrowserNonceAlreadyExist);

            // insert into BrowserNonce and fire an event
            let current_block_number = <frame_system::Module<T>>::block_number();
            BrowserAccountNonce::<T>::insert(sender.clone(), (current_block_number, nonce_hash.clone(), task_hash.clone()));
            Self::deposit_event(RawEvent::BrowserAccountGeneration(sender, nonce_hash, task_hash));

            Ok(())
        }

        #[weight = 100]
        pub fn generate_account_without_p3 (
           origin,
           nonce: Cid,
           delegator_nonce_hash: Cid,
           delegator_nonce_rsa: Cid,
           key_type: Cid,
           p1: Cid,
           p2_n: u32,
           p2_k: u32,
           browser_pk: ClientPubKey,
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;

            // check browser is not paired.
            let browser_account;
            let browser = Self::bytes_to_account(&browser_pk.as_slice());
            match browser {
                Ok(p) => {
                    browser_account = p;
                }
                Err(_e) => {
                    debug::info!("failed to parse browser pubKey");
                    return Err(Error::<T>::AccountIdConvertionError.into());
                }
            }
            ensure!(BrowserAccountNonce::<T>::contains_key(&browser_account), Error::<T>::NonceNotExist);

            // check task hash
            let task = AccountGenerationDataWithoutP3 {
                key_type: key_type,
                n: p2_n,
                k: p2_k,
                delegator_nonce_hash: delegator_nonce_hash.clone(),
                delegator_nonce_rsa: delegator_nonce_rsa,
                p1: p1,
            };
            let app_nonce_hash = Self::sha2_256(&nonce.as_slice());
            let (block_number, browser_nonce_hash, browser_task_hash) = BrowserAccountNonce::<T>::get(&browser_account);
            ensure!(browser_nonce_hash == app_nonce_hash, Error::<T>::NonceNotMatch);
            let task_data = task.encode();
            let task_hash = Self::sha2_256(&task_data);
            let task_hash_str = hex::encode(&task_hash);
            let browser_task_hash_str = hex::encode(&browser_task_hash);
            debug::info!("task_data:{:?}", task_data);
            debug::info!("browser task_hash_str:{}", browser_task_hash_str);
            debug::info!("app task_hash_str:{}", task_hash_str);
            ensure!(browser_task_hash == task_hash, Error::<T>::TaskNotMatch);

            let current_block_number = <frame_system::Module<T>>::block_number();
            if current_block_number - block_number > TASK_TIMEOUT_PERIOD.into() {
                BrowserAccountNonce::<T>::remove(&browser_account);
                debug::info!("browser task timeout");
                return Err(Error::<T>::TaskTimeout.into());
            }

            // account generation requested and fire an event
            AccountGenerationTaskDelegator::<T>::insert(browser_task_hash.clone(), (delegator_nonce_hash, browser_account.clone()));
            AccountGenerationTasks::insert(browser_task_hash.clone(), task.clone());
            BrowserAccountNonce::<T>::remove(&browser_account);
            Self::deposit_event(RawEvent::AccountGenerationRequested(sender, browser_task_hash, task));

            Ok(())
        }

        #[weight = 100]
        pub fn update_generate_account_without_p3_result(
            origin,
            task_id: Cid,
            delegator_nonce: Cid,
            p2: Cid,
            p2_deployment_ids: Vec<Cid>,
            multi_sig_account: MultiSigAccount,
        )-> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;

            if !AccountGenerationTaskDelegator::<T>::contains_key(&task_id) {
                // todo remove this branch: temp for mock test
                // ensure!(AccountGenerationTaskDelegator::contains_key(&task_id), Error::<T>::TaskNotExist);
                ensure!(!Assets::<T>::contains_key(&multi_sig_account), Error::<T>::AssetAlreadyExist);
                // let mut delegator = [0u8; 32];
                // let public_key_bytes = Self::account_to_bytes(&sender);
                // match public_key_bytes {
                //     Ok(p) => {
                //         delegator = p;
                //     }
                //     Err(_e) => {
                //         debug::info!("failed to parse account");
                //         Err(Error::<T>::AccountIdConvertionError)?
                //     }
                // }

                // let (history_delegator_nonce_hash, _) = AccountGenerationTaskDelegator::<T>::get(&task_id);
                // let delegator_nonce_hash = Self::sha2_256(&delegator_nonce);
                // ensure!(delegator_nonce_hash == history_delegator_nonce_hash.as_slice(), Error::<T>::InvalidSig);

                let asset_info = Asset {
                    owner: sender,
                    p2: p2,
                    deployment_ids: p2_deployment_ids,
                    web: T::AccountId::default(),
                    app: T::AccountId::default(),
                    multi_sig_account: Vec::new(),
                    data_adhoc: AccountGenerationDataWithoutP3::default()
                };

                Assets::<T>::insert(multi_sig_account.clone(), asset_info.clone());
                // AccountGenerationTaskDelegator::<T>::remove(&task_id);
                Self::deposit_event(RawEvent::AssetGenerated(task_id, multi_sig_account, asset_info));
            } else {
                ensure!(AccountGenerationTaskDelegator::<T>::contains_key(&task_id), Error::<T>::TaskNotExist);
                ensure!(AccountGenerationTasks::contains_key(&task_id), Error::<T>::TaskNotExist);
                ensure!(!Assets::<T>::contains_key(&multi_sig_account), Error::<T>::AssetAlreadyExist);
                let (history_delegator_nonce_hash, _) = AccountGenerationTaskDelegator::<T>::get(&task_id);
                let delegator_nonce_hash = Self::sha2_256(&delegator_nonce);
                ensure!(delegator_nonce_hash == history_delegator_nonce_hash.as_slice(), Error::<T>::InvalidSig);

                let task = AccountGenerationTasks::get(&task_id);
                let (_, browser) = AccountGenerationTaskDelegator::<T>::get(&task_id);
                let (app, _) = BrowserAppPair::<T>::get(&browser);

                let asset_info = Asset {
                    owner: sender,
                    p2: p2,
                    deployment_ids: p2_deployment_ids,
                    web: browser.clone(),
                    app: app,
                    multi_sig_account: multi_sig_account.clone(),
                    data_adhoc: task
                };

                if BrowserMultiSigAccounts::<T>::contains_key(&browser) {
                    BrowserMultiSigAccounts::<T>::insert(browser, vec![multi_sig_account.clone()]);
                } else {
                    let mut accounts = BrowserMultiSigAccounts::<T>::take(&browser);
                    accounts.push(multi_sig_account.clone());
                    BrowserMultiSigAccounts::<T>::insert(browser, accounts);
                }

                Assets::<T>::insert(multi_sig_account.clone(), asset_info.clone());
                // todo clear useless data
                // AccountGenerationTaskDelegator::<T>::remove(&task_id);
                Self::deposit_event(RawEvent::AssetGenerated(task_id, multi_sig_account, asset_info));
            }

            Ok(())
        }

        #[weight = 100]
        pub fn browser_sign_tx(
            origin,
            data_adhoc: TxData,
            delegator_nonce_hash: Cid,
            delegator_nonce_rsa: Cid,
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(BrowserAppPair::<T>::contains_key(&sender), Error::<T>::AppBrowserPairNotExist);

            let task = SignTransactionData {
                data_adhoc: data_adhoc,
                delegator_nonce_hash: delegator_nonce_hash,
                delegator_nonce_rsa: delegator_nonce_rsa,
            };
            let task_data = task.encode();
            let task_id = Self::sha2_256(&task_data).to_vec();
            ensure!(!SignTransactionTasks::contains_key(&task_id), Error::<T>::SignTransactionTaskAlreadyExist);
            ensure!(!SignTransactionTaskSender::<T>::contains_key(&task_id), Error::<T>::SignTransactionTaskAlreadyExist);

            SignTransactionTasks::insert((&task_id).clone(), task.clone());
            let current_block_number = <frame_system::Module<T>>::block_number();
            SignTransactionTaskSender::<T>::insert((&task_id).clone(), (current_block_number, sender.clone()));
            Self::deposit_event(RawEvent::BrowserSignTransactionRequested(sender, task_id, task));

            Ok(())
        }

        #[weight = 100]
        pub fn update_p1_signature (
            origin,
            task_id: Cid,
            multisig_address: Cid,
            p1_signature: TxData,
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(AppBrowserPair::<T>::contains_key(&sender), Error::<T>::AppBrowserPairNotExist);
            ensure!(Assets::<T>::contains_key(&multisig_address), Error::<T>::AssetNotExist);
            ensure!(SignTransactionTasks::contains_key(&task_id), Error::<T>::TaskNotExist);
            ensure!(SignTransactionTaskSender::<T>::contains_key(&task_id), Error::<T>::TaskNotExist);

            let asset = Assets::<T>::get(&multisig_address);
            ensure!(asset.owner == sender, Error::<T>::InvalidAssetOwner);


            let (browser, _metadata) = AppBrowserPair::<T>::get(&sender);
            let (block_number, task_browser) = SignTransactionTaskSender::<T>::get(&task_id);
            ensure!(browser == task_browser, Error::<T>::AppBrowserNotPair);

            let current_block_number = <frame_system::Module<T>>::block_number();
            if current_block_number - block_number > TASK_TIMEOUT_PERIOD.into() {
                SignTransactionTasks::remove(&task_id);
                SignTransactionTaskSender::<T>::remove(&task_id);
                debug::info!("browser task timeout");
                return Err(Error::<T>::TaskTimeout.into());
            }


            let task = SignTransactionTasks::get(&task_id);
            let task_info = SignTransactionTask {
                task_id: task_id,
                multisig_address: multisig_address,
                p1_signature: p1_signature,
                task_data: task,
            };

            Self::deposit_event(RawEvent::SignTransactionRequested(sender, task_info));

            Ok(())
        }

        #[weight = 100]
        pub fn update_sign_tx_result(
            origin,
            task_id: Cid,
            delegator_nonce: Cid,
            succeed: bool,
        ) -> dispatch::DispatchResult {
            let _sender = ensure_signed(origin)?;
            ensure!(SignTransactionTasks::contains_key(&task_id), Error::<T>::TaskNotExist);
            ensure!(!SignTransactionResults::contains_key(&task_id), Error::<T>::SignTransactionResultExist);

            // let mut delegator = [0u8; 32];
            // let public_key_bytes = Self::account_to_bytes(&sender);
            // match public_key_bytes {
            //     Ok(p) => {
            //         delegator = p;
            //     }
            //     Err(_e) => {
            //         debug::info!("failed to parse account");
            //         Err(Error::<T>::AccountIdConvertionError)?
            //     }
            // }
            let task = SignTransactionTasks::get(&task_id);
            let delegator_nonce_hash = Self::sha2_256(&delegator_nonce);
            ensure!(task.delegator_nonce_hash.as_slice() == delegator_nonce_hash, Error::<T>::InvalidSig);

            SignTransactionResults::insert(task_id.clone(), succeed);
            SignTransactionTasks::remove(&task_id);
            SignTransactionTaskSender::<T>::remove(&task_id);
            Self::deposit_event(RawEvent::UpdateSignTransaction(task_id, succeed));

            Ok(())
        }

        #[weight = 100]
        pub fn update_delegator(
            origin,
            delegator_pubkey: TeaPubKey,
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;

            let mut _delegator_tea_id = [0u8; 32];
            let public_key_bytes = Self::account_to_bytes(&sender);
            match public_key_bytes {
                Ok(p) => {
                    _delegator_tea_id = p;
                }
                Err(_e) => {
                    debug::info!("failed to parse account");
                    return Err(Error::<T>::AccountIdConvertionError.into());
                }
            }
            // todo how to check delegator pubkey with layer1 account

            let mut exist = false;
            let current_block_number = <frame_system::Module<T>>::block_number();
            let mut delegates = Delegates::<T>::get();
            for (d, b) in &mut delegates {
                if d == &delegator_pubkey {
                    *b = current_block_number;
                    exist = true;
                    break;
                }
            }
            if !exist {
                delegates.push((delegator_pubkey, current_block_number));
            }
            Delegates::<T>::put(delegates);

            Ok(())
        }

        // #[weight = 100]
        // pub fn transfer_asset(
        //     origin,
        //     from: Cid,
        //     to: Cid,
        // ) -> dispatch::DispatchResult {
        //     let sender = ensure_signed(origin)?;
        //     ensure!(from != to, Error::<T>::InvalidToAccount);
        //     let current_block_number = <frame_system::Module<T>>::block_number();
        //     if TransferAssetTasks::<T>::contains_key(&from) {
        //         ensure!(TransferAssetTasks::<T>::get(&from).to == to, Error::<T>::InvalidToAccount);
        //         // if timeout, need to remove the transfer-asset task.
        //         if TransferAssetTasks::<T>::get(&from).start_height +
        //         MAX_TRANSFER_ASSET_TASK_PERIOD.into() < current_block_number {
        //             TransferAssetTasks::<T>::remove(&from);
        //             TransferAssetSignatures::<T>::remove(&from);
        //         }
        //     }

        //     let mut client_from = T::AccountId::default();
        //     let account_from = Self::bytes_to_account(&mut from.as_slice());
        //     match account_from {
        //         Ok(f) => {
        //             client_from = f;
        //         }
        //         Err(_e) => {
        //             debug::info!("failed to parse client");
        //             Err(Error::<T>::AccountIdConvertionError)?
        //         }
        //     }
        //     let mut client_to = T::AccountId::default();
        //     let account_to = Self::bytes_to_account(&mut from.as_slice());
        //     match account_to {
        //         Ok(t) => {
        //             client_to = t;
        //         }
        //         Err(_e) => {
        //             debug::info!("failed to parse client");
        //             Err(Error::<T>::AccountIdConvertionError)?
        //         }
        //     }

        //     let account_vec = sender.encode();
        //     ensure!(account_vec.len() == 32, Error::<T>::AccountIdConvertionError);

        //     if TransferAssetSignatures::<T>::contains_key(&from) {
        //         let signatures = TransferAssetSignatures::<T>::get(&from);
        //         for sig in signatures.iter() {
        //            ensure!(&sender != sig, Error::<T>::SenderAlreadySigned);
        //         }
        //         let mut signatures = TransferAssetSignatures::<T>::take(&from);
        //         signatures.push(sender.clone());
        //         TransferAssetSignatures::<T>::insert(&from, signatures.clone());
        //         Self::deposit_event(RawEvent::TransferAssetSign(from.clone(), sender.clone()));

        //         if signatures.len() >= MIN_TRANSFER_ASSET_SIGNATURE_COUNT {
        //            // transfer balance
        //            let total_balance = T::Currency::total_balance(&client_from);
        //            if total_balance > 0.into() {
        //                T::Currency::transfer(&client_from, &client_to, total_balance, ExistenceRequirement::AllowDeath)?;
        //                TransferAssetSignatures::<T>::remove(&from);
        //                TransferAssetTasks::<T>::remove(&from);
        //            }

        //            // todo transfer btc and other asset
        //            if AccountAssets::contains_key(&from) {
        //                let mut from_account_assets = AccountAssets::take(&from);
        //                if AccountAssets::contains_key(&to) {
        //                    let mut to_account_assets = AccountAssets::take(&to);
        //                    to_account_assets.btc.append(&mut from_account_assets.btc);
        //                    to_account_assets.eth.append(&mut from_account_assets.eth);
        //                    to_account_assets.dot.append(&mut from_account_assets.dot);
        //                    AccountAssets::insert(&to, to_account_assets);
        //                } else {
        //                    AccountAssets::insert(&to, from_account_assets);
        //                }

        //            }

        //            Self::deposit_event(RawEvent::TransferAssetEnd(from.clone(), TransferAssetTasks::<T>::get(&from)));
        //         }
        //     } else {
        //         TransferAssetSignatures::<T>::insert(&from, vec![sender.clone()]);
        //     }

        //     if !TransferAssetTasks::<T>::contains_key(&from) {
        //        let new_task = TransferAssetTask {
        //            from: from.clone(),
        //             to: to.clone(),
        //             start_height: current_block_number,
        //        };
        //        TransferAssetTasks::<T>::insert(from.clone(), &new_task);
        //        Self::deposit_event(RawEvent::TransferAssetBegin(from.clone(), new_task));
        //     }

        //     Ok(())
        // }

        fn on_finalize(block_number: T::BlockNumber) {
            Self::update_runtime_status(block_number);
        }

        // TODO, mock method, replace it with layer2 solution.
        #[weight = 10]
        pub fn test_add_account_asset(
            origin,
            key_type: Vec<u8>,
            account: Cid,
        )-> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;

            let mut target_account_assets = {
                if AccountAssets::<T>::contains_key(&sender) {
                    AccountAssets::<T>::take(&sender)
                }
                else {
                    AccountAssetData {
                        btc: vec![],
                        eth: vec![],
                        dot: vec![],
                    }
                }
                
            };

            let flag: i8 = match hex::encode(key_type).as_str() {
                // btc
                "627463" => {
                    target_account_assets.btc.push(account);
                    1
                },
                // eth
                "657468" => {
                    target_account_assets.eth.push(account);
                    1
                },
                // dot
                "646f74" => {
                    target_account_assets.dot.push(account);
                    1
                },
                _ => {
                    debug::info!("Invalid key type");
                    -1
                },
            };

            AccountAssets::<T>::insert(sender, target_account_assets);

            ensure!(flag == 1, Error::<T>::InvalidKeyTypeForAccountAsset);
      
            Ok(())
        }

        #[weight = 1000]
        pub fn test_transfer_all_asset(
            origin,
            to: T::AccountId,
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            
            ensure!(sender != to, Error::<T>::InvalidToAccount);

            
            // transfer balance
            let total_balance = T::Currency::free_balance(&sender);
            
            let amount = total_balance.saturating_sub(T::Currency::minimum_balance());
            debug::info!("amount => {:?}", &amount);
            if amount > 0.into() {
                T::Currency::transfer(&sender, &to, amount, ExistenceRequirement::AllowDeath)?;
            }

            // transfer asset
            let from = sender;
            if AccountAssets::<T>::contains_key(&from) {
                let mut from_asset = AccountAssets::<T>::take(&from);
                if AccountAssets::<T>::contains_key(&to) {
                    let mut to_asset = AccountAssets::<T>::take(&to);
                    to_asset.btc.append(&mut from_asset.btc);
                    to_asset.eth.append(&mut from_asset.eth);
                    to_asset.dot.append(&mut from_asset.dot);
                    AccountAssets::<T>::insert(&to, to_asset);
                } else {
                    AccountAssets::<T>::insert(&to, from_asset);
                }
            }


            Ok(())
        }
        
    }
}

impl<T: Trait> Module<T> {
    fn bytes_to_account(mut account_bytes: &[u8]) -> Result<T::AccountId, Error<T>> {
        match T::AccountId::decode(&mut account_bytes) {
            Ok(client) => {
                Ok(client)
            }
            Err(_e) => Err(Error::<T>::AccountIdConvertionError),
        }
    }

    fn account_to_bytes(account: &T::AccountId) -> Result<[u8; 32], Error<T>> {
        let account_vec = account.encode();
        if account_vec.len() != 32 {
            return Err(Error::<T>::AccountIdConvertionError);
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&account_vec);
        Ok(bytes)
    }

    fn update_runtime_status(_block_number: T::BlockNumber) {}

    /// Do a sha2 256-bit hash and return result.
    pub fn sha2_256(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.input(data);
        let mut output = [0u8; 32];
        output.copy_from_slice(&hasher.result());
        output
    }

    // pub fn verify_signature(public_key: [u8; 32], nonce_signature: Vec<u8>, data: Vec<u8> ) -> bool {
    //     let pubkey = sr25519::Public(public_key);
    //     let mut bytes = [0u8; 64];
    //     bytes.copy_from_slice(&nonce_signature);
    //     let signature = sr25519::Signature::from_raw(bytes);
    //     return signature.verify(&data[..], &pubkey);
    // }
}
