use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("The escrow has expired")]
    EscrowExpired,
    #[msg("The escrow claim window has not opened")]
    EscrowNotUnlocked,
    #[msg("The escrow has not expired yet")]
    EscrowNotExpired,
    #[msg("Escrow terms can only be updated before the claim window opens")]
    UpdateWindowClosed,
    #[msg("Amounts must be greater than zero")]
    InvalidAmount,
    #[msg("The unlock time and expiration must define a valid future window")]
    InvalidTimeWindow,
}
