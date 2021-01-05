# gluon-pallet
pallet of gluon wallet

## Functions of gluon pallet

### Register Gluon wallet account

```browser_send_nonce```: Browser send the hashed nonce to layer1 and layer1: store the hashed nonce in account info map 

```send_registration_application```: App send registration application to layer1

### Generate BTC 2/3 MultiSig Account

```browser_generate_account```: Browser send hash of nonce, hash of task detail, and signature to layer1, and layer1 verfiy the signature and store the hashed nonce and hash of the task detail.

```generate_account_without_p3```: App send nonce, signed nonce using AppSecKey, delegator and task full detail to layer1, and layer1 verify the signature, compare the nonce hash of browser and app,  compare the task hash of browser and app.

```update_generate_account_without_p3_result:```: Layer2 update the task result.

### In common usage cases

```browser_sign_tx```: Browser request gluon to sign using P2, send signature and transaction data to layer1, and layer1 verify the account of browser and store the transaction detail temporarily to the block chain.

```update_p1_signature```: App sign transaction using P1 and send the signature to layer1, and layer1 verify app account and store the signature temporarily to the block chain. Start task to sign transaction using P2 and remove signature and transaction data from the block chain.

```update_sign_tx_result```: Layer2 update the task result.
