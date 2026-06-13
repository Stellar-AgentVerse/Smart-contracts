#![no_std]

mod contract;
mod storage {
    pub mod types;
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests;
