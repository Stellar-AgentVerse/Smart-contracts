use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AccError {
    NegativeAmount = 1,
    BadSignatureOrder = 2,
    UnknownSigner = 3,
    InsufficientBalance = 4,
    Unauthorized = 5,
    Overflow = 6,
    Underflow = 7,
    AlreadyInitialized = 8,
    NotInitialized = 9,
    InvalidAmount = 10,
    ContractPaused = 11,
    NotFound = 12,
    InvalidAddress = 13,
    AllowanceExceeded = 14,
}
