#![cfg(not(target_arch = "bpf"))]

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};

declare_id!("6k8vntYQMbU9AUtnMcypeoS8bf1Ncqv5ZQPqrU3DoH5X");

#[program]
pub mod vault {
    use super::*;

    pub fn vault_cpi_to_minter(_ctx: Context<VaultCpiToMinter>) -> Result<()> {
        Ok(())
    }

    pub fn receive_cpi_from_minter(_ctx: Context<ReceiveCpiFromMinter>) -> Result<()> {
        Ok(())
    }

    pub fn transfer_sol(ctx: Context<TransferSol>, amount: u64) -> Result<()> {
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            ctx.accounts.from.key,
            ctx.accounts.to.key,
            amount,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.from.to_account_info(),
                ctx.accounts.to.to_account_info(),
            ],
        )?;
        Ok(())
    }

    pub fn receive_sol(_ctx: Context<ReceiveSol>) -> Result<()> {
        // No-op: lamports are received by sending to the PDA or account address
        Ok(())
    }

    pub fn transfer_spl(ctx: Context<TransferSpl>, amount: u64) -> Result<()> {
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.from_token.to_account_info(),
            to: ctx.accounts.to_token.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        anchor_spl::token::transfer(cpi_ctx, amount)?;
        Ok(())
    }

    pub fn receive_spl(_ctx: Context<ReceiveSpl>) -> Result<()> {
        // No-op: SPL tokens are received by sending to the token account
        Ok(())
    }
}

#[derive(Accounts)]
pub struct VaultCpiToMinter {}

#[derive(Accounts)]
pub struct ReceiveCpiFromMinter {}

#[derive(Accounts)]
pub struct TransferSol<'info> {
    #[account(mut)]
    pub from: Signer<'info>,
    /// CHECK: Safe, validated by system program
    #[account(mut)]
    pub to: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ReceiveSol {}

#[derive(Accounts)]
pub struct TransferSpl<'info> {
    #[account(mut)]
    pub from_token: Account<'info, anchor_spl::token::TokenAccount>,
    #[account(mut)]
    pub to_token: Account<'info, anchor_spl::token::TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, anchor_spl::token::Token>,
}

#[derive(Accounts)]
pub struct ReceiveSpl<'info> {
    #[account(mut)]
    pub to_token: Account<'info, anchor_spl::token::TokenAccount>,
    pub token_program: Program<'info, anchor_spl::token::Token>,
}
