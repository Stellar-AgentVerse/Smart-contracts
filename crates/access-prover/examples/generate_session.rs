use ark_ff::UniformRand;
use ark_bn254::Fr;
use rand::thread_rng;
use access_prover::circuit::{get_poseidon_config, poseidon_hash_native};
use access_prover::proof::fr_to_hex;
use access_prover::types::{SessionInput, MerklePathJson};

fn main() {
    let config = get_poseidon_config::<Fr>(3);
    let mut rng = thread_rng();
    
    // Generate some test keys and secrets
    let user_secret = Fr::from(123456789u64);
    let authorization_note = poseidon_hash_native(&[user_secret], &config);
    
    // Construct a valid Merkle path of depth 16
    let leaf_index = 7u32;
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
    
    // Nonce and hash representing request session
    let session_nonce = Fr::from(987654321u64);
    let ciphertext_hash = Fr::from(555555555u64);
    
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
    
    let serialized = serde_json::to_string_pretty(&session_input).unwrap();
    std::fs::write("session.json", serialized).expect("failed to write session.json");
    println!("Generated session.json successfully!");
}
