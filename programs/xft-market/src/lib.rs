use anchor_lang::prelude::*;
declare_program!(minter);
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use borsh::{BorshDeserialize, BorshSerialize};

// Correct Anchor CPI imports for admin_xft
use admin_xft::cpi::accounts::GetFees;
use admin_xft::cpi::get_fees;

declare_id!("JDmExoWsKJe7eMEcxaNKgBe1dgHXe5ns3wGqxgc7kAez");

#[program]
pub mod xft_market {
    use super::*;
    // Add instructions here
    
    //settings[0] = label_id
    //settings[1] = license_id
    //settings[2] = price
    //settings[3] = quantity
    //settings[4] = price_type 1 = fixed, 2 = increment 5% every by 3= auction
    //settings[5] = is_active
    //settings[6] = created_at
    //settings[7] = auction_end_time
    //settings[8] = auction_start_price
    //settings[9] = auction_increment_percentage
    //settings[10] = auction_min_price
    //settings[11] = auction_max_price
    //settings[12] = auction_buy_now_price
    //settings[13] = marketplace_fee_percentage
    //settings[14] = payment token 1 = sol, 2 = dexsta

    //addresses[0] = seller
    //addresses[1] = label_vault
    //addresses[2] = operator
    //addresses[3] = seller_payout_address
    //addresses[4] = platform_payout_address
    
    pub fn sell(
        ctx: Context<Sell>,
        xft_id: u64,
        settings: Vec<u64>,
        seller_payout_address: Pubkey,
    ) -> Result<()> {
        let mut settings = settings;
        // Validate inputs
        require!(settings[3] > 0, MarketError::InvalidQuantity);
        let mut allowed = false;
        let mut addresses = Vec::new();
        addresses.push(ctx.accounts.seller.key());
        addresses.push(Pubkey::default()); // dead address to be updated later
        addresses.push(Pubkey::default()); // dead address to be updated later
        addresses.push(seller_payout_address); // dead address to be updated later
        addresses.push(Pubkey::default()); // dead address to be updated later
        
        if settings[1] > 0 && settings[1] == 0 {
            // Item is being sold under a label, check if caller is label owner
            let minter_program = ctx.accounts.xft_minter_program.to_account_info();
            let cpi_accounts = minter::cpi::accounts::IsLabelOwner {
                label_account: ctx.accounts.label_account.clone(),
                authority: ctx.accounts.seller.to_account_info(),
            };
            let cpi_ctx = CpiContext::new(minter_program, cpi_accounts);
            let allowed = minter::cpi::is_label_owner(cpi_ctx, ctx.accounts.seller.key(), xft_id)?.get();

            if allowed {
                // TODO: CPI call to minter program get_vault(xft_id) returns vault address
                // let minter_program = ctx.accounts.xft_minter_program.to_account_info();
                // let cpi_accounts = minter::cpi::accounts::GetVault {
                //     xft_account: ctx.accounts.xft_account.clone(),
                // };
                // let cpi_ctx = CpiContext::new(minter_program, cpi_accounts);
                // let vault_address = minter::cpi::get_vault(cpi_ctx, xft_id)?;
                ///Set addresses[1] as vault address
                let mut addresses = settings.clone();
                if addresses.len() <= 1 {
                    while addresses.len() <= 1 {
                    }
                }
                // Only assign Pubkey to addresses if addresses is a Vec<Pubkey>
                // addresses[1] = Pubkey::default();
            }
        }

        if settings[0] > 0 && settings[1] > 0 {
            ///caller is using a market license to sell under this label
            // Check if cmarket license is valid
            let (is_license_valid, returned_label_id) = is_market_license(&ctx.accounts.xft_account, &ctx.accounts.parent_xft_account)?;
            allowed = is_license_valid && returned_label_id == settings[0];
            if allowed {
                // Set addresses[1] as caller
                if addresses.len() <= 1 {
                    while addresses.len() <= 1 {
                        addresses.push(Pubkey::default());
                    }
                }
                addresses[1] = Pubkey::default();
                settings[13] = 0; // marketplace fee percentage
            }
        }
        settings[6] = Clock::get()?.unix_timestamp as u64;
        // Create or update the listing
        let listing = Listing {
            seller: ctx.accounts.seller.key(),
            xft_id,
            settings: settings.clone(),
            addresses: addresses.clone(),
            price: settings[2],
            quantity: settings[3],
            is_active: true,
        };
        
        if settings[0] > 0 {

            // Add child to parent
            add_child_to_parent_xft(&ctx.accounts.parent_xft_account, xft_id)?;
        }

        // Transfer XFT from seller to escrow (listing account)
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.seller_xft_token_account.to_account_info(),
                to: ctx.accounts.listing_account.to_account_info(),
                authority: ctx.accounts.seller.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, settings[3])?; // Transfer quantity amount

