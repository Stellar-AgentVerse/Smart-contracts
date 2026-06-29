use ark_ff::PrimeField;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_r1cs_std::prelude::*;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::alloc::AllocVar;
use serde::{Deserialize, Serialize};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

// Standard Poseidon configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoseidonConfig<F: PrimeField> {
    pub full_rounds: usize,
    pub partial_rounds: usize,
    pub alpha: u64,
    pub mds: Vec<Vec<F>>,
    pub ark: Vec<Vec<F>>,
}

// Deterministically generate Poseidon parameters for a given state width
pub fn get_poseidon_config<F: PrimeField>(width: usize) -> PoseidonConfig<F> {
    let mut rng = ChaCha8Rng::seed_from_u64(1337); // fixed seed for reproducibility
    let full_rounds = 8;
    let partial_rounds = if width <= 3 { 56 } else { 60 };
    let alpha = 5;
    
    // Generate ARK (round constants)
    let mut ark = Vec::new();
    for _ in 0..(full_rounds + partial_rounds) {
        let mut round_constants = Vec::new();
        for _ in 0..width {
            round_constants.push(F::rand(&mut rng));
        }
        ark.push(round_constants);
    }
    
    // Generate MDS matrix using Cauchy matrix construction: MDS[i][j] = 1 / (x_i + y_j)
    let mut x = Vec::new();
    let mut y = Vec::new();
    while x.len() < width {
        let val = F::rand(&mut rng);
        if !x.contains(&val) {
            x.push(val);
        }
    }
    while y.len() < width {
        let val = F::rand(&mut rng);
        if !x.contains(&val) && !y.contains(&val) {
            // Check that it won't cause x_i + y_j = 0 for any existing x_i
            let mut ok = true;
            for &xi in &x {
                if xi + val == F::zero() {
                    ok = false;
                    break;
                }
            }
            if ok {
                y.push(val);
            }
        }
    }
    
    let mut mds = vec![vec![F::zero(); width]; width];
    for i in 0..width {
        for j in 0..width {
            mds[i][j] = (x[i] + y[j]).inverse().expect("failed to invert Cauchy matrix element");
        }
    }
    
    PoseidonConfig {
        full_rounds,
        partial_rounds,
        alpha,
        mds,
        ark,
    }
}

// Native Poseidon permutation
fn poseidon_permute_native<F: PrimeField>(
    state: &mut [F],
    config: &PoseidonConfig<F>,
) {
    let t = state.len();
    let mut round_idx = 0;
    
    // 1. Full rounds (first RF/2 rounds)
    for _ in 0..(config.full_rounds / 2) {
        for i in 0..t {
            state[i] += config.ark[round_idx][i];
        }
        for i in 0..t {
            let x2 = state[i].square();
            let x4 = x2.square();
            state[i] *= x4;
        }
        let mut new_state = vec![F::zero(); t];
        for i in 0..t {
            for j in 0..t {
                new_state[i] += state[j] * config.mds[i][j];
            }
        }
        state.copy_from_slice(&new_state);
        round_idx += 1;
    }
    
    // 2. Partial rounds (RP rounds)
    for _ in 0..config.partial_rounds {
        for i in 0..t {
            state[i] += config.ark[round_idx][i];
        }
        let x2 = state[0].square();
        let x4 = x2.square();
        state[0] *= x4;
        
        let mut new_state = vec![F::zero(); t];
        for i in 0..t {
            for j in 0..t {
                new_state[i] += state[j] * config.mds[i][j];
            }
        }
        state.copy_from_slice(&new_state);
        round_idx += 1;
    }
    
    // 3. Full rounds (last RF/2 rounds)
    for _ in 0..(config.full_rounds / 2) {
        for i in 0..t {
            state[i] += config.ark[round_idx][i];
        }
        for i in 0..t {
            let x2 = state[i].square();
            let x4 = x2.square();
            state[i] *= x4;
        }
        let mut new_state = vec![F::zero(); t];
        for i in 0..t {
            for j in 0..t {
                new_state[i] += state[j] * config.mds[i][j];
            }
        }
        state.copy_from_slice(&new_state);
        round_idx += 1;
    }
}

// Native Poseidon hash
pub fn poseidon_hash_native<F: PrimeField>(
    inputs: &[F],
    config: &PoseidonConfig<F>,
) -> F {
    let t = 3;
    let mut state = vec![F::zero(); t];
    for i in 0..inputs.len() {
        if i + 1 < t {
            state[i + 1] = inputs[i];
        }
    }
    poseidon_permute_native(&mut state, config);
    state[1]
}

