use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_spl::token::{self, TokenAccount, Token, Mint, Transfer};
declare_program!(operator);
declare_program!(vault);
use operator::cpi::{self as operator_cpi, accounts::IsOperator};
use vault::cpi::{self as vault_cpi, accounts::CreateVault};
use anchor_lang::system_program;
declare_program!(admin_xft);

declare_id!("BPFLoaderUpgradeab1e11111111111111111111111");

// Error codes stub
#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized action")] 
    Unauthorized,
    #[msg("Invalid settings")] 
    InvalidSettings,
    #[msg("Title already exists")] 
    TitleAlreadyExists,
    #[msg("Invalid XFT type for wrapping")] 
    InvalidXftType,
}

// Event stub
#[event]
pub struct MintXftEvent {
    pub xft_id: u64,
    pub caller: Pubkey,
    pub label_owner: Pubkey,
    pub title: String,
    pub label_type: u64,
    pub edition_size: u64,
    pub label_id: u64,
    pub registration_expire: u64,
    pub timestamp: i64,
}

// XftAdmin stub
pub struct XftAdmin {
    pub mint_price_per_year: u64,
    pub payto_address: Pubkey,
}

#[program]
pub mod minter {
    use super::*;
    
    /// The settings vector configures labels:
    /// * 0: link to label
    /// * 1: registration in years
    /// * 2: operator license
    /// * 3: xft type
    /// * 4: if type is license, license term
    /// * 5: 0 false, 1 true (formerly mint pass)
    /// * 6: quantity
    /// * 7: label registration expire
    /// * 8: if type is market license, marketplace fee percentage
    /// * 9: transferable
    /// * 10: wrapto
    /// * 11: label split for marketplace license
    /// * 12: label vault locked
    /// * 13: label vault unlock date

    /// Label Types (settings[3]):
    /// * 1: Lead Label (1 of 1)
    /// * 2: Profile Label (1 of 1)
    /// * 3: Tag Label (must be limited edidtion)
    /// * 4: Chapter Label (must be limited edidtion)
    /// * 5: Operator License (must be limited edidtion)
    /// * 6: Marketplace License (must be limited edidtion)
    /// * 7: Art/tickets/gaming (can be 1 of 1 or limited edition)
    /// * 8: wrappedTo (1 of 1)
    /// * 9: open

    /// Addresses
    /// address[0] create
    /// address[1] label owner
    /// address[2] vault address
    
