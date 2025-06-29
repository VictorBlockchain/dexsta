use anchor_lang::prelude::*;
use anchor_lang::{AnchorSerialize, AnchorDeserialize};
declare_id!("6k8vntYQMbU9AUtnMcypeoS8bf1Ncqv5ZQPqrU3DoH5X");
declare_program!(minter);
declare_program!(operator);
use minter::program::Minter;
use operator::program::Operator;

#[program]
pub mod xft_vault {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn create_vault(
        ctx: Context<CreateVault>,
        xft_id: u64,
        xft_type: u64,
    ) -> Result<Pubkey> {
        // Only allow CPI from xft-minter
        let minter_program = ctx.accounts.minter_program.key();
        require!(*ctx.program_id == minter_program, VaultError::Unauthorized);
        let vault = &mut ctx.accounts.vault;
        vault.xft_id = xft_id;
        vault.xft_type = xft_type;
        // Set unlock_date to a default value, e.g., 0 or current timestamp if needed
        vault.unlock_date = 0;
        Ok(vault.key())
    }

    pub fn withdraw_sol(ctx: Context<WithdrawSol>, xft_id: u64, amount: u64) -> Result<()> {
        let authority = ctx.accounts.authority.key();
        let minter_program = ctx.accounts.minter_program.to_account_info();
        let operator_program = ctx.accounts.operator_program.to_account_info();
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        require!(ctx.accounts.vault.xft_id == xft_id, VaultError::Unauthorized);
        require!(ctx.accounts.vault.unlock_date < now, VaultError::WithdrawTooSoon);
        // Only perform owner/operator checks if xft_type != 7 and != 8
        if ctx.accounts.vault.xft_type != 7 && ctx.accounts.vault.xft_type != 8 {
            // CPI: is_label_owner
            let is_label_owner_ix = minter::cpi::accounts::IsLabelOwner {
                authority: ctx.accounts.authority.to_account_info(),
                label_account: ctx.accounts.vault.to_account_info(), // placeholder, replace with actual label account if needed
            };
            let is_label_owner_ctx = CpiContext::new(minter_program.clone(), is_label_owner_ix);
            let is_owner = minter::cpi::is_label_owner(is_label_owner_ctx, authority, xft_id)?.get();
            if is_owner {
                process_sol_transfer(&ctx, amount)?;
                return Ok(());
            }
            // CPI: is_operator
            let is_operator_ctx = CpiContext::new(
                operator_program.clone(),
                operator::cpi::accounts::IsOperator {
                    operator_account: ctx.accounts.vault.to_account_info(),
                },
            );
            // Instead of expecting a return value, fetch the operator account data directly
            let operator_account = OperatorAccount::fetch(&ctx.accounts.vault.to_account_info(), &authority, xft_id)?;
            let settings = operator_account.settings;
            let access_expire = settings.get(1).copied().unwrap_or(0);
            let now = clock.unix_timestamp as u64;
            let (is_op, _license, _role, next_withdraw, max_withdraw) = if access_expire > now {
                let license = settings.get(0).copied().unwrap_or(0);
                let role = settings.get(2).copied().unwrap_or(0);
                let next_withdraw = settings.get(3).copied().unwrap_or(0);
                let max_withdraw = settings.get(4).copied().unwrap_or(0);
                (true, license, role, next_withdraw, max_withdraw)
            } else {
                (false, 0, 0, 0, 0)
            };
            require!(is_op, VaultError::Unauthorized);
            require!(next_withdraw < now as u64, VaultError::WithdrawTooSoon);
            require!(max_withdraw >= amount, VaultError::WithdrawTooMuch);
            process_sol_transfer(&ctx, amount)?;
        }
        // CPI: update_next_withdraw
        let update_next_withdraw_ctx = CpiContext::new(
            operator_program,
            operator::cpi::accounts::UpdateNextWithdraw {
                vault_signer: ctx.accounts.vault.to_account_info(),
                operator_account: ctx.accounts.vault.to_account_info(),
            },
        );
        operator::cpi::update_next_withdraw(update_next_withdraw_ctx, authority, xft_id)?;
        Ok(())
    }

       pub fn withdraw_spl(ctx: Context<WithdrawSpl>, xft_id: u64) -> Result<()> {
        let authority = ctx.accounts.authority.key();
        let minter_program = ctx.accounts.minter_program.to_account_info();
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        require!(ctx.accounts.vault.xft_id == xft_id, VaultError::Unauthorized);
        require!(ctx.accounts.vault.unlock_date < now, VaultError::WithdrawTooSoon);
                if ctx.accounts.vault.xft_type != 7 && ctx.accounts.vault.xft_type != 8 {
        
        // CPI: is_label_owner
        let is_label_owner_ix = minter::cpi::accounts::IsLabelOwner {
            authority: ctx.accounts.authority.to_account_info(),
            label_account: ctx.accounts.vault.to_account_info(), // placeholder
        };
        let is_label_owner_ctx = CpiContext::new(minter_program, is_label_owner_ix);
        let is_owner = minter::cpi::is_label_owner(is_label_owner_ctx, authority, xft_id)?.get();
        require!(is_owner, VaultError::Unauthorized);
        }
        // TODO: Implement SPL withdrawal logic
        Ok(())
    }
    
