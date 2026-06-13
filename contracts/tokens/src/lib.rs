#![no_std]

mod contract;
mod core {
    pub mod token;
}
mod events;
mod storage {
    pub mod types;
}

// Re-export contract for tests
pub use contract::{MyToken, MyTokenClient};

#[cfg(test)]
mod tests;
