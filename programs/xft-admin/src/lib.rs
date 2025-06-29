#![cfg(not(target_arch = "bpf"))]

use anchor_lang::prelude::*;

declare_id!("Admin111111111111111111111111111111111111111");

#[program]
pub mod admin_xft {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>, platform_xft_in: u64, payout_address: Pubkey, mint_fee_per_year: u64, marketplace_fee_sol: u64, marketplace_fee_dexsta: u64, dexsta_address: Pubkey) -> Result<()> {
        let admin = &mut ctx.accounts.admin;
        admin.platform_xft_in = platform_xft_in;
        admin.payout_address = payout_address;
        admin.mint_fee_per_year = mint_fee_per_year;
        admin.marketplace_fee_sol = marketplace_fee_sol;
        admin.marketplace_fee_dexsta = marketplace_fee_dexsta;
        admin.dexsta_address = dexsta_address;
        admin.bump = ctx.bumps.admin;
        Ok(())
    }
    pub fn set_fees(ctx: Context<SetFees>, mint_fee_per_year: u64, marketplace_fee_sol: u64, marketplace_fee_dexsta: u64) -> Result<()> {
        let admin = &mut ctx.accounts.admin;
        require!(is_super_operator(ctx.accounts.admin_operator.as_ref(), admin.platform_xft_in), AdminError::Unauthorized);
        admin.mint_fee_per_year = mint_fee_per_year;
        admin.marketplace_fee_sol = marketplace_fee_sol;
        admin.marketplace_fee_dexsta = marketplace_fee_dexsta;
        Ok(())
    }
    pub fn set_payout_address(ctx: Context<SetPayoutAddress>, payout_address: Pubkey) -> Result<()> {
        let admin = &mut ctx.accounts.admin;
        require!(is_super_operator(ctx.accounts.admin_operator.as_ref(), admin.platform_xft_in), AdminError::Unauthorized);
        admin.payout_address = payout_address;
        Ok(())
    }
    pub fn get_fees(ctx: Context<GetFees>) -> Result<(u64, u64, u64, Pubkey, Pubkey)> {
        let admin = &ctx.accounts.admin;
        Ok((admin.mint_fee_per_year, admin.marketplace_fee_sol, admin.marketplace_fee_dexsta, admin.dexsta_address, admin.payout_address))
    }
}

#[account]
pub struct AdminXFT {
    pub platform_xft_in: u64,
    pub payout_address: Pubkey,
    pub mint_fee_per_year: u64,
    pub marketplace_fee_sol: u64,
    pub marketplace_fee_dexsta: u64,
    pub dexsta_address: Pubkey,
    pub bump: u8,
}

impl AdminXFT {
    pub const LEN: usize = 8 + 32 + 8 + 8 + 1;
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = 8 + 8 + 32 + 8 + 8 + 1, seeds = [b"admin_xft"], bump)]
    pub admin: Account<'info, AdminXFT>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetFees<'info> {
    #[account(mut, seeds = [b"admin_xft"], bump = admin.bump)]
    pub admin: Account<'info, AdminXFT>,
    /// CHECK: Must be checked in handler
    pub admin_operator: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct SetPayoutAddress<'info> {
    #[account(mut, seeds = [b"admin_xft"], bump = admin.bump)]
    pub admin: Account<'info, AdminXFT>,
    /// CHECK: Must be checked in handler
    pub admin_operator: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct GetFees<'info> {
    #[account(seeds = [b"admin_xft"], bump = admin.bump)]
    pub admin: Account<'info, AdminXFT>,
}

#[error_code]
pub enum AdminError {
    #[msg("Unauthorized: Only super operators linked to the platform XFT can update fees or payout address")] 
    Unauthorized,
}

fn is_super_operator(_admin_operator: &AccountInfo, _platform_xft_in: u64) -> bool {
    // TODO: Implement CPI to operator program to check if admin_operator is a super operator for platform_xft_in
    true // placeholder
}