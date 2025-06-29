#![cfg(not(target_arch = "bpf"))]

use anchor_lang::prelude::*;
use minter::cpi::accounts::IsLabelOwner;
use minter::program::Minter;

declare_id!("CvEyB4XdT5nBiGfCK1vW8eSuuAW7o9EZ8v7dFwafZ6P3");

// Fallback operator check logic (outside #[program] mod)
pub fn is_operator_fallback(_address: Pubkey, _xft_id: u64) -> Result<bool> {
    // TODO: Implement actual logic
    Ok(false)
}

#[program]
pub mod operator {
    use super::*;
    use minter::cpi::accounts::IsLabelOwner;

    pub fn receive_cpi_from_vault(_ctx: Context<ReceiveCpiFromVault>) -> Result<()> {
        // This will be called by the vault program via CPI
        Ok(())
    }

    pub fn receive_cpi_from_admin_xft(_ctx: Context<ReceiveCpiFromAdminXft>) -> Result<()> {
        // This will be called by the admin_xft program via CPI
        Ok(())
    }

    pub fn receive_cpi_from_minter(_ctx: Context<ReceiveCpiFromMinter>) -> Result<()> {
        // This will be called by the minter program via CPI
        Ok(())
    }

    pub fn cpi_to_minter(_ctx: Context<CpiToMinter>) -> Result<()> {
        // This will make a CPI call to the minter program (to be implemented)
        Ok(())
    }
    ///settings[0] = license
    ///settings[1] = access expire
    ///settings[2] = role (1 = super operator, can add other operators)
    ///setings[3] = next withdraw date (time stamp)
    ///settings[4] = max solana withdraw amount
    //settings[5] = withdraw frequency (in days: ie every 7 days)
    pub fn add_operator(ctx: Context<AddOperator>, operator: Pubkey, xft_id: u64, settings: Vec<u64>) -> Result<()> {
        // CPI call to xft-minter::is_label_owner
        let minter_program = ctx.accounts.xft_minter_program.to_account_info();
        let cpi_accounts = IsLabelOwner {
            label_account: ctx.accounts.label_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(minter_program, cpi_accounts);
        let is_owner = minter::cpi::is_label_owner(cpi_ctx, ctx.accounts.authority.key(), xft_id)?.get();
        
        let mut allowed = is_owner;
        if !allowed {
            // Check if authority is already an operator with role = 1 (super operator)
            let mut is_op_accounts = IsOperator {
                operator_account: ctx.accounts.operator_account.to_account_info(),
            };
            let is_op_ctx = Context::new(
                ctx.program_id,
                &mut is_op_accounts,
                &[],
                Default::default(),
            );
            let (is_op, _license, role, _next_withdraw, _max_withdraw) = operator::is_operator(
                is_op_ctx,
                ctx.accounts.authority.key(),
                xft_id,
            )?;
            allowed = is_op && role == 1;
            if !allowed {
                allowed = is_operator_fallback(ctx.accounts.authority.key(), xft_id)?;
            }
        }
        require!(allowed, OperatorError::NotAuthorized);
        // Store the operator mapping (operator -> xft_id)
        let _operator_account = OperatorAccount::try_from_init(
            &mut ctx.accounts.operator_account,
            &operator,
            xft_id,
            settings.clone(),
        )?;
        emit!(OperatorAdded {
            operator,
            xft_id,
            settings,
            authority: ctx.accounts.authority.key(),
        });
        Ok(())
    }

    pub fn is_operator(ctx: Context<IsOperator>, address: Pubkey, xft_id: u64) -> Result<(bool, u64, u64, u64, u64)> {

        // Fetch the operator account for (address, xft_id)
        let operator_account = OperatorAccount::fetch(&ctx.accounts.operator_account, &address, xft_id)?;
        let settings = operator_account.settings;
        // settings[1] = access expire
        let access_expire = settings.get(1).copied().unwrap_or(0);
        let now = Clock::get()?.unix_timestamp as u64;
        if access_expire > now {
            // settings[0] = license, settings[2] = role, settings[3] = next withdraw date, settings[4] = max solana withdraw amount
            let license = settings.get(0).copied().unwrap_or(0);
            let role = settings.get(2).copied().unwrap_or(0);
            let next_withdraw = settings.get(3).copied().unwrap_or(0);
            let max_withdraw = settings.get(4).copied().unwrap_or(0);
            Ok((true, license, role, next_withdraw, max_withdraw))
        } else {
            Ok((false, 0, 0, 0, 0))
        }
    }
    
    pub fn remove_operator(ctx: Context<RemoveOperator>, operator: Pubkey, xft_id: u64) -> Result<()> {
        // CPI call to xft-minter::is_label_owner
        let minter_program = ctx.accounts.xft_minter_program.to_account_info();
        let cpi_accounts = IsLabelOwner {
            label_account: ctx.accounts.label_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(minter_program, cpi_accounts);
        let is_owner = minter::cpi::is_label_owner(cpi_ctx, ctx.accounts.authority.key(), xft_id)?.get();
        
        let mut allowed = is_owner;
        if !allowed {
            // Check if authority is already an operator with role = 1 (super operator)
            let mut is_op_accounts = IsOperator {
                operator_account: ctx.accounts.operator_account.to_account_info(),
            };
            let is_op_ctx = Context::new(
                ctx.program_id,
                &mut is_op_accounts,
                &[],
                Default::default(),
            );
            let (is_op, _license, role, _next_withdraw, _max_withdraw) = operator::is_operator(
                is_op_ctx,
                ctx.accounts.authority.key(),
                xft_id,
            )?;
            allowed = is_op && role == 1;
            if !allowed {
                allowed = is_operator_fallback(ctx.accounts.authority.key(), xft_id)?;
            }
        }
        require!(allowed, OperatorError::NotAuthorized);
        // Fetch the operator account for (operator, xft_id)
        let mut operator_account = OperatorAccount::fetch(&ctx.accounts.operator_account, &operator, xft_id)?;
        // Set settings[2] = 0 (role), settings[1] = 0 (access expire), settings[0] = 0 (license)
        if operator_account.settings.len() > 2 {
            operator_account.settings[2] = 0;
        }
        if operator_account.settings.len() > 1 {
            operator_account.settings[1] = 0;
        }
        if operator_account.settings.len() > 0 {
            operator_account.settings[0] = 0;
        }
        // Save the updated settings (mock/prototype)
        OperatorAccount::try_from_init(
            &mut ctx.accounts.operator_account,
            &operator,
            xft_id,
            operator_account.settings.clone(),
        )?;
        Ok(())
    }

    pub fn edit_withdraw_settings(ctx: Context<EditWithdrawSettings>, operator: Pubkey, xft_id: u64, withdraw_frequency: u64, max_sol_amount: u64) -> Result<()> {
                // CPI call to xft-minter::is_label_owner
        let minter_program = ctx.accounts.xft_minter_program.to_account_info();
        let cpi_accounts = IsLabelOwner {
            label_account: ctx.accounts.label_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(minter_program, cpi_accounts);
        let is_owner = minter::cpi::is_label_owner(cpi_ctx, ctx.accounts.authority.key(), xft_id)?.get();
        
        let mut allowed = is_owner;
        if !allowed {
            // Check if authority is already an operator with role = 1 (super operator)
            let mut is_op_accounts = IsOperator {
                operator_account: ctx.accounts.operator_account.to_account_info(),
            };
            let is_op_ctx = Context::new(
                ctx.program_id,
                &mut is_op_accounts,
                &[],
                Default::default(),
            );
            let (is_op, _license, role, _next_withdraw, _max_withdraw) = operator::is_operator(
                is_op_ctx,
                ctx.accounts.authority.key(),
                xft_id,
            )?;
            allowed = is_op && role == 1;
            if !allowed {
                allowed = is_operator_fallback(ctx.accounts.authority.key(), xft_id)?;
            }
        }
        require!(allowed, OperatorError::NotAuthorized);
        // Fetch the operator account for (operator, xft_id)
        let mut operator_account = OperatorAccount::fetch(&ctx.accounts.operator_account, &operator, xft_id)?;
        // Update settings[5] = withdraw_frequency, settings[4] = max_sol_amount
        if operator_account.settings.len() > 5 {
            operator_account.settings[5] = withdraw_frequency;
        } else {
            // Extend settings if needed
            while operator_account.settings.len() <= 5 {
                operator_account.settings.push(0);
            }
            operator_account.settings[5] = withdraw_frequency;
        }
        if operator_account.settings.len() > 4 {
            operator_account.settings[4] = max_sol_amount;
        } else {
            while operator_account.settings.len() <= 4 {
                operator_account.settings.push(0);
            }
            operator_account.settings[4] = max_sol_amount;
        }
        // Save the updated settings (mock/prototype)
        OperatorAccount::try_from_init(
            &mut ctx.accounts.operator_account,
            &operator,
            xft_id,
            operator_account.settings.clone(),
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct ReceiveCpiFromVault {}

#[derive(Accounts)]
pub struct ReceiveCpiFromAdminXft {}

#[derive(Accounts)]
pub struct ReceiveCpiFromMinter {}

#[derive(Accounts)]
pub struct CpiToMinter {}

#[derive(Accounts)]
pub struct AddOperator<'info> {
    /// The user attempting to add an operator (must be label owner or operator)
    #[account(signer)]
    pub authority: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub label_account: AccountInfo<'info>,
    /// The xft-minter program for CPI
    pub xft_minter_program: Program<'info, Minter>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub operator_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct IsOperator<'info> {
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub operator_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RemoveOperator<'info> {
    #[account(signer)]
    pub authority: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub label_account: AccountInfo<'info>,
    pub xft_minter_program: Program<'info, Minter>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub operator_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct EditWithdrawSettings<'info> {
    #[account(signer)]
    pub authority: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub label_account: AccountInfo<'info>,
    pub xft_minter_program: Program<'info, Minter>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub operator_account: AccountInfo<'info>,
}

#[error_code]
pub enum OperatorError {
    #[msg("Not authorized to add operator for this xft_id")] 
    NotAuthorized,
}

#[event]
pub struct OperatorAdded {
    pub operator: Pubkey,
    pub xft_id: u64,
    pub settings: Vec<u64>,
    pub authority: Pubkey,
}

// Helper for fetching operator account (mock/prototype)
pub struct OperatorAccount {
    pub settings: Vec<u64>,
}
impl OperatorAccount {
    pub fn fetch(_account: &AccountInfo, _address: &Pubkey, _xft_id: u64) -> Result<Self> {
        // TODO: Replace with actual deserialization logic
        // For now, return a dummy account with example settings
        Ok(OperatorAccount {
            settings: vec![0, u64::MAX, 1, 1234567890, 1000], // [license, access_expire, role, next_withdraw, max_withdraw]
        })
    }
    pub fn try_from_init(_account: &mut AccountInfo, _operator: &Pubkey, _xft_id: u64, settings: Vec<u64>) -> Result<Self> {
        // TODO: Implement actual account init logic
        Ok(OperatorAccount { settings })
    }
}
