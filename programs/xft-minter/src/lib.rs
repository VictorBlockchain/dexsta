use anchor_lang::prelude::*;

declare_id!("BPFLoaderUpgradeab1e11111111111111111111111");

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
    /// * 8: unused (formerly redeem days)
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
        address: Pubkey,
    ) -> Result<()> {
        // TODO: Implement logic to mint an XFT with metadata and settings
        Ok(())
    }

    pub fn wrap_xft(_ctx: Context<WrapXft>) -> Result<()> {
        Ok(())
    }

    pub fn transfer_xft(_ctx: Context<TransferXft>) -> Result<()> {
        Ok(())
    }

    pub fn get_xft(_ctx: Context<GetXft>) -> Result<()> {
        Ok(())
    }

    pub fn get_label_owner(_ctx: Context<GetLabelOwner>) -> Result<()> {
        Ok(())
    }

    pub fn check_operator(_ctx: Context<CheckOperator>) -> Result<()> {
        Ok(())
    }

    pub fn is_label_owner(ctx: Context<IsLabelOwner>, address: Pubkey, xft_id: u64) -> Result<bool> {
        // TODO: Replace with actual account fetch logic
        // Simulated fetch: get settings and addresses for xft_id
        let settings: Vec<u64> = vec![0; 8]; // placeholder, should fetch from account
        let addresses: Vec<Pubkey> = vec![Pubkey::default(); 2]; // placeholder, should fetch from account

        // Check if settings[3] (xft type) is not 7 or 8
        let xft_type = settings.get(3).copied().unwrap_or(0);
        if xft_type == 7 || xft_type == 8 {
            return Ok(false);
        }
        // Check if address matches addresses[1] (label owner)
        let label_owner = addresses.get(1).copied().unwrap_or(Pubkey::default());
        if address != label_owner {
            return Ok(false);
        }
        // Check if settings[7] (registration expire) > now
        let expire = settings.get(7).copied().unwrap_or(0);
        let now = Clock::get()?.unix_timestamp as u64;
        if expire <= now {
            return Ok(false);
        }
        Ok(true)
    }
}

// Account structs for Anchor instructions

#[derive(Accounts)]
pub struct MintXft<'info> {
    /// The authority who is minting the XFT
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub xft_mint: AccountInfo<'info>,
    /// The payer for rent/fees
    #[account(mut)]
    pub payer: Signer<'info>,
    /// System program
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WrapXft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub xft_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TransferXft<'info> {
    #[account(mut)]
    pub from: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    pub to: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub xft_account: AccountInfo<'info>,
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