pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("5bewKCQhwLDMb9SGAW2KTCkqvNMNZziuMTno2tkrcz2G");

// Two parties — a maker and a taker — can swap tokens without trusting each other or a third party.
// The maker deposits token A into a program-controlled vault, specifies how much of token B they want,
// and sets a timed claim window. A taker can complete the swap during that window; after it closes,
// the maker can reclaim their tokens.

// Maker deposits token A  →  vault (PDA-owned)
//                                       ↓  taker sends token B to maker
//                                       ↓  vault releases token A to taker
//                                       ↓  escrow + vault accounts closed, rent returned

#[program]
pub mod escrowq32026 {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn make(
        ctx: Context<Make>,
        seed: u64,
        deposit: u64,
        receive: u64,
        unlock_at: i64,
        expiration: i64,
    ) -> Result<()> {
        ctx.accounts
            .init_escrow(seed, receive, &ctx.bumps, unlock_at, expiration)?;
        ctx.accounts.deposit(deposit)
    }

    #[instruction(discriminator = 1)]
    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.take()
    }

    #[instruction(discriminator = 2)]
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund_and_close_vault()
    }

    #[instruction(discriminator = 3)]
    pub fn update(
        ctx: Context<Update>,
        receive: u64,
        unlock_at: i64,
        expiration: i64,
    ) -> Result<()> {
        ctx.accounts.update(receive, unlock_at, expiration)
    }
}