    pub fn withdraw_xft(ctx: Context<WithdrawXft>, xft_id: u64) -> Result<()> {
        let authority = ctx.accounts.authority.key();
        let minter_program = ctx.accounts.minter_program.to_account_info();
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        require!(ctx.accounts.vault.xft_id == xft_id, VaultError::Unauthorized);
        require!(ctx.accounts.vault.unlock_date < now, VaultError::WithdrawTooSoon);
        let mut is_owner = true;
        if ctx.accounts.vault.xft_type != 7 && ctx.accounts.vault.xft_type != 8 {
            // CPI: is_label_owner
            let is_label_owner_ix = minter::cpi::accounts::IsLabelOwner {
                authority: ctx.accounts.authority.to_account_info(),
                label_account: ctx.accounts.vault.to_account_info(), // placeholder
            };
            let is_label_owner_ctx = CpiContext::new(minter_program, is_label_owner_ix);
            is_owner = minter::cpi::is_label_owner(is_label_owner_ctx, authority, xft_id)?.get();
        }
        require!(is_owner, VaultError::Unauthorized);
        // Allow withdrawal if label owner
        // TODO: Implement XFT withdrawal logic
        Ok(())
    }
    
    pub fn lock_vault(ctx: Context<LockVault>, xft_id: u64, unlock_date: i64) -> Result<()> {
        let authority = ctx.accounts.authority.key();
        let minter_program = ctx.accounts.minter_program.to_account_info();
        // Only perform owner check if xft_type != 7 and != 8
        if ctx.accounts.vault.xft_type != 7 && ctx.accounts.vault.xft_type != 8 {
            // CPI: is_label_owner
            let is_label_owner_ix = minter::cpi::accounts::IsLabelOwner {
            authority: ctx.accounts.authority.to_account_info(),
            label_account: ctx.accounts.vault.to_account_info(), // placeholder
            };
            let is_label_owner_ctx = CpiContext::new(minter_program, is_label_owner_ix);
            let is_owner = minter::cpi::is_label_owner(is_label_owner_ctx, authority, xft_id)?.get();
            require!(is_owner, VaultError::Unauthorized);
        }
        let vault = &mut ctx.accounts.vault;
        vault.unlock_date = unlock_date;
        // TODO: Implement lock logic (e.g., set unlock_date far in the future or a locked flag)
        Ok(())
    }

}

#[account]
pub struct Vault {
    pub xft_id: u64,
    pub xft_type: u64, // Added field
    pub unlock_date: i64,
    // Add more fields as needed
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
#[instruction(xft_id: u64)]
pub struct CreateVault<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 8 + 8,
        seeds = [b"vault", xft_id.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: This is the xft-minter program, used for CPI only, not deserialized
    pub minter_program: AccountInfo<'info>,
}

// Add #[derive(Accounts)] with #[instruction(xft_id: u64)] for PDA seeds
#[derive(Accounts)]
#[instruction(xft_id: u64)]
pub struct WithdrawSol<'info> {
    #[account(
        mut,
        seeds = [b"vault", xft_id.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: This is the xft-minter program, used for CPI only, not deserialized
    pub minter_program: AccountInfo<'info>,
    /// CHECK: This is the xft-operator program, used for CPI only, not deserialized
    pub operator_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(xft_id: u64)]
pub struct WithdrawSpl<'info> {
    #[account(
        mut,
        seeds = [b"vault", xft_id.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: This is the xft-minter program, used for CPI only, not deserialized
    pub minter_program: AccountInfo<'info>,
    // Add token accounts as needed
}

#[derive(Accounts)]
#[instruction(xft_id: u64)]
pub struct WithdrawXft<'info> {
    #[account(
        mut,
        seeds = [b"vault", xft_id.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: This is the xft-minter program, used for CPI only, not deserialized
    pub minter_program: AccountInfo<'info>,
    // Add XFT token accounts as needed
}

#[derive(Accounts)]
#[instruction(xft_id: u64)]
pub struct LockVault<'info> {
    #[account(
        mut,
        seeds = [b"vault", xft_id.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: This is the xft-minter program, used for CPI only, not deserialized
    pub minter_program: AccountInfo<'info>,
}

// Add OperatorAccount struct for operator checks
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct OperatorAccount {
    pub settings: Vec<u64>,
}
impl OperatorAccount {
    pub fn fetch(account: &AccountInfo, _address: &Pubkey, _xft_id: u64) -> Result<Self> {
        let data = account.try_borrow_data()?;
        OperatorAccount::deserialize(&mut &data[..])
            .map_err(|_| error!(VaultError::Unauthorized))
    }
}

fn process_sol_transfer(ctx: &Context<WithdrawSol>, amount: u64) -> Result<()> {
    let vault_account_info = ctx.accounts.vault.to_account_info();
    let authority_account_info = ctx.accounts.authority.to_account_info();
    let system_program = ctx.accounts.system_program.to_account_info();
    let ix = anchor_lang::solana_program::system_instruction::transfer(
        vault_account_info.key,
        authority_account_info.key,
        amount,
    );
    anchor_lang::solana_program::program::invoke(
        &ix,
        &[
            vault_account_info.clone(),
            authority_account_info.clone(),
            system_program.clone(),
        ],
    )?;
    Ok(())
}

#[error_code]
pub enum VaultError {
    #[msg("Only the xft-minter program can call this instruction")] 
    Unauthorized,
    #[msg("Withdraw too soon")] 
    WithdrawTooSoon,
    #[msg("Withdraw amount too high")] 
    WithdrawTooMuch,
}
