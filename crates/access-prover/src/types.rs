use serde::{Deserialize, Serialize};
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_bn254::Bn254;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] ark_serialize::SerializationError),
    #[error("Hex decoding error: {0}")]
    Hex(#[from] hex::FromHexError),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MerklePathJson {
    pub siblings: Vec<String>,
    pub indices: Vec<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionInput {
    pub policy_root: String,
    pub session_nonce: String,
    pub ciphertext_hash: String,
    pub user_secret: String,
    pub authorization_note: String,
    pub merkle_path: MerklePathJson,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicInputsJson {
    pub policy_root: String,
    pub session_nonce: String,
    pub ciphertext_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProofJson {
    pub a: String, // hex-encoded compressed G1Affine
    pub b: String, // hex-encoded compressed G2Affine
    pub c: String, // hex-encoded compressed G1Affine
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProofOutput {
    pub version: String,
    pub proof: ProofJson,
    pub public_inputs: PublicInputsJson,
}

// Serialization helpers
pub fn serialize_proof(proof: &ark_groth16::Proof<Bn254>) -> Result<ProofJson, TypeError> {
    let mut a_bytes = Vec::new();
    proof.a.serialize_compressed(&mut a_bytes)?;
    
    let mut b_bytes = Vec::new();
    proof.b.serialize_compressed(&mut b_bytes)?;
    
    let mut c_bytes = Vec::new();
    proof.c.serialize_compressed(&mut c_bytes)?;
    
    Ok(ProofJson {
        a: hex::encode(a_bytes),
        b: hex::encode(b_bytes),
        c: hex::encode(c_bytes),
    })
}

pub fn deserialize_proof(proof_json: &ProofJson) -> Result<ark_groth16::Proof<Bn254>, TypeError> {
    let a_bytes = hex::decode(&proof_json.a)?;
    let a = <Bn254 as ark_ec::pairing::Pairing>::G1Affine::deserialize_compressed(&a_bytes[..])?;
    
    let b_bytes = hex::decode(&proof_json.b)?;
    let b = <Bn254 as ark_ec::pairing::Pairing>::G2Affine::deserialize_compressed(&b_bytes[..])?;
    
    let c_bytes = hex::decode(&proof_json.c)?;
    let c = <Bn254 as ark_ec::pairing::Pairing>::G1Affine::deserialize_compressed(&c_bytes[..])?;
    
    Ok(ark_groth16::Proof { a, b, c })
}
