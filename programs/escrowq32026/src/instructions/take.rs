use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
    TransferChecked,
};

use crate::{error::ErrorCode, Escrow, ESCROW_SEED};

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,
    /// CHECK: constrained to the maker recorded in the escrow.
    #[account(mut, address = escrow.maker)]
    pub maker: UncheckedAccount<'info>,
    #[account(
        address = escrow.mint_a,
        mint::token_program = token_program
    )]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        address = escrow.mint_b,
        mint::token_program = token_program
    )]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        close = maker,
        has_one = mint_a,
        has_one = mint_b,
        seeds = [ESCROW_SEED, escrow.maker.as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Box<Account<'info, Escrow>>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Take<'info> {
    pub fn take(&mut self) -> Result<()> {
        require!(
            Clock::get()?.unix_timestamp < self.escrow.expiration,
            ErrorCode::EscrowExpired
        );
        require!(
            self.vault.amount > 0 && self.escrow.receive > 0,
            ErrorCode::InvalidAmount
        );

        let transfer_receive = TransferChecked {
            from: self.taker_ata_b.to_account_info(),
            mint: self.mint_b.to_account_info(),
            to: self.maker_ata_b.to_account_info(),
            authority: self.taker.to_account_info(),
        };
        transfer_checked(
            CpiContext::new(self.token_program.key(), transfer_receive),
            self.escrow.receive,
            self.mint_b.decimals,
        )?;

        let seed = self.escrow.seed.to_le_bytes();
        let bump = [self.escrow.bump];
        let signer_seeds: &[&[u8]] = &[ESCROW_SEED, self.escrow.maker.as_ref(), &seed, &bump];
        let transfer_deposit = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.taker_ata_a.to_account_info(),
            authority: self.escrow.to_account_info(),
        };
        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.key(),
                transfer_deposit,
                &[signer_seeds],
            ),
            self.vault.amount,
            self.mint_a.decimals,
        )?;

        let close_vault = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };
        close_account(CpiContext::new_with_signer(
            self.token_program.key(),
            close_vault,
            &[signer_seeds],
        ))
    }
}
