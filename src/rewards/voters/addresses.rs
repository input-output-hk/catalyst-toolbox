use chain_crypto::{Blake2b256, Ed25519, PublicKey, Signature, Verification};
use futures::TryFutureExt;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Metadata {
    pub voting_key: String,
    pub stake_public_key: String,
    pub reward_address: String,
    pub nonce: u64,
}

#[derive(Deserialize)]
pub struct Transaction {
    hash: String,
    tx_id: String,
    metadata: Metadata,
    signature: String,
    block_index: u32,
}

fn validate_transaction(transaction: &Transaction) -> bool {
    // First, we parse the data with cbor format, and convert the fields to byte representation.
    // We only need the signature and the verification key (we use the examples of
    // https://github.com/cardano-foundation/CIPs/blob/master/CIP-0015/test-vector.md)
    let signature_byte = hex::decode(&transaction.signature).expect("Decoding failed");

    let verification_key_byte =
        hex::decode(&transaction.metadata.stake_public_key).expect("Decoding failed");

    let signature: Signature<[u8; 32], Ed25519> =
        Signature::<[u8; 32], Ed25519>::from_binary(&signature_byte).expect("Conversion failed");

    let verification_key: PublicKey<Ed25519> =
        PublicKey::<Ed25519>::from_binary(&verification_key_byte).expect("Conversion failed");

    // Next, we need to hash the cbor data. We input the cbor data to Blake2b256 and verify
    // the signature over that message
    let mess_bytes = hex::decode(&"a119ef64a40158200036ef3e1f0d3f5989e2d155ea54bdb2a72c4c456ccb959af4c94868f473f5a002582086870efc99c453a873a16492ce87738ec79a0ebd064379a62e2c9cf4e119219e035839009f0186a15f50592b311b57980e06cf9e791dfcb998a1fb8bfd65d06eae3a0a7aeda4aea522e74e4fe36759fca80789a613a58a4364f6ecef041904d2").unwrap();
    let hashed_message = Blake2b256::new(&mess_bytes);

    matches!(
        signature.verify(&verification_key, &hashed_message.as_hash_bytes()),
        Verification::Success
    )
}
