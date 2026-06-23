use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use ark_ff::{BigInteger, PrimeField};
use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_snark::{CircuitSpecificSetupSNARK, SNARK};
use rand::thread_rng;
use thiserror::Error;

use crate::circuit::AccessCircuit;
use crate::types::{SessionInput, ProofOutput, PublicInputsJson, serialize_proof, deserialize_proof};

#[derive(Error, Debug)]
pub enum ProverError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] ark_serialize::SerializationError),
    #[error("Hex decoding error: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("Parsing error: {0}")]
    Parsing(String),
    #[error("ZKP Synthesis error: {0}")]
    Synthesis(#[from] ark_relations::r1cs::SynthesisError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Type conversion error: {0}")]
    TypeError(#[from] crate::types::TypeError),
}

// Robust helper to parse field element from hex or decimal string
pub fn parse_fr(s: &str) -> Result<Fr, String> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        let hex_part = &s[2..];
        let bytes = hex::decode(hex_part).map_err(|e| format!("Invalid hex: {}", e))?;
        let mut padded = vec![0u8; 32];
        if bytes.len() > 32 {
            return Err("Hex string too long for field element".to_string());
        }
        let start = 32 - bytes.len();
        padded[start..].copy_from_slice(&bytes);
        Ok(Fr::from_be_bytes_mod_order(&padded))
    } else if let Ok(val) = Fr::from_str(s) {
        Ok(val)
    } else if s.chars().all(|c| c.is_ascii_hexdigit()) {
        let bytes = hex::decode(s).map_err(|e| format!("Invalid hex: {}", e))?;
        let mut padded = vec![0u8; 32];
        if bytes.len() > 32 {
            return Err("Hex string too long for field element".to_string());
        }
        let start = 32 - bytes.len();
        padded[start..].copy_from_slice(&bytes);
        Ok(Fr::from_be_bytes_mod_order(&padded))
    } else {
        Err(format!("Failed to parse field element from string: {}", s))
    }
}

pub fn fr_to_hex(fr: &Fr) -> String {
    let bytes_be = fr.into_bigint().to_bytes_be();
    format!("0x{}", hex::encode(bytes_be))
}

pub fn run_setup<P: AsRef<Path>>(
    pk_path: P,
    vk_path: P,
    vk_const_path: Option<P>,
) -> Result<(), ProverError> {
    let mut rng = thread_rng();
    
    // Create empty circuit for setup
    // Merkle tree depth of 16
    let circuit = AccessCircuit::<Fr> {
        policy_root: None,
        session_nonce: None,
        ciphertext_hash: None,
        user_secret: None,
        authorization_note: None,
        merkle_siblings: vec![None; 16],
        merkle_indices: vec![None; 16],
    };
    
    println!("Generating parameter keys (this may take a few seconds)...");
    let (pk, vk) = Groth16::<Bn254>::setup(circuit, &mut rng)?;
    
    // Write PK to file
    let mut pk_file = File::create(pk_path)?;
    pk.serialize_compressed(&mut pk_file)?;
    
    // Write VK to file
    let mut vk_file = File::create(vk_path)?;
    vk.serialize_compressed(&mut vk_file)?;
    
    // Write VK as Rust constant if requested
    if let Some(const_path) = vk_const_path {
        let mut vk_bytes = Vec::new();
        vk.serialize_compressed(&mut vk_bytes)?;
        
        let mut const_file = File::create(const_path)?;
        writeln!(const_file, "/// Generated Verification Key for anonymous access verification")?;
        writeln!(const_file, "pub const VERIFICATION_KEY_BYTES: &[u8] = &[")?;
        for chunk in vk_bytes.chunks(12) {
            let chunk_str = chunk.iter().map(|b| format!("0x{:02x}", b)).collect::<Vec<_>>().join(", ");
            writeln!(const_file, "    {},", chunk_str)?;
        }
        writeln!(const_file, "];")?;
    }
    
    Ok(())
}

pub fn run_prove<P: AsRef<Path>>(
    pk_path: P,
    session: &SessionInput,
) -> Result<ProofOutput, ProverError> {
    // 1. Parse inputs to Fr
    let policy_root = parse_fr(&session.policy_root).map_err(ProverError::Parsing)?;
    let session_nonce = parse_fr(&session.session_nonce).map_err(ProverError::Parsing)?;
    let ciphertext_hash = parse_fr(&session.ciphertext_hash).map_err(ProverError::Parsing)?;
    let user_secret = parse_fr(&session.user_secret).map_err(ProverError::Parsing)?;
    let authorization_note = parse_fr(&session.authorization_note).map_err(ProverError::Parsing)?;
    
    let mut merkle_siblings = Vec::new();
    for s in &session.merkle_path.siblings {
        merkle_siblings.push(Some(parse_fr(s).map_err(ProverError::Parsing)?));
    }
    
    let mut merkle_indices = Vec::new();
    for &idx in &session.merkle_path.indices {
        merkle_indices.push(Some(idx));
    }
    
    // Make sure path is padded to 16
    while merkle_siblings.len() < 16 {
        merkle_siblings.push(Some(Fr::from(0u32)));
        merkle_indices.push(Some(false));
    }
    
    // 2. Load Proving Key
    let mut pk_file = File::open(pk_path)?;
    let pk = ProvingKey::<Bn254>::deserialize_compressed(&mut pk_file)?;
    
    // 3. Create circuit instance
    let circuit = AccessCircuit {
        policy_root: Some(policy_root),
        session_nonce: Some(session_nonce),
        ciphertext_hash: Some(ciphertext_hash),
        user_secret: Some(user_secret),
        authorization_note: Some(authorization_note),
        merkle_siblings,
        merkle_indices,
    };
    
    // 4. Generate proof
    let mut rng = thread_rng();
    let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng)?;
    
    // 5. Build output JSON
    let proof_json = serialize_proof(&proof)?;
    let public_inputs = PublicInputsJson {
        policy_root: fr_to_hex(&policy_root),
        session_nonce: fr_to_hex(&session_nonce),
        ciphertext_hash: fr_to_hex(&ciphertext_hash),
    };
    
    Ok(ProofOutput {
        version: "1.0.0".to_string(),
        proof: proof_json,
        public_inputs,
    })
}

pub fn run_verify<P: AsRef<Path>>(
    vk_path: P,
    proof_out: &ProofOutput,
) -> Result<bool, ProverError> {
    // 1. Load Verification Key
    let mut vk_file = File::open(vk_path)?;
    let vk = VerifyingKey::<Bn254>::deserialize_compressed(&mut vk_file)?;
    
    verify_with_vk(&vk, proof_out)
}

