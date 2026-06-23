use clap::{Parser, Subcommand};
use access_prover::proof::{run_setup, run_prove, run_verify};
use access_prover::types::{SessionInput, ProofOutput};

#[derive(Parser)]
#[command(name = "access-prover")]
#[command(about = "ZK Anonymous Access Prover CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate proving and verification keys
    Setup {
        #[arg(long, default_value = "proving_key.bin")]
        pk: String,
        
        #[arg(long, default_value = "verification_key.bin")]
        vk: String,
        
        #[arg(long, default_value = "vk_const.rs")]
        vk_const: String,
    },
    /// Generate a proof of access for a session
    Prove {
        #[arg(long)]
        session: String,
        
        #[arg(long, default_value = "proving_key.bin")]
        pk: String,
        
        #[arg(long, default_value = "proof.json")]
        proof_out: String,
    },
    /// Verify a proof of access
    Verify {
        #[arg(long)]
        proof: String,
        
        #[arg(long, default_value = "verification_key.bin")]
        vk: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Setup { pk, vk, vk_const } => {
            println!("Starting setup...");
            match run_setup(&pk, &vk, Some(&vk_const)) {
                Ok(_) => {
                    println!("Setup completed successfully!");
                    println!("Proving key written to: {}", pk);
                    println!("Verification key written to: {}", vk);
                    println!("Verification key constant written to: {}", vk_const);
                }
                Err(e) => {
                    eprintln!("Error during setup: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Prove { session, pk, proof_out } => {
            println!("Generating proof for session: {}", session);
            
            // Read session JSON
            let session_data = match std::fs::read_to_string(&session) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Failed to read session file: {}", e);
                    std::process::exit(1);
                }
            };
            
            let session_input: SessionInput = match serde_json::from_str(&session_data) {
                Ok(input) => input,
                Err(e) => {
                    eprintln!("Failed to parse session JSON: {}", e);
                    std::process::exit(1);
                }
            };
            
            match run_prove(&pk, &session_input) {
                Ok(proof_output) => {
                    let serialized = serde_json::to_string_pretty(&proof_output).unwrap();
                    if let Err(e) = std::fs::write(&proof_out, serialized) {
                        eprintln!("Failed to write proof output: {}", e);
                        std::process::exit(1);
                    }
                    println!("Proof generated and written to: {}", proof_out);
                }
                Err(e) => {
                    eprintln!("Error generating proof: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Verify { proof, vk } => {
            println!("Verifying proof: {}", proof);
            
            // Read proof JSON
            let proof_data = match std::fs::read_to_string(&proof) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Failed to read proof file: {}", e);
                    std::process::exit(1);
                }
            };
            
            let proof_output: ProofOutput = match serde_json::from_str(&proof_data) {
                Ok(out) => out,
                Err(e) => {
                    eprintln!("Failed to parse proof JSON: {}", e);
                    std::process::exit(1);
                }
            };
            
            match run_verify(&vk, &proof_output) {
                Ok(is_valid) => {
                    if is_valid {
                        println!("Verification RESULT: SUCCESS");
                    } else {
                        println!("Verification RESULT: FAILURE (Invalid Proof)");
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error during verification: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