        // Save the listing
        let listing_account_info = &ctx.accounts.listing_account;
        let mut data = listing_account_info.try_borrow_mut_data()?;
        let mut listing_account: ListingAccount = ListingAccount::try_from_slice(&data)?;
        listing_account.listing = listing;
        listing_account.serialize(&mut *data)?;
        
        emit!(ListingCreated {
            seller: ctx.accounts.seller.key(),
            xft_id,
            price: settings[2],
            quantity: settings[3],
            price_type: settings[4],
            created_at: settings[6],
        });

        Ok(())
    }

    pub fn cancel_sell(
        ctx: Context<CancelSell>,
        xft_id: u64,
    ) -> Result<()> {
        // Fetch the listing
        let listing_account_info = &ctx.accounts.listing_account;
        let mut data = listing_account_info.try_borrow_mut_data()?;
        let mut listing_account: ListingAccount = ListingAccount::try_from_slice(&data)?;
        let mut listing = listing_account.listing.clone();
        
        // Verify ownership
        require!(
            listing.addresses[0] == ctx.accounts.seller.key(),
            MarketError::NotAuthorized
        );

        // Mark as inactive
        listing.is_active = false;
        
        // Set settings[5] = 0 and remove xft_id from parent settings[0]
        remove_child_from_parent(&ctx.accounts.parent_xft_account, xft_id)?;
        
        // Update the listing
        listing_account.listing = listing;
        listing_account.serialize(&mut *data)?;

        emit!(ListingCancelled {
            seller: ctx.accounts.seller.key(),
            xft_id,
        });

        Ok(())
    }

    pub fn edit_sell(
        ctx: Context<EditSell>,
        xft_id: u64,
        new_price: u64,
        new_price_type: u64,
    ) -> Result<()> {
        // Validate inputs
        require!(new_price > 0, MarketError::InvalidPrice);
        // Fetch the listing
        let listing_account_info = &ctx.accounts.listing_account;
        let mut data = listing_account_info.try_borrow_mut_data()?;
        let mut listing_account: ListingAccount = ListingAccount::try_from_slice(&data)?;
        let mut listing = listing_account.listing.clone();
        // Verify ownership
        require!(
            listing.addresses[0] == ctx.accounts.seller.key(),
            MarketError::NotAuthorized
        );
        // Update listing details
        listing.settings[2] = new_price;
        listing.settings[4] = new_price_type;
        // Save the updated listing
        listing_account.listing = listing.clone();
        listing_account.serialize(&mut *data)?;
        emit!(ListingEdited {
            seller: ctx.accounts.seller.key(),
            xft_id,
            new_price,
            new_quantity: listing.settings[3],
        });
        Ok(())
    }

    pub fn buy(
        ctx: Context<Buy>,
        xft_id: u64,
        quantity: u64,
    ) -> Result<()> {
        // Restore correct CPI context for get_fees
        let cpi_program = ctx.accounts.xft_admin_program.to_account_info();
        let cpi_accounts = GetFees {
            admin: ctx.accounts.admin_account.clone(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        let result = get_fees(cpi_ctx)?;
        let (_mint_fee_per_year, marketplace_fee_sol, marketplace_fee_dexsta, dexsta_address, payout_address) = result.get();

        // Placeholder values for admin fees and addresses
        let marketplace_fee_sol = marketplace_fee_sol;
        let marketplace_fee_dexsta = marketplace_fee_dexsta;
        let payout_address = payout_address;
        let dexsta_address = dexsta_address;

        // Validate inputs
        require!(quantity > 0, MarketError::InvalidQuantity);

        // Fetch the listing
        let listing_account_info = &ctx.accounts.listing_account;
        let data = listing_account_info.try_borrow_data()?;
        let listing_account: ListingAccount = ListingAccount::try_from_slice(&data)?;
        let listing = &listing_account.listing;
        
        // Verify listing is active
        require!(listing.settings[5] == 1, MarketError::ListingNotActive);
        
        // Verify sufficient quantity
        require!(listing.settings[3] >= quantity, MarketError::InsufficientQuantity);
        
        // Check if this is a licensed label with fees
        let settings = &listing.settings;
        
        let mut total_cost = settings[2] * quantity;
        let mut label_fee = 0;
        let mut marketplace_fee_amount = 0;
        if settings.get(13).copied().unwrap_or(0) > 0 && settings.get(1).copied().unwrap_or(0) > 0 {
            // Calculate label fee percentage from settings[13]
            let fee_percentage = settings.get(13).copied().unwrap_or(0);
            label_fee = (total_cost * fee_percentage) / 10000; // Assuming fee is in basis points (10000 = 100%)
            total_cost += label_fee;
            // Check payment type and handle fees accordingly
            if settings.get(14).copied().unwrap_or(0) == 1 {
                // Payment is in SOL - pay marketplace fee to listing.addresses[1]
                if let Some(fee_recipient) = listing.addresses.get(1) {
                    if *fee_recipient != Pubkey::default() {
                        marketplace_fee_amount = (total_cost * marketplace_fee_sol) / 10000;
                        let payout_account = get_account_info_for_pubkey(&ctx, fee_recipient).expect("payout account not found");
                        pay_sol(&ctx.accounts.buyer.to_account_info(), &payout_account, marketplace_fee_amount)?;
                    }
                }
            } else if settings.get(14).copied().unwrap_or(0) == 2 {
                // Payment is in SPL token - handle SPL token transfer for marketplace fee
                if let Some(fee_recipient) = listing.addresses.get(1) {
                    if *fee_recipient != Pubkey::default() {
                        marketplace_fee_amount = (total_cost * marketplace_fee_dexsta) / 10000;
                        let payout_account = get_account_info_for_pubkey(&ctx, fee_recipient).expect("payout account not found");
                        let authority_account = get_account_info_for_pubkey(&ctx, &dexsta_address).expect("authority not found");
                        pay_spl(&ctx.accounts.buyer.to_account_info(), &payout_account, &authority_account, &ctx.accounts.token_program.to_account_info(), marketplace_fee_amount)?;
                    }
                }
            }
        }
        
        // Apply marketplace fee to total cost
        let marketplace_fee = if settings.get(14).copied().unwrap_or(0) == 1 {
            marketplace_fee_sol
        } else {
            if marketplace_fee_dexsta > 0 {
                marketplace_fee_dexsta
            } else {
                0
            }
        };
        let marketplace_fee_amount = (total_cost * marketplace_fee) / 10000; // Assuming fee is in basis points
        if marketplace_fee_amount > 0 {
            if settings.get(14).copied().unwrap_or(0) == 1 {
                let payout_account = get_account_info_for_pubkey(&ctx, &listing.addresses[1]).expect("payout account not found");
                pay_sol(&ctx.accounts.buyer.to_account_info(), &payout_account, marketplace_fee_amount)?;
            } else if settings.get(14).copied().unwrap_or(0) == 2 {
                let payout_account = get_account_info_for_pubkey(&ctx, &listing.addresses[1]).expect("payout account not found");
                let authority_account = get_account_info_for_pubkey(&ctx, &dexsta_address).expect("authority not found");
                pay_spl(&ctx.accounts.buyer.to_account_info(), &payout_account, &authority_account, &ctx.accounts.token_program.to_account_info(), marketplace_fee_amount)?;
            }

            // Set listing.addresses[4] to admin payout address
            let mut updated_listing = listing.clone();
            while updated_listing.addresses.len() <= 4 {
                updated_listing.addresses.push(Pubkey::default());
            }
            updated_listing.addresses[4] = payout_address;
        }
        total_cost += marketplace_fee_amount;
                

        // Transfer SOL from buyer to seller
        // Determine payment recipient based on listing settings
        let payment_recipient = listing.seller;

        // Send payment based on payment type
        if settings.get(14).copied().unwrap_or(0) == 1 {
            let payout_account = get_account_info_for_pubkey(&ctx, &payment_recipient).expect("payout account not found");
            pay_sol(&ctx.accounts.buyer.to_account_info(), &payout_account, total_cost)?;
        } else if settings.get(14).copied().unwrap_or(0) == 2 {
            let payout_account = get_account_info_for_pubkey(&ctx, &payment_recipient).expect("payout account not found");
            let authority_account = get_account_info_for_pubkey(&ctx, &dexsta_address).expect("authority not found");
            pay_spl(&ctx.accounts.buyer.to_account_info(), &payout_account, &authority_account, &ctx.accounts.token_program.to_account_info(), total_cost)?;
        }
        
        // Transfer XFT to buyer
        let xft_program = ctx.accounts.xft_minter_program.to_account_info();
        let cpi_accounts = minter::cpi::accounts::TransferXft {
            from: ctx.accounts.seller.clone(),
            to: ctx.accounts.buyer.to_account_info(),
            xft_account: ctx.accounts.xft_account.clone(),
        };
        let cpi_ctx = CpiContext::new(xft_program, cpi_accounts);
        minter::cpi::transfer_xft(cpi_ctx)?;
        // Update listing quantity
        let mut updated_listing = listing.clone();
        updated_listing.quantity -= quantity;
        
        if updated_listing.quantity == 0 {
            updated_listing.is_active = false;
            remove_child_from_parent(&ctx.accounts.parent_xft_account, xft_id)?;
        }

        // Save the updated listing
        let listing_account_info = &ctx.accounts.listing_account;
        let mut data = listing_account_info.try_borrow_mut_data()?;
        let mut listing_account: ListingAccount = ListingAccount::try_from_slice(&data)?;
        listing_account.listing = updated_listing;
        listing_account.serialize(&mut *data)?;

        emit!(PurchaseCompleted {
            buyer: ctx.accounts.buyer.key(),
            seller: listing.seller,
            xft_id,
            quantity,
            total_cost,
        });

        Ok(())
    }
}

// Add this helper function for market license check
pub fn is_market_license(xft_account_info: &AccountInfo, parent_xft_account_info: &AccountInfo) -> Result<(bool, u64)> {
    let xft_account = XftAccount::try_from_slice(&xft_account_info.data.borrow())?;
    let settings = xft_account.settings;
    // Check if settings[7] > now (expire check)
    let expire = settings.get(7).copied().unwrap_or(0);
    let now = Clock::get()?.unix_timestamp as u64;
    if expire <= now {
        return Ok((false, 0));
    }
    // Get the parent xft account (settings[0] of xft_id)
    let parent_xft_id = settings.get(0).copied().unwrap_or(0);
    if parent_xft_id == 0 {
        return Ok((false, 0));
    }
    let parent_xft_account = XftAccount::try_from_slice(&parent_xft_account_info.data.borrow())?;
    let parent_settings = parent_xft_account.settings;
    // Check if settings[7] of parent_xft account > now
    let parent_expire = parent_settings.get(7).copied().unwrap_or(0);
    if parent_expire <= now {
        return Ok((false, parent_xft_id));
    }
    Ok((true, parent_xft_id))
}

// Helper function to add a child XFT to a parent XFT's settings array
pub fn add_child_to_parent_xft(
    parent_xft_account_info: &AccountInfo,
    xft_id: u64,
) -> Result<()> {
    let data = &mut parent_xft_account_info.try_borrow_mut_data()?;
    let mut parent_xft_account = XftAccount::try_from_slice(&data)?;
    let mut parent_settings = parent_xft_account.settings.clone();
    while parent_settings.len() <= 14 {
        parent_settings.push(0);
    }
    let mut child_index = 14;
    while child_index < parent_settings.len() && parent_settings[child_index] != 0 {
        child_index += 1;
    }
    if child_index >= parent_settings.len() {
        parent_settings.push(xft_id);
    } else {
        parent_settings[child_index] = xft_id;
    }
    parent_xft_account.settings = parent_settings;
    parent_xft_account.serialize(&mut **data)?;
    Ok(())
}

// Helper function to remove a child XFT from a parent XFT's settings array
pub fn remove_child_from_parent(
    parent_xft_account_info: &AccountInfo,
    xft_id: u64,
) -> Result<()> {
    let data = &mut parent_xft_account_info.try_borrow_mut_data()?;
    let mut parent_xft_account = XftAccount::try_from_slice(&data)?;
    let mut parent_settings = parent_xft_account.settings.clone();
    let mut found = false;
    for i in 14..parent_settings.len() {
        if parent_settings[i] == xft_id {
            parent_settings[i] = 0;
            found = true;
            break;
        }
    }
    if !found {
        return Err(error!(MarketError::ParentAccountMismatch));
    }
    parent_xft_account.settings = parent_settings;
    parent_xft_account.serialize(&mut **data)?;
    Ok(())
}

// Internal helper to pay SOL from one account to another
fn pay_sol<'a>(from: &AccountInfo<'a>, to: &AccountInfo<'a>, amount: u64) -> Result<()> {
    let ix = anchor_lang::solana_program::system_instruction::transfer(
        from.key,
        to.key,
        amount,
    );
    anchor_lang::solana_program::program::invoke(
        &ix,
        &[from.clone(), to.clone()],
    )?;
    Ok(())
}