pub fn verify_with_vk(
    vk: &VerifyingKey<Bn254>,
    proof_out: &ProofOutput,
) -> Result<bool, ProverError> {
    // 2. Deserialize proof
    let proof = deserialize_proof(&proof_out.proof)?;
    
    // 3. Parse public inputs in order: policy_root, session_nonce, ciphertext_hash
    let policy_root = parse_fr(&proof_out.public_inputs.policy_root).map_err(ProverError::Parsing)?;
    let session_nonce = parse_fr(&proof_out.public_inputs.session_nonce).map_err(ProverError::Parsing)?;
    let ciphertext_hash = parse_fr(&proof_out.public_inputs.ciphertext_hash).map_err(ProverError::Parsing)?;
    
    let public_inputs = vec![policy_root, session_nonce, ciphertext_hash];
    
    // 4. Verify
    let is_valid = Groth16::<Bn254>::verify(vk, &public_inputs, &proof)?;
    Ok(is_valid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::{get_poseidon_config, poseidon_hash_native};
    use crate::types::MerklePathJson;
    use rand::thread_rng;
    use ark_ff::UniformRand;
    use ark_r1cs_std::prelude::*;
    use ark_r1cs_std::fields::fp::FpVar;
    use ark_r1cs_std::alloc::AllocVar;
    
    #[test]
    fn test_parse_fr() {
        // Decimal
        let val = parse_fr("12345").unwrap();
        assert_eq!(val, Fr::from(12345u64));
        
        // Hex with 0x
        let val2 = parse_fr("0x3039").unwrap(); // 12345 in hex is 0x3039
        assert_eq!(val2, Fr::from(12345u64));
        
        // Hex without 0x (must contain non-decimal chars to bypass Fr::from_str)
        let val3 = parse_fr("ff").unwrap();
        assert_eq!(val3, Fr::from(255u64));
        
        // Invalid
        assert!(parse_fr("abcg").is_err());
    }
    
    #[test]
    fn test_poseidon_native_and_circuit() {
        use ark_relations::r1cs::ConstraintSystem;
        
        let config = get_poseidon_config::<Fr>(3);
        let in1 = Fr::from(42u64);
        let in2 = Fr::from(100u64);
        
        let hash_native = poseidon_hash_native(&[in1, in2], &config);
        
        let cs = ConstraintSystem::<Fr>::new_ref();
        let in1_var = FpVar::new_witness(cs.clone(), || Ok(in1)).unwrap();
        let in2_var = FpVar::new_witness(cs.clone(), || Ok(in2)).unwrap();
        
        let hash_var = crate::circuit::poseidon_hash_circuit(&[in1_var, in2_var], &config).unwrap();
        assert_eq!(hash_var.value().unwrap(), hash_native);
        assert!(cs.is_satisfied().unwrap());
    }
    
    #[test]
    fn test_proof_e2e_happy_and_sad() {
        let config = get_poseidon_config::<Fr>(3);
        let mut rng = thread_rng();
        
        // Setup secrets
        let user_secret = Fr::from(99999u64);
        let authorization_note = poseidon_hash_native(&[user_secret], &config);
        
        // Construct a valid Merkle path of depth 16
        let leaf_index = 5u32; // binary: 101
        let mut siblings = Vec::new();
        let mut indices = Vec::new();
        let mut current = authorization_note;
        
        for i in 0..16 {
            let sibling = Fr::rand(&mut rng);
            let is_right = ((leaf_index >> i) & 1) == 1;
            
            siblings.push(sibling);
            indices.push(is_right);
            
            current = if is_right {
                poseidon_hash_native(&[sibling, current], &config)
            } else {
                poseidon_hash_native(&[current, sibling], &config)
            };
        }
        let policy_root = current;
        
        // Session params
        let session_nonce = Fr::from(12345678u64);
        let ciphertext_hash = Fr::from(88888888u64);
        
        // Create session input JSON
        let session_input = SessionInput {
            policy_root: fr_to_hex(&policy_root),
            session_nonce: fr_to_hex(&session_nonce),
            ciphertext_hash: fr_to_hex(&ciphertext_hash),
            user_secret: fr_to_hex(&user_secret),
            authorization_note: fr_to_hex(&authorization_note),
            merkle_path: MerklePathJson {
                siblings: siblings.iter().map(fr_to_hex).collect(),
                indices,
            },
        };
        
        // Temporary keys in target directory
        let dir = std::env::temp_dir();
        let pk_path = dir.join("test_pk.bin");
        let vk_path = dir.join("test_vk.bin");
        let vk_const_path = dir.join("test_vk_const.rs");
        
        // Setup
        run_setup(&pk_path, &vk_path, Some(&vk_const_path)).unwrap();
        
        // Prove
        let proof_out = run_prove(&pk_path, &session_input).unwrap();
        
        // Verify (Happy path)
        let is_valid = run_verify(&vk_path, &proof_out).unwrap();
        assert!(is_valid, "Happy-path verification failed");
        
        // Sad path: Tampered policy_root
        let mut proof_bad_root = proof_out.clone();
        proof_bad_root.public_inputs.policy_root = fr_to_hex(&Fr::from(11111u64));
        let is_valid_bad_root = run_verify(&vk_path, &proof_bad_root).unwrap();
        assert!(!is_valid_bad_root, "Verification should fail with bad policy root");
        
        // Sad path: Tampered nonce
        let mut proof_bad_nonce = proof_out.clone();
        proof_bad_nonce.public_inputs.session_nonce = fr_to_hex(&Fr::from(11111u64));
        let is_valid_bad_nonce = run_verify(&vk_path, &proof_bad_nonce).unwrap();
        assert!(!is_valid_bad_nonce, "Verification should fail with bad nonce");
        
        // Clean up temp files if they exist
        let _ = std::fs::remove_file(pk_path);
        let _ = std::fs::remove_file(vk_path);
        let _ = std::fs::remove_file(vk_const_path);
    }
}