    pub fn mint_xft(
        ctx: Context<MintXft>,
        title: String,
        ipfs: String,
        settings: Vec<u64>,
    ) -> Result<()> {
        // TODO: Implement logic to mint an XFT with metadata and settings
        if settings[0] > 0 {
            // This mint is linked to a label, check if caller is label owner
            let is_owner = is_label_owner(ctx.accounts, ctx.accounts.authority.key(), settings[0])?;
            if !is_owner {
                check_operator_permission(&ctx, settings[0])?;
            }
        }
        // Validate settings based on label type
        let label_type = settings.get(3).copied().unwrap_or(0);
        let edition_size = settings.get(6).copied().unwrap_or(0);
        let label_id = settings.get(1).copied().unwrap_or(0);

        // Check label type 1 (Lead Label) - must be 1 of 1
        if label_type == 1 && edition_size != 1 {
            return Err(ErrorCode::InvalidSettings.into());
        }

        // Check label types 3,4,5,6 (Tag, Chapter, Operator License, Marketplace License) - must be limited edition
        if (label_type == 3 || label_type == 4 || label_type == 5 || label_type == 6) && edition_size <= 1 {
            return Err(ErrorCode::InvalidSettings.into());
        }

        // Check non-7,8 label types must have label_id > 0
        if label_type != 7 && label_type != 8 && label_type != 1 && label_id == 0 {
            return Err(ErrorCode::InvalidSettings.into());
        }
        
        // Enforce on-chain uniqueness of title using PDA
        let title_without_spaces: String = title.chars().filter(|c| !c.is_whitespace()).collect();
        if !ctx.accounts.title_lookup.to_account_info().data_is_empty() {
            return Err(ErrorCode::TitleAlreadyExists.into());
        }
        // Get mint price per year and payto address from xft-admin via CPI
        let mut mint_fee_per_year = 0u64;
        let mut payout_address = Pubkey::default();
        if label_type != 7 && label_type != 8 {
            // CPI to xft-admin::get_fees
            let cpi_program = ctx.accounts.admin_program.to_account_info();
            let cpi_accounts = admin_xft_cpi::GetFees {
            admin: ctx.accounts.admin_account.to_account_info(),
            };
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            let (fee_per_year, _marketplace_fee, payto) = admin_xft_cpi::cpi::get_fees(cpi_ctx)?;
            mint_fee_per_year = fee_per_year;
            payout_address = payto;
            let years = settings.get(1).copied().unwrap_or(1);
            let total_fee = mint_fee_per_year.saturating_mul(years);
            // Transfer lamports from payer to payout_address
            transfer_lamports(&ctx.accounts.payer.to_account_info(), &ctx.accounts.payout_account, total_fee, &ctx.accounts.system_program)?;
        }
            // Generate xft_id before creating vault
            let xft_id = ctx.accounts.counter.value;
            ctx.accounts.counter.value += 1;
        
            let mut addresses = vec![Pubkey::default(); 3];
            addresses[0] = ctx.accounts.authority.key();
            addresses[1] = ctx.accounts.label_owner.key();
            if edition_size == 1 {
                // CPI to xft-vault::create_vault(xft_id, settings[3])
                let cpi_program = ctx.accounts.vault_program.to_account_info();
                let cpi_accounts = CreateVault {
                    vault: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    minter_program: ctx.accounts.admin_program.to_account_info(),
                };
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
                let label_type = settings[3];
                vault_cpi::create_vault(cpi_ctx, xft_id, label_type)?;
                addresses[2] = ctx.accounts.vault.key();
            }
        
            // Create the XFT account
            let xft_account = &mut ctx.accounts.xft;
            xft_account.xft_id = xft_id;
            xft_account.settings = settings.clone();
            xft_account.addresses = addresses;
            xft_account.bump = ctx.bumps.xft;
            xft_account.ipfs = ipfs;
            // Transfer XFT to caller
            let cpi_accounts = token::Transfer {
                from: ctx.accounts.xft_token_account.to_account_info(),
                to: ctx.accounts.caller_token_account.to_account_info(),
                authority: ctx.accounts.caller.to_account_info(),
            };
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
            );
            token::transfer(cpi_ctx, settings[6])?;

