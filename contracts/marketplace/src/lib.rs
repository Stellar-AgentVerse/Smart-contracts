#![no_std]

mod contract;
pub mod privacy;
mod storage {
    pub mod types;
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests;