// Internal helper to pay SPL tokens from one account to another
fn pay_spl<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = token::Transfer {
        from: from.clone(),
        to: to.clone(),
        authority: authority.clone(),
    };
    let cpi_ctx = CpiContext::new(token_program.clone(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;
    Ok(())
}

// Helper to map a Pubkey in listing.addresses to the correct AccountInfo in the Buy context
fn get_account_info_for_pubkey<'info>(
    ctx: &Context<Buy<'info>>,
    key: &Pubkey,
) -> Option<AccountInfo<'info>> {
    if key == ctx.accounts.seller_payout.key {
        Some(ctx.accounts.seller_payout.clone())
    } else if key == ctx.accounts.label_vault.key {
        Some(ctx.accounts.label_vault.clone())
    } else if key == ctx.accounts.operator_payout.key {
        Some(ctx.accounts.operator_payout.clone())
    } else if key == ctx.accounts.platform_payout.key {
        Some(ctx.accounts.platform_payout.clone())
    } else if key == ctx.accounts.dexsta_authority.key {
        Some(ctx.accounts.dexsta_authority.clone())
    } else {
        None
    }
}

#[error_code]
pub enum MarketError {
    InvalidQuantity,
    InvalidPrice,
    NotAuthorized,
    ListingNotActive,
    InsufficientQuantity,
    ParentAccountMismatch,
}