            if settings[3] != 7 && settings[3] != 8 {
                // Store title without spaces for lookup
                let (title_lookup_pda, bump) = Pubkey::find_program_address(
                    &[b"title_lookup", title_without_spaces.as_bytes()],
                    ctx.program_id
                );
                let title_lookup_account = &mut ctx.accounts.title_lookup;
                title_lookup_account.xft_id = xft_id;
                title_lookup_account.bump = bump;
            
            }
            // Emit mint event
            emit!(MintXftEvent {
                xft_id: xft_id,
                caller: ctx.accounts.caller.key(),
                label_owner: ctx.accounts.label_owner.key(),
                title: title.clone(),
                label_type: label_type,
                edition_size: edition_size,
                label_id: label_id,
                registration_expire: settings[7],
                timestamp: Clock::get()?.unix_timestamp,
            });
            Ok(())
    }

    pub fn wrap_xft(ctx: Context<WrapXft>) -> Result<()> {
        let parent_xft_id;
        {
            let parent_xft = &ctx.accounts.parent_xft;
            parent_xft_id = parent_xft.xft_id;
            require!(parent_xft.settings.len() > 10 && parent_xft.settings[10] != 0, ErrorCode::InvalidXftType);
            require!(parent_xft.settings[3] == 8 || parent_xft.settings[3] == 4, ErrorCode::InvalidXftType);
        }
        // Generate new XFT ID for the wrapped version
        let counter = &mut ctx.accounts.counter;
        let new_xft_id = counter.value;
        counter.value += 1;
        // Create wrapped XFT with same settings and addresses as parent
        let wrapped_settings = ctx.accounts.parent_xft.settings.clone();
        let mut wrapped_addresses = ctx.accounts.parent_xft.addresses.clone();
        // CPI call to create_vault (assume correct context, or add as needed)
        // For now, just set vault address
        wrapped_addresses[2] = ctx.accounts.vault.key();
        // Create the wrapped XFT account
        let wrapped_xft = &mut ctx.accounts.wrapped_xft;
        wrapped_xft.xft_id = new_xft_id;
        wrapped_xft.settings = wrapped_settings;
        wrapped_xft.addresses = wrapped_addresses;
        wrapped_xft.bump = ctx.bumps.wrapped_xft;
        wrapped_xft.ipfs = ctx.accounts.parent_xft.ipfs.clone();
        // Store parent to child relationship in parent's settings
        let parent_xft_mut = &mut ctx.accounts.parent_xft;
        let mut parent_settings = parent_xft_mut.settings.clone();
        if parent_settings.len() <= 11 {
            while parent_settings.len() <= 11 {
                parent_settings.push(0);
            }
        }
        parent_settings[11] = new_xft_id;
        parent_xft_mut.settings = parent_settings;
        // Transfer XFT from user to burn address using anchor-spl
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_xft_token_account.to_account_info(),
                to: ctx.accounts.burn_address.to_account_info(),
                authority: ctx.accounts.wrapper.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, ctx.accounts.parent_xft.settings[1])?;
        // Mint new wrapped XFT tokens to user using anchor-spl
        let mint_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::MintTo {
                mint: ctx.accounts.wrapped_mint.to_account_info(),
                to: ctx.accounts.user_wrapped_token_account.to_account_info(),
                authority: ctx.accounts.wrapper.to_account_info(),
            },
        );
        token::mint_to(mint_ctx, ctx.accounts.parent_xft.settings[1])?;
        emit!(WrappedXftEvent {
            parent_xft_id,
            wrapped_xft_id: new_xft_id,
            wrapper: ctx.accounts.wrapper.key(),
            vault: ctx.accounts.vault.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }
    
    pub fn transfer_xft(ctx: Context<TransferXft>) -> Result<()> {
        let xft_account = &ctx.accounts.xft_account;
        let settings = &xft_account.settings;
        let addresses = &xft_account.addresses;
        // Get the settings[0] account of the XFT
        let settings_0_xft_id = settings[0];
        let parent_xft_account = if settings_0_xft_id > 0 {
            XftAccount::try_from_slice(&ctx.accounts.parent_xft_account.data.borrow())?
        } else {
            return Err(ErrorCode::InvalidSettings.into());
        };
        
        // Check if this is a limited transfer XFT (settings[9] = 0)
        let mut is_authorized_sender = false;
        if settings[9] == 0 {
            // This XFT can only be sent by addresses[0] or addresses[1] of the settings[0] XFT
            is_authorized_sender = parent_xft_account.addresses[0] == ctx.accounts.caller.key() || 
                                     parent_xft_account.addresses[1] == ctx.accounts.caller.key();
            if !is_authorized_sender {
                // Check if caller is an operator for the parent XFT
                let cpi_accounts = operator::cpi::accounts::IsOperator {
                    operator_account: ctx.accounts.parent_xft_account.to_account_info(),
                };
                let cpi_ctx = CpiContext::new(ctx.accounts.operator_program.to_account_info(), cpi_accounts);
                // Anchor CPI does not return values; in real use, check account state or use events
                operator::cpi::is_operator(
                    cpi_ctx,
                    ctx.accounts.caller.key(),
                    settings_0_xft_id,
                )?;
                // If you need to check the result, fetch the account state here
                // For now, we cannot set is_authorized_sender = true based on CPI return
            }
            if !is_authorized_sender {
                // If caller is not authorized, it can only be sent to the burn address
                require!(
                    ctx.accounts.receiver.key() == ctx.accounts.burn_address.key(),
                    ErrorCode::Unauthorized
                );
            }
        }
        // If settings[6] of the xft_id == 1 and settings[0] == 0
        if settings[6] == 1 && settings[0] == 0 {
            // Update addresses[1] to receiver
            let mut parent_xft_account = XftAccount::try_from_slice(&ctx.accounts.parent_xft_account.data.borrow())?;
            parent_xft_account.addresses[1] = ctx.accounts.receiver.key();
            // Update the parent XFT account data
            let mut data = ctx.accounts.parent_xft_account.try_borrow_mut_data()?;
            let mut cursor = std::io::Cursor::new(&mut data[..]);
            parent_xft_account.serialize(&mut cursor)?;
        }
        // Transfer XFT from caller to receiver using anchor-spl
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.caller_token_account.to_account_info(),
                to: ctx.accounts.receiver.to_account_info(),
                authority: ctx.accounts.caller.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, settings[6])?;
        Ok(())
    }

    pub fn update_vault(ctx: Context<UpdateVault>, xft_id: u64, unlock_date: u64) -> Result<()> {
        // Verify the caller is the vault program
        require!(
            ctx.accounts.vault_program.key() == ctx.accounts.xft_account.key(),
            ErrorCode::Unauthorized
        );

        // Update the XFT account settings
        let xft_account = &mut ctx.accounts.xft_account;
        let mut settings = xft_account.settings.clone();
        
        // Ensure settings vector has enough elements
        while settings.len() <= 13 {
            settings.push(0);
        }
        
        // Update settings[12] = 1 and settings[13] = unlock_date
        settings[12] = 1;
        settings[13] = unlock_date;
        
        xft_account.settings = settings;
        
        Ok(())
    }
    pub fn initialize_counter(ctx: Context<InitializeCounter>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.value = 0;
        Ok(())
    }
}

