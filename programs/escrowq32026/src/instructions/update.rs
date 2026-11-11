use anchor_lang::prelude::*;

use crate::{error::ErrorCode, Escrow, ESCROW_SEED};

#[derive(Accounts)]
pub struct Update<'info> {
    pub maker: Signer<'info>,
    #[account(
        mut,
        has_one = maker,
        seeds = [ESCROW_SEED, maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,
}

impl<'info> Update<'info> {
    pub fn update(&mut self, receive: u64, expiration: i64) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(now < self.escrow.expiration, ErrorCode::EscrowExpired);
        require!(receive > 0, ErrorCode::InvalidAmount);
        require!(expiration > now, ErrorCode::InvalidExpiration);

        self.escrow.receive = receive;
        self.escrow.expiration = expiration;
        Ok(())
    }
}