#[derive(Accounts)]
pub struct Sell<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub parent_xft_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub xft_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub operator_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub xft_minter_program: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub label_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub token_program: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub seller_xft_token_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub listing_account: AccountInfo<'info>,
    /// CHECK: Label vault for payouts
    #[account(mut)]
    pub label_vault: AccountInfo<'info>,
    /// CHECK: Operator payout account
    #[account(mut)]
    pub operator_payout: AccountInfo<'info>,
    /// CHECK: Seller payout account
    #[account(mut)]
    pub seller_payout: AccountInfo<'info>,
    /// CHECK: Platform payout account
    #[account(mut)]
    pub platform_payout: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct EditSell<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub listing_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub listing_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub seller: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub xft_admin_program: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub admin_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub xft_account: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub xft_minter_program: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub token_program: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub system_program: AccountInfo<'info>,
    /// CHECK: payout accounts
    #[account(mut)]
    pub seller_payout: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub label_vault: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub operator_payout: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub platform_payout: AccountInfo<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub dexsta_authority: AccountInfo<'info>,
    /// CHECK: Parent XFT account for child/parent logic
    #[account(mut)]
    pub parent_xft_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CancelSell<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    /// CHECK: This is safe for prototyping; actual checks should be implemented in production
    #[account(mut)]
    pub listing_account: AccountInfo<'info>,
    /// CHECK: Parent XFT account for child/parent logic
    #[account(mut)]
    pub parent_xft_account: AccountInfo<'info>,
}