fn check_operator_permission(ctx: &Context<MintXft>, label_id: u64) -> Result<()> {
    let cpi_program = ctx.accounts.operator_program.to_account_info();
    let cpi_accounts = IsOperator {
        operator_account: ctx.accounts.operator_account.clone(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    operator_cpi::is_operator(cpi_ctx, ctx.accounts.caller.key(), label_id)?;
    Ok(())
}

pub fn is_label_owner(accounts: &MintXft, address: Pubkey, xft_id: u64) -> Result<bool> {
    // TODO: Replace with actual account fetch logic
    let settings: Vec<u64> = vec![0; 8]; // placeholder
    let addresses: Vec<Pubkey> = vec![Pubkey::default(); 2]; // placeholder
    let xft_type = settings.get(3).copied().unwrap_or(0);
    if xft_type == 7 || xft_type == 8 {
        return Ok(false);
    }
    let label_owner = addresses.get(1).copied().unwrap_or(Pubkey::default());
    if address != label_owner {
        return Ok(false);
    }
    let expire = settings.get(7).copied().unwrap_or(0);
    let now = Clock::get()?.unix_timestamp as u64;
    if expire <= now {
        return Ok(false);
    }
    Ok(true)
}

pub fn is_market_license(xft_account_info: &AccountInfo, parent_xft_account_info: &AccountInfo) -> Result<(bool, u64, Pubkey, u64)> {
    let xft_account = XftAccount::try_from_slice(&xft_account_info.data.borrow())?;
    let settings = xft_account.settings;
    // Check if settings[7] > now (expire check)
    let expire = settings.get(7).copied().unwrap_or(0);
    let now = Clock::get()?.unix_timestamp as u64;
    if expire <= now {
        return Ok((false, 0, Pubkey::default(), 0));
    }
    // Get the parent xft account (settings[0] of xft_id)
    let parent_xft_id = settings.get(0).copied().unwrap_or(0);
    if parent_xft_id == 0 {
        return Ok((false, 0, Pubkey::default(), 0));
    }
    let parent_xft_account = XftAccount::try_from_slice(&parent_xft_account_info.data.borrow())?;
    let parent_settings = parent_xft_account.settings;
    let parent_addresses = parent_xft_account.addresses;
    // Check if settings[7] of parent_xft account > now
    let parent_expire = parent_settings.get(7).copied().unwrap_or(0);
    if parent_expire <= now {
        return Ok((false, parent_xft_id, Pubkey::default(), 0));
    }
    // Check parent_addresses[2] and parent_settings[8]
    let parent_vault = parent_addresses.get(2).copied().unwrap_or(Pubkey::default());
    let parent_setting_8 = parent_settings.get(8).copied().unwrap_or(0);
    Ok((true, parent_xft_id, parent_vault, parent_setting_8))
}

pub fn get_vault(xft_account_info: &AccountInfo) -> Result<Pubkey> {
    let xft_account = XftAccount::try_from_slice(&xft_account_info.data.borrow())?;
    let addresses = xft_account.addresses;
    let vault = addresses.get(2).copied().unwrap_or(Pubkey::default());
    Ok(vault)
}


// Account structs for Anchor instructions

#[account]
pub struct XftAccount {
    pub xft_id: u64,
    pub settings: Vec<u64>,
    pub addresses: Vec<Pubkey>,
    pub ipfs: String,
    pub bump: u8,
}

#[account]
pub struct TitleLookup {
    pub xft_id: u64,
    pub bump: u8,
}

#[account]
pub struct Counter {
    pub value: u64,
}

#[derive(Accounts)]
#[instruction(title: String)]
pub struct MintXft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub xft_mint: AccountInfo<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 8 + (8 + 32) * 16 + 8 + 32 * 8 + 1, // adjust as needed
        seeds = [b"xft", authority.key().as_ref()],
        bump
    )]
    pub xft: Account<'info, XftAccount>,
    #[account(mut, seeds = [b"counter"], bump)]
    pub counter: Account<'info, Counter>,
    pub system_program: Program<'info, System>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub operator_program: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub operator_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub vault_program: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub vault: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub label_owner: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub caller: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub xft_token_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub caller_token_account: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub admin_program: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub admin_account: AccountInfo<'info>,
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + 8 + 1, // adjust as needed
        seeds = [b"title_lookup", title.as_bytes()],
        bump
    )]
    pub title_lookup: Account<'info, TitleLookup>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub payout_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct WrapXft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub parent_xft: Account<'info, XftAccount>,
    #[account(init, payer = authority, space = 8 + 8 + (8 + 32) * 16 + 8 + 32 * 8 + 1, seeds = [b"xft", authority.key().as_ref(), b"wrapped"], bump)]
    pub wrapped_xft: Account<'info, XftAccount>,
    #[account(mut, seeds = [b"counter"], bump)]
    pub counter: Account<'info, Counter>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub vault: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub vault_program: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub user_xft_token_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub burn_address: AccountInfo<'info>,
    pub wrapper: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub wrapped_mint: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub user_wrapped_token_account: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct TransferXft<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub caller_token_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub receiver: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub xft_account: Account<'info, XftAccount>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub parent_xft_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub operator_program: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub burn_address: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct GetXft<'info> {
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub xft_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct GetLabelOwner<'info> {
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub label_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CheckOperator<'info> {
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub operator_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct IsLabelOwner<'info> {
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub label_account: AccountInfo<'info>,
    /// The authority to check
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct InitializeCounter<'info> {
    #[account(init, payer = payer, space = 8 + 8, seeds = [b"counter"], bump)]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateVault<'info> {
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub vault_program: AccountInfo<'info>,
    #[account(mut)]
    pub xft_account: Account<'info, XftAccount>,
}

