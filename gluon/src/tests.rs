// Tests to be written here

use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use frame_system::offchain::Account;
use hex as hex_o;
use hex_literal::hex;
use serde::{Deserialize, Serialize};
use serde_json::Result;
use sp_core::crypto::AccountId32;
use sp_core::{
    crypto, ed25519,
    hash::{H256, H512},
    sr25519, Pair,
};
use std::str::FromStr;
use std::vec::Vec;

#[test]
fn it_works_for_default_value() {
    new_test_ext().execute_with(|| {
        assert!(true);
        // asserting that the stored value is equal to what we stored
        // assert_eq!(TemplateModule::something(), Some(42));
    });
}

#[test]
fn test_vector_should_work() {
    let pair = ed25519::Pair::from_seed(&hex!(
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
    ));
    let public = pair.public();
    assert_eq!(
        public,
        ed25519::Public::from_raw(hex!(
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        ))
    );
    let message = b"";
    let signature = hex!("e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b");
    let signature = ed25519::Signature::from_raw(signature);
    assert!(&pair.sign(&message[..]) == &signature);
    assert!(ed25519::Pair::verify(&signature, &message[..], &public));
    assert!(signature.verify(&message[..], &public));
}

#[test]
fn test_vector_by_string_should_work() {
    let pair = ed25519::Pair::from_string(
        "0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        None,
    )
    .unwrap();
    let public = pair.public();
    assert_eq!(
        public,
        ed25519::Public::from_raw(hex!(
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        ))
    );
    let message = b"";
    let signature = hex!("e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b");
    let signature = ed25519::Signature::from_raw(signature);
    assert!(&pair.sign(&message[..]) == &signature);
    assert!(ed25519::Pair::verify(&signature, &message[..], &public));
    assert!(signature.verify(&message[..], &public));
}

#[test]
fn test_ed25519_sig_should_work() {
    let public = ed25519::Public(hex!(
        "e9889b1c54ccd6cf184901ded892069921d76f7749b6f73bed6cf3b9be1a8a44"
    ));
    let message = hex!("e9889b1c54ccd6cf184901ded892069921d76f7749b6f73bed6cf3b9be1a8a441d2905c84be11c0f314792f365e8385270495ebd112dc6363d3f02ec7ccfe475");
    let signature = hex!("26bcb7e99923c877cb6d50afedaf0fca0af4f3c78b437e8c48c7107f9ebdd1a00aa482e67ca244a40f44cf295d1b9f5c416202a5b785401408d8cffad0f18302");
    let signature = ed25519::Signature::from_raw(signature);
    assert!(ed25519::Pair::verify(&signature, &message[..], &public));
    assert!(signature.verify(&message[..], &public));
}

#[test]
fn test_auth_payload_should_right() {
    let winner_tea_id = hex!("e9889b1c54ccd6cf184901ded892069921d76f7749b6f73bed6cf3b9be1a8a44");
    let ref_num = hex!("0c6123c17c95bd6617a01ef899f5895ddb190eb3265f341687f4c0ad1b1f366f");

    let auth_payload = [&winner_tea_id[..], &ref_num[..]].concat();

    assert_eq!(hex_o::encode(auth_payload), "e9889b1c54ccd6cf184901ded892069921d76f7749b6f73bed6cf3b9be1a8a440c6123c17c95bd6617a01ef899f5895ddb190eb3265f341687f4c0ad1b1f366f");
}

#[test]
fn test_delegate_sig_should_work() {
    let winner_tea_id = hex!("e9889b1c54ccd6cf184901ded892069921d76f7749b6f73bed6cf3b9be1a8a44");
    let ref_num = hex!("ba9147ba50faca694452db7c458e33a9a0322acbaac24bf35db7bb5165dff3ac");

    let auth_payload = [&winner_tea_id[..], &ref_num[..]].concat();

    let public = ed25519::Public(hex!(
        "e9889b1c54ccd6cf184901ded892069921d76f7749b6f73bed6cf3b9be1a8a44"
    ));

    let signature = hex!("4f62e775a61ac904d6cfc18473203761b77ef60255c745ea71255606956596f0511ca61d1b2cd039d789f01a98f1d746bb6dc58feb07f945cc4c053050ab0103");
    let signature = ed25519::Signature::from_raw(signature);
    assert!(ed25519::Pair::verify(
        &signature,
        &auth_payload[..],
        &public
    ));
    assert!(signature.verify(&auth_payload[..], &public));

}

#[test]
fn get_tea_id_and_sig() {
    let pair = ed25519::Pair::from_seed(&hex!(
        "119c37b9aa65572ad9e24dd49c4f4da5330fe476f3313c560ffc67888f92b758"
    ));
    let public = pair.public();
    println!("pub: {:?}", public);

    let message = hex!("c7e016fad0796bb68594e49a6ef1942cf7e73497e69edb32d19ba2fab3696597");
    let signature = &pair.sign(&message[..]);
    println!("sig: {:?}", signature);
}

#[test]
fn test_account_id_verification_should_work() {
    let message = hex!("1234");
    let signature = hex!("8ac5b79ca6ff412f576a0163b1daeb6b5d0a51fccab87b25c4d66d0323ca3a7ee95e4a036945409477bc9cd79f48e31c456ce6327ff1ba568817ce19cdee9e81");
    let signature = sr25519::Signature::from_raw(signature);

    let public =
        sr25519::Public::from_str("5GBykvvrUz3vwTttgHzUEPdm7G1FND1reBfddQLdiaCbhoMd").unwrap();
    println!("pub: {:?}", public);
    assert!(sr25519::Pair::verify(&signature, &message[..], &public));
    assert!(signature.verify(&message[..], &public));
}
