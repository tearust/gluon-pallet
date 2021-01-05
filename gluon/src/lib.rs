#![cfg_attr(not(feature = "std"), no_std)]

/// A FRAME pallet template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references

/// For more guidance on Substrate FRAME, see the example pallet
/// https://github.com/paritytech/substrate/blob/master/frame/example/src/lib.rs
use frame_system::ensure_signed;
use codec::{Decode, Encode};
use frame_support::{
    debug,
    decl_event, decl_module, decl_storage, decl_error, dispatch,
    StorageMap, StorageValue, ensure,
    traits::{Currency,  ExistenceRequirement::AllowDeath}};
use sp_std::prelude::*;
use sp_core::ed25519;
use pallet_balances as balances;
use sp_runtime::traits::Verify;
use sha2::{Sha256, Digest};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

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

const RUNTIME_ACTIVITY_THRESHOLD: u32 = 3600;
const MIN_TRANSFER_ASSET_SIGNATURE_COUNT: usize = 2;
const MAX_TRANSFER_ASSET_TASK_PERIOD: u32 = 100;
const TASK_TIMEOUT_PERIOD: u32 = 30;

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct AccountAsset {
    pub account_id: Cid,
    pub btc: Vec<Cid>,
    pub eth: Vec<Cid>,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct TransferAssetTask<BlockNumber> {
    pub from: Cid,
    pub to: Cid,
    pub start_height: BlockNumber,
}

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
	        map hasher(twox_64_concat) Cid => AccountAsset;

        // Register Gluon wallet account.
        // Temporary storage
	    BrowserNonce get(fn browser_nonce):
	        map hasher(blake2_128_concat) T::AccountId => (T::BlockNumber, Cid);
	    // Permanent storage
        BrowserAppPair get(fn browser_app_pair):
            map hasher(blake2_128_concat) T::AccountId => T::AccountId;
        AppBrowserPair get(fn app_browser_pair):
            map hasher(blake2_128_concat) T::AccountId => T::AccountId;

        // Generate BTC 2/3 MultiSig Account
        // Temporary storage
        BrowserAccountNonce get(fn browser_account_nonce):
            map hasher(blake2_128_concat) T::AccountId => (T::BlockNumber, Cid, Cid); // value: (nonce hash, task hash)
        // Intermediate storage to wait for task result
        AccountGenerationTaskDelegator get(fn account_generation_task_delegator):
            map hasher(blake2_128_concat) Cid => Cid; // key: app, value: key type
        // Permanent storage
        Assets get(fn assets):
            map hasher(blake2_128_concat) Cid => Asset<T::AccountId>; // key: multiSigAccount value


        // Sign transaction
        // Temporary storage
	    SignTransactionTasks get(fn sign_transaction_tasks):
	        map hasher(blake2_128_concat) Cid => SignTransactionData;
	    SignTransactionTaskSender get(fn sign_transaction_sender):
	        map hasher(blake2_128_concat) Cid => (T::BlockNumber, T::AccountId);
	    // Permanent storage
	    SignTransactionResults get(fn sign_transaction_results):
	        map hasher(blake2_128_concat) Cid => bool;

        // DelegatesCount get(fn delegates_count): u64;
        Delegates get(fn delegates): Vec<(TeaPubKey, T::BlockNumber)>;
	         // map hasher(blake2_128_concat) TeaPubKey => T::BlockNumber;
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
                index = index+1;
                if index > start {
                    result.push(d.into());
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
		AccountGenrationRequested(AccountId, Cid, AccountGenerationDataWithoutP3),
		AssetGenerated(Cid, Cid, Asset<AccountId>),
		BrowserSignTransactionRequested(AccountId, Cid, SignTransactionData),
		SignTransactionRequested(AccountId, Cid, Cid, SignTransactionData),
		UpdateSignTransaction(Cid, bool),
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
            ensure!(!BrowserNonce::<T>::contains_key(&sender), Error::<T>::BrowserNonceAlreadyExist);
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
            nonce_signature: Signature,
            browser_pk: ClientPubKey,
		) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(!AppBrowserPair::<T>::contains_key(&sender), Error::<T>::AppBrowserPairAlreadyExist);
            ensure!(nonce_signature.len() == 64, Error::<T>::InvalidNonceSig);

            // check signature of appPubKey
            let mut app_pk = [0u8; 32];
            let app_public_key_bytes = Self::account_to_bytes(&sender);
            match app_public_key_bytes {
                Ok(p) => {
                    app_pk = p;
                }
                Err(_e) => {
                    debug::info!("failed to parse app account");
                    Err(Error::<T>::AccountIdConvertionError)?
                }
            }
            ensure!(Self::verify_signature(app_pk, nonce_signature, nonce.clone()), Error::<T>::InvalidNonceSig);

            // check browser is not paired.
            let mut browser_account = T::AccountId::default();
            let browser = Self::bytes_to_account(&mut browser_pk.as_slice());
            match browser {
                Ok(p) => {
                    browser_account = p;
                }
                Err(_e) => {
                    debug::info!("failed to parse browser pubKey");
                    Err(Error::<T>::AccountIdConvertionError)?
                }
            }
            ensure!(!BrowserAppPair::<T>::contains_key(&browser_account), Error::<T>::AppBrowserPairAlreadyExist);
            ensure!(BrowserNonce::<T>::contains_key(&browser_account), Error::<T>::NonceNotExist);

            let app_nonce_hash = Self::sha2_256(&nonce.as_slice());
            let (block_number, browser_nonce_hash) = BrowserNonce::<T>::get(&browser_account);
            let current_block_number = <frame_system::Module<T>>::block_number();
            if current_block_number - block_number > TASK_TIMEOUT_PERIOD.into() {
                BrowserNonce::<T>::remove(&browser_account);
                debug::info!("browser task timeout");
                Err(Error::<T>::TaskTimeout)?
            }
            ensure!(browser_nonce_hash == app_nonce_hash, Error::<T>::NonceNotMatch);

            // pair succeed, record it into BrowserAppPair and AppBrowserPair and
            // remove data from AppRegistration and BrowserNonce.
            BrowserAppPair::<T>::insert(browser_account.clone(), sender.clone());
            AppBrowserPair::<T>::insert(browser_account.clone(), sender.clone());
            BrowserNonce::<T>::remove(browser_account.clone());

            // pair finished and fire an event
            Self::deposit_event(RawEvent::RegistrationApplicationSucceed(sender, browser_account));

            Ok(())
		}

        #[weight = 100]
        pub fn browser_generate_account(
            origin,
            nonce_hash: Cid,
            task_hash: Cid,
        ) -> dispatch::DispatchResult {
            // todo add logic of timeout
            let sender = ensure_signed(origin)?;
            ensure!(!BrowserAccountNonce::<T>::contains_key(&sender), Error::<T>::BrowserNonceAlreadyExist);

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
           nonce_signature: Cid,
           delegator_nonce_hash: Cid,
           delegator_nonce_rsa: Cid,
           key_type: Cid,
           p1: Cid,
           p2_n: u32,
           p2_k: u32,
           browser_pk: ClientPubKey,
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(nonce_signature.len() == 64, Error::<T>::InvalidNonceSig);

            // check browser is not paired.
            let mut browser_account = T::AccountId::default();
            let browser = Self::bytes_to_account(&mut browser_pk.as_slice());
            match browser {
                Ok(p) => {
                    browser_account = p;
                }
                Err(_e) => {
                    debug::info!("failed to parse browser pubKey");
                    Err(Error::<T>::AccountIdConvertionError)?
                }
            }
            ensure!(BrowserAccountNonce::<T>::contains_key(&browser_account), Error::<T>::NonceNotExist);

            // check signature of appPubKey
            let mut app_pk = [0u8; 32];
            let app_public_key_bytes = Self::account_to_bytes(&sender);
            match app_public_key_bytes {
                Ok(p) => {
                    app_pk = p;
                }
                Err(_e) => {
                    debug::info!("failed to parse app account");
                    Err(Error::<T>::AccountIdConvertionError)?
                }
            }
            ensure!(Self::verify_signature(app_pk, nonce_signature, nonce.clone()), Error::<T>::InvalidNonceSig);

            // check task hash
            let task = AccountGenerationDataWithoutP3 {
                key_type: key_type.clone(),
                n: p2_n,
                k: p2_k,
                delegator_nonce_hash: delegator_nonce_hash.clone(),
                delegator_nonce_rsa: delegator_nonce_rsa,
                p1: p1,
            };
            let task_data = task.encode();
            let task_hash = Self::sha2_256(&task_data);
            let app_nonce_hash = Self::sha2_256(&nonce.as_slice());
            let (block_number, browser_nonce_hash, browser_task_hash) = BrowserAccountNonce::<T>::get(&browser_account);
            ensure!(browser_nonce_hash == app_nonce_hash, Error::<T>::NonceNotMatch);
            ensure!(browser_task_hash == task_hash, Error::<T>::TaskNotMatch);

            let current_block_number = <frame_system::Module<T>>::block_number();
            if current_block_number - block_number > TASK_TIMEOUT_PERIOD.into() {
                BrowserAccountNonce::<T>::remove(&browser_account);
                debug::info!("browser task timeout");
                Err(Error::<T>::TaskTimeout)?
            }

            // account generation requested and fire an event
            AccountGenerationTaskDelegator::insert(task_hash.to_vec(), delegator_nonce_hash.clone());
            BrowserAccountNonce::<T>::remove(&browser_account);
            Self::deposit_event(RawEvent::AccountGenrationRequested(sender, task_hash.to_vec(), task));

            Ok(())
        }

		#[weight = 100]
        pub fn update_generate_account_without_p3_result(
	        origin,
	        task_id: Cid,
	        delegator_nonce: Cid,
	        p2: Cid,
            p2_deployment_ids: Vec<Cid>,
            multi_sig_account: Cid,
        )-> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(AccountGenerationTaskDelegator::contains_key(&task_id), Error::<T>::TaskNotExist);
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
            let history_delegator_nonce_hash = AccountGenerationTaskDelegator::get(&task_id);
            let delegator_nonce_hash = Self::sha2_256(&delegator_nonce);
            ensure!(delegator_nonce_hash == history_delegator_nonce_hash.as_slice(), Error::<T>::InvalidSig);

            let asset_info = Asset {
                owner: sender.clone(),
                p2: p2.clone(),
                deployment_ids: p2_deployment_ids.clone()
            };

            Assets::<T>::insert(multi_sig_account.clone(), asset_info.clone());
            AccountGenerationTaskDelegator::remove(&task_id);
            Self::deposit_event(RawEvent::AssetGenerated(task_id, multi_sig_account, asset_info));

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
            p1_signautre: TxData,
		) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;
		    ensure!(AppBrowserPair::<T>::contains_key(&sender), Error::<T>::AppBrowserPairNotExist);
		    ensure!(Assets::<T>::contains_key(&multisig_address), Error::<T>::AssetNotExist);
		    ensure!(SignTransactionTasks::contains_key(&task_id), Error::<T>::TaskNotExist);
		    ensure!(SignTransactionTaskSender::<T>::contains_key(&task_id), Error::<T>::TaskNotExist);

            let asset = Assets::<T>::get(&multisig_address);
            ensure!(asset.owner == sender, Error::<T>::InvalidAssetOwner);


            let browser = AppBrowserPair::<T>::get(&sender);
            let (block_number, task_browser) = SignTransactionTaskSender::<T>::get(&task_id);
            ensure!(browser == task_browser, Error::<T>::AppBrowserNotPair);

            let current_block_number = <frame_system::Module<T>>::block_number();
            if current_block_number - block_number > TASK_TIMEOUT_PERIOD.into() {
                SignTransactionTasks::remove(&task_id);
                SignTransactionTaskSender::<T>::remove(&task_id);
                debug::info!("browser task timeout");
                Err(Error::<T>::TaskTimeout)?
            }


            let task = SignTransactionTasks::get(&task_id);
            Self::deposit_event(RawEvent::SignTransactionRequested(sender, task_id, multisig_address, task));

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

            SignTransactionResults::insert(task_id.clone(), succeed.clone());
            SignTransactionTasks::remove(&task_id);
            SignTransactionTaskSender::<T>::remove(&task_id);
            Self::deposit_event(RawEvent::UpdateSignTransaction(task_id, succeed));

            Ok(())
		}

        #[weight = 100]
        pub fn update_delegator(
		    origin,
		) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;

            let mut delegator_tea_id = [0u8; 32];
            let public_key_bytes = Self::account_to_bytes(&sender);
            match public_key_bytes {
                Ok(p) => {
                    delegator_tea_id = p;
                }
                Err(_e) => {
                    debug::info!("failed to parse account");
                    Err(Error::<T>::AccountIdConvertionError)?
                }
            }

            let mut exist = false;
            let current_block_number = <frame_system::Module<T>>::block_number();
            let mut delegates = Delegates::<T>::get();
            for (d, b) in &mut delegates {
                if d == &delegator_tea_id {
                    *b = current_block_number;
                    exist = true;
                    break;
                }
            }
            if !exist {
                delegates.push((delegator_tea_id, current_block_number));
            }
            Delegates::<T>::put(delegates);

            Ok(())
		}

		#[weight = 100]
        pub fn transfer_asset(
		    origin,
            from: Cid,
            to: Cid,
		) -> dispatch::DispatchResult {
		    let sender = ensure_signed(origin)?;
		    ensure!(from != to, Error::<T>::InvalidToAccount);
		    let current_block_number = <frame_system::Module<T>>::block_number();
		    if TransferAssetTasks::<T>::contains_key(&from) {
		        ensure!(TransferAssetTasks::<T>::get(&from).to == to, Error::<T>::InvalidToAccount);
		        // if timeout, need to remove the transfer-asset task.
		        if TransferAssetTasks::<T>::get(&from).start_height +
		        MAX_TRANSFER_ASSET_TASK_PERIOD.into() < current_block_number {
		            TransferAssetTasks::<T>::remove(&from);
		            TransferAssetSignatures::<T>::remove(&from);
		        }
		    }

            let mut client_from = T::AccountId::default();
            let account_from = Self::bytes_to_account(&mut from.as_slice());
            match account_from {
                Ok(f) => {
                    client_from = f;
                }
                Err(_e) => {
                    debug::info!("failed to parse client");
                    Err(Error::<T>::AccountIdConvertionError)?
                }
            }
            let mut client_to = T::AccountId::default();
            let account_to = Self::bytes_to_account(&mut from.as_slice());
            match account_to {
                Ok(t) => {
                    client_to = t;
                }
                Err(_e) => {
                    debug::info!("failed to parse client");
                    Err(Error::<T>::AccountIdConvertionError)?
                }
            }

            let account_vec = sender.encode();
            ensure!(account_vec.len() == 32, Error::<T>::AccountIdConvertionError);

            if TransferAssetSignatures::<T>::contains_key(&from) {
                let signatures = TransferAssetSignatures::<T>::get(&from);
                for sig in signatures.iter() {
                   ensure!(&sender != sig, Error::<T>::SenderAlreadySigned);
                }
                let mut signatures = TransferAssetSignatures::<T>::take(&from);
                signatures.push(sender.clone());
                TransferAssetSignatures::<T>::insert(&from, signatures.clone());
                Self::deposit_event(RawEvent::TransferAssetSign(from.clone(), sender.clone()));

                if signatures.len() >= MIN_TRANSFER_ASSET_SIGNATURE_COUNT {
                   // transfer balance
                   let total_balance = T::Currency::total_balance(&client_from);
                   if total_balance > 0.into() {
                       T::Currency::transfer(&client_from, &client_to, total_balance, AllowDeath)?;
                       TransferAssetSignatures::<T>::remove(&from);
                       TransferAssetTasks::<T>::remove(&from);
                   }

                   // todo transfer btc and other asset
                   if AccountAssets::contains_key(&from) {
                       let mut from_account_assets = AccountAssets::take(&from);
                       if AccountAssets::contains_key(&to) {
                           let mut to_account_assets = AccountAssets::take(&to);
                           to_account_assets.btc.append(&mut from_account_assets.btc);
                           to_account_assets.eth.append(&mut from_account_assets.eth);
                           AccountAssets::insert(&to, to_account_assets);
                       } else {
                           AccountAssets::insert(&to, from_account_assets);
                       }

                   }

                   Self::deposit_event(RawEvent::TransferAssetEnd(from.clone(), TransferAssetTasks::<T>::get(&from)));
                }
            } else {
                TransferAssetSignatures::<T>::insert(&from, vec![sender.clone()]);
            }

            if !TransferAssetTasks::<T>::contains_key(&from) {
               let new_task = TransferAssetTask {
                   from: from.clone(),
            	    to: to.clone(),
            	    start_height: current_block_number,
               };
               TransferAssetTasks::<T>::insert(from.clone(), &new_task);
               Self::deposit_event(RawEvent::TransferAssetBegin(from.clone(), new_task));
            }

            Ok(())
		}

		fn on_finalize(block_number: T::BlockNumber) {
            Self::update_runtime_status(block_number);
        }
	}
}

impl<T: Trait> Module<T> {
    fn bytes_to_account(mut account_bytes: &[u8]) -> Result<T::AccountId, Error<T>> {
        match T::AccountId::decode(&mut account_bytes) {
            Ok(client) => {
                return Ok(client);
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

    fn update_runtime_status(_block_number: T::BlockNumber) {
    }

    /// Do a sha2 256-bit hash and return result.
    pub fn sha2_256(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.input(data);
        let mut output = [0u8; 32];
        output.copy_from_slice(&hasher.result());
        output
    }

    pub fn verify_signature(public_key: [u8; 32], nonce_signature: Vec<u8>, data: Vec<u8> ) -> bool {
        let pubkey = ed25519::Public(public_key);
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&nonce_signature);
        let signature = ed25519::Signature::from_raw(bytes);
        return signature.verify(&data[..], &pubkey);
    }
}