// Circuit Poseidon permutation
fn poseidon_permute_circuit<F: PrimeField>(
    state: &mut [FpVar<F>],
    config: &PoseidonConfig<F>,
) -> Result<(), SynthesisError> {
    let t = state.len();
    let mut round_idx = 0;
    
    // 1. Full rounds (first RF/2 rounds)
    for _ in 0..(config.full_rounds / 2) {
        for i in 0..t {
            state[i] = &state[i] + &FpVar::Constant(config.ark[round_idx][i]);
        }
        for i in 0..t {
            let x2 = state[i].square()?;
            let x4 = x2.square()?;
            state[i] = &state[i] * &x4;
        }
        let mut new_state = vec![FpVar::zero(); t];
        for i in 0..t {
            for j in 0..t {
                new_state[i] = &new_state[i] + &(&state[j] * &FpVar::Constant(config.mds[i][j]));
            }
        }
        state.clone_from_slice(&new_state);
        round_idx += 1;
    }
    
    // 2. Partial rounds (RP rounds)
    for _ in 0..config.partial_rounds {
        for i in 0..t {
            state[i] = &state[i] + &FpVar::Constant(config.ark[round_idx][i]);
        }
        let x2 = state[0].square()?;
        let x4 = x2.square()?;
        state[0] = &state[0] * &x4;
        
        let mut new_state = vec![FpVar::zero(); t];
        for i in 0..t {
            for j in 0..t {
                new_state[i] = &new_state[i] + &(&state[j] * &FpVar::Constant(config.mds[i][j]));
            }
        }
        state.clone_from_slice(&new_state);
        round_idx += 1;
    }
    
    // 3. Full rounds (last RF/2 rounds)
    for _ in 0..(config.full_rounds / 2) {
        for i in 0..t {
            state[i] = &state[i] + &FpVar::Constant(config.ark[round_idx][i]);
        }
        for i in 0..t {
            let x2 = state[i].square()?;
            let x4 = x2.square()?;
            state[i] = &state[i] * &x4;
        }
        let mut new_state = vec![FpVar::zero(); t];
        for i in 0..t {
            for j in 0..t {
                new_state[i] = &new_state[i] + &(&state[j] * &FpVar::Constant(config.mds[i][j]));
            }
        }
        state.clone_from_slice(&new_state);
        round_idx += 1;
    }
    
    Ok(())
}

// Circuit Poseidon hash
pub fn poseidon_hash_circuit<F: PrimeField>(
    inputs: &[FpVar<F>],
    config: &PoseidonConfig<F>,
) -> Result<FpVar<F>, SynthesisError> {
    let t = 3;
    let mut state = vec![FpVar::Constant(F::zero()); t];
    for i in 0..inputs.len() {
        if i + 1 < t {
            state[i + 1] = inputs[i].clone();
        }
    }
    poseidon_permute_circuit(&mut state, config)?;
    Ok(state[1].clone())
}

#[derive(Clone, Debug)]
pub struct AccessCircuit<F: PrimeField> {
    // Public inputs
    pub policy_root: Option<F>,
    pub session_nonce: Option<F>,
    pub ciphertext_hash: Option<F>,

    // Private inputs
    pub user_secret: Option<F>,
    pub authorization_note: Option<F>,
    pub merkle_siblings: Vec<Option<F>>,
    pub merkle_indices: Vec<Option<bool>>,
}

impl<F: PrimeField> ConstraintSynthesizer<F> for AccessCircuit<F> {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        let config = get_poseidon_config::<F>(3);
        
        // 1. Allocate public inputs
        let policy_root_var = FpVar::new_input(cs.clone(), || {
            self.policy_root.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let session_nonce_var = FpVar::new_input(cs.clone(), || {
            self.session_nonce.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let ciphertext_hash_var = FpVar::new_input(cs.clone(), || {
            self.ciphertext_hash.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        // 2. Allocate private inputs
        let user_secret_var = FpVar::new_witness(cs.clone(), || {
            self.user_secret.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let authorization_note_var = FpVar::new_witness(cs.clone(), || {
            self.authorization_note.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        let mut sibling_vars = Vec::new();
        for sibling in &self.merkle_siblings {
            let var = FpVar::new_witness(cs.clone(), || {
                sibling.ok_or(SynthesisError::AssignmentMissing)
            })?;
            sibling_vars.push(var);
        }
        
        let mut index_vars = Vec::new();
        for index in &self.merkle_indices {
            let var = Boolean::new_witness(cs.clone(), || {
                index.ok_or(SynthesisError::AssignmentMissing)
            })?;
            index_vars.push(var);
        }
        
        // 3. Verify: authorization_note = Poseidon(user_secret)
        let computed_note = poseidon_hash_circuit(&[user_secret_var.clone()], &config)?;
        computed_note.enforce_equal(&authorization_note_var)?;
        
        // 4. Verify Merkle path to policy_root
        let mut current_hash = authorization_note_var;
        for i in 0..sibling_vars.len() {
            let sibling = &sibling_vars[i];
            let is_right = &index_vars[i];
            
            // If index is true (right), then sibling is on the right, current_hash on the left
            // Else sibling is on the left, current_hash on the right
            let left = is_right.select(sibling, &current_hash)?;
            let right = is_right.select(&current_hash, sibling)?;
            
            current_hash = poseidon_hash_circuit(&[left, right], &config)?;
        }
        current_hash.enforce_equal(&policy_root_var)?;
        
        // 5. Bind session_nonce and ciphertext_hash with user_secret
        let binding1 = poseidon_hash_circuit(&[user_secret_var, session_nonce_var], &config)?;
        let _binding2 = poseidon_hash_circuit(&[binding1, ciphertext_hash_var], &config)?;
        
        Ok(())
    }
}