// Listing struct for storing listing data
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct Listing {
    pub seller: Pubkey,
    pub xft_id: u64,
    pub settings: Vec<u64>,
    pub addresses: Vec<Pubkey>,
    pub price: u64,
    pub quantity: u64,
    pub is_active: bool,
}

// ListingAccount for storing a listing on-chain
#[account]
pub struct ListingAccount {
    pub listing: Listing,
}

// Event definitions
#[event]
pub struct ListingCreated {
    pub seller: Pubkey,
    pub xft_id: u64,
    pub price: u64,
    pub quantity: u64,
    pub price_type: u64,
    pub created_at: u64,
}

#[event]
pub struct ListingCancelled {
    pub seller: Pubkey,
    pub xft_id: u64,
}

#[event]
pub struct ListingEdited {
    pub seller: Pubkey,
    pub xft_id: u64,
    pub new_price: u64,
    pub new_quantity: u64,
}

#[event]
pub struct PurchaseCompleted {
    pub buyer: Pubkey,
    pub seller: Pubkey,
    pub xft_id: u64,
    pub quantity: u64,
    pub total_cost: u64,
}

// Add a local definition for XftAccount for deserialization
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct XftAccount {
    pub xft_id: u64,
    pub settings: Vec<u64>,
    pub addresses: Vec<Pubkey>,
    pub ipfs: String,
    pub bump: u8,
}