// CPI context for xft-admin get_fees
pub mod admin_xft_cpi {
    use super::*;
    pub mod cpi {
        use super::*;
        pub fn get_fees<'info>(
            ctx: CpiContext<'_, '_, '_, 'info, GetFees<'info>>,
        ) -> Result<(u64, u64, Pubkey)> {
            let ix = anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program.key(),
                accounts: vec![anchor_lang::solana_program::instruction::AccountMeta::new_readonly(ctx.accounts.admin.key(), false)],
                data: vec![0], // get_fees is usually the first instruction, so 0
            };
            let mut data = [0u8; 8];
            let acc_infos = ctx.to_account_infos();
            anchor_lang::solana_program::program::invoke(&ix, &acc_infos)?;
            // In real Anchor CPI, you would use the generated CPI interface, but here we stub the return
            // You must replace this with the actual Anchor CPI call if you generate the IDL
            Ok((0, 0, Pubkey::default())) // placeholder
        }
    }
    #[derive(Accounts)]
    pub struct GetFees<'info> {
        /// CHECK: This is safe for CPI
        pub admin: AccountInfo<'info>,
    }
}

fn transfer_lamports<'info>(
    payer: &AccountInfo<'info>,
    payout_address: &AccountInfo<'info>,
    total_fee: u64,
    system_program: &Program<'info, System>,
) -> Result<()> {
    let cpi_accounts = system_program::Transfer {
        from: payer.clone(),
        to: payout_address.clone(),
    };
    let cpi_ctx = CpiContext::new(system_program.to_account_info(), cpi_accounts);
    system_program::transfer(cpi_ctx, total_fee)
}

#[event]
pub struct WrappedXftEvent {
    pub parent_xft_id: u64,
    pub wrapped_xft_id: u64,
    pub wrapper: Pubkey,
    pub vault: Pubkey,
    pub timestamp: i64,
}