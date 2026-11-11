use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("The escrow has expired")]
    EscrowExpired,
    #[msg("Amounts must be greater than zero")]
    InvalidAmount,
    #[msg("The expiration must be in the future")]
    InvalidExpiration,
}
