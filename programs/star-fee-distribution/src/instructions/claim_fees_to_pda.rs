use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::errors::ErrorCode;
use crate::constants::{FEE_COLLECTOR_SEED, FEE_VAULT_SEED};

#[derive(Accounts)]
pub struct ClaimFeesToPDA<'info> {
    /// CHECK: Program authority (our program)
    #[account(
        mut,
        seeds = [FEE_COLLECTOR_SEED],
        bump
    )]
    pub fee_collector: UncheckedAccount<'info>,
    
    /// CHECK: DAMM v2 program
    #[account(address = damm_v2::ID)]
    pub amm_program: UncheckedAccount<'info>,
    
    /// CHECK: Pool account
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    
    /// CHECK: Position account (owned by our PDA)
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    
    /// CHECK: Position NFT account
    #[account(mut)]
    pub position_nft_account: UncheckedAccount<'info>,
    
    /// CHECK: Pool authority
    #[account(mut)]
    pub pool_authority: UncheckedAccount<'info>,
    
    /// CHECK: Base token mint (token A)
    #[account(mut)]
    pub base_mint: UncheckedAccount<'info>,
    
    /// CHECK: Quote token mint (token B)
    #[account(mut)]
    pub quote_mint: UncheckedAccount<'info>,
    
    /// CHECK: Base token vault
    #[account(mut)]
    pub token_a_vault: UncheckedAccount<'info>,
    
    /// CHECK: Quote token vault
    #[account(mut)]
    pub token_b_vault: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [FEE_VAULT_SEED, base_mint.key().as_ref()],
        bump,
        token::mint = base_mint,
        token::authority = fee_collector
    )]
    pub program_token_a_vault: Box<Account<'info, TokenAccount>>,
    
    /// Program's quote token vault for fee collection
    #[account(
        mut,
        seeds = [FEE_VAULT_SEED, quote_mint.key().as_ref()],
        bump,
        token::mint = quote_mint,
        token::authority = fee_collector
    )]
    pub program_token_b_vault: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: Token program
    pub token_program: Program<'info, Token>,
    
    /// CHECK: Event authority
    pub event_authority: UncheckedAccount<'info>,
}

impl<'info> ClaimFeesToPDA<'info> {
    pub fn handle(ctx: Context<ClaimFeesToPDA>) -> Result<()> {
        msg!("Claiming fees to program PDA for pool: {}", ctx.accounts.pool.key());
        
        // Validate that the position exists and is valid
        validate_position_accounts_pda(&ctx)?;
        
        // Record balances before claim
        let base_balance_before = ctx.accounts.program_token_a_vault.amount;
        let quote_balance_before = ctx.accounts.program_token_b_vault.amount;
        
        msg!("Base vault balance before: {} units", base_balance_before);
        msg!("Quote vault balance before: {} units", quote_balance_before);
        
        // Use DAMM v2 CPI to claim position fees to our program's token vaults
        match damm_v2::cpi::claim_position_fee(
            CpiContext::new_with_signer(
                ctx.accounts.amm_program.to_account_info(),
                damm_v2::cpi::accounts::ClaimPositionFee {
                    pool_authority: ctx.accounts.pool_authority.to_account_info(),
                    pool: ctx.accounts.pool.to_account_info(),
                    position: ctx.accounts.position.to_account_info(),
                    token_a_account: ctx.accounts.program_token_a_vault.to_account_info(),
                    token_b_account: ctx.accounts.program_token_b_vault.to_account_info(),
                    token_a_vault: ctx.accounts.token_a_vault.to_account_info(),
                    token_b_vault: ctx.accounts.token_b_vault.to_account_info(),
                    token_a_mint: ctx.accounts.base_mint.to_account_info(),
                    token_b_mint: ctx.accounts.quote_mint.to_account_info(),
                    position_nft_account: ctx.accounts.position_nft_account.to_account_info(),
                    owner: ctx.accounts.fee_collector.to_account_info(),
                    token_a_program: ctx.accounts.token_program.to_account_info(),
                    token_b_program: ctx.accounts.token_program.to_account_info(),
                    event_authority: ctx.accounts.event_authority.to_account_info(),
                    program: ctx.accounts.amm_program.to_account_info(),
                },
                &[&[
                    FEE_COLLECTOR_SEED,
                    &[ctx.bumps.fee_collector]
                ]]
            ),
        ) {
            Ok(_) => {
                // Reload accounts to get updated balances
                ctx.accounts.program_token_a_vault.reload()?;
                ctx.accounts.program_token_b_vault.reload()?;
                
                let base_balance_after = ctx.accounts.program_token_a_vault.amount;
                let quote_balance_after = ctx.accounts.program_token_b_vault.amount;
                
                let base_claimed = base_balance_after.saturating_sub(base_balance_before);
                let quote_claimed = quote_balance_after.saturating_sub(quote_balance_before);
                
                msg!("Base fees claimed: {} units", base_claimed);
                msg!("Quote fees claimed: {} units", quote_claimed);
                
                // CRITICAL: Enforce quote-only fees
                // If ANY base fees were claimed, fail the transaction
                require!(
                    base_claimed == 0,
                    ErrorCode::BaseFeesDetected
                );
                
                msg!("✅ Quote-only validation passed - no base fees detected");
                msg!("Fees claimed successfully to program PDA!");
                
                // Emit event
                emit!(crate::events::QuoteFeesClaimed {
                    pool: ctx.accounts.pool.key(),
                    position: ctx.accounts.position.key(),
                    base_fees_claimed: base_claimed,
                    quote_fees_claimed: quote_claimed,
                    program_base_vault: ctx.accounts.program_token_a_vault.key(),
                    program_quote_vault: ctx.accounts.program_token_b_vault.key(),
                    timestamp: Clock::get()?.unix_timestamp,
                });
                
                Ok(())
            }
            Err(e) => {
                msg!("Failed to claim fees to PDA: {:?}", e);
                // Check if it's a "no fees to claim" error
                if e.to_string().contains("no fees") || e.to_string().contains("insufficient") {
                    return Err(ErrorCode::NoFeesToClaim.into());
                }
                Err(e)
            }
        }
    }
}

/// Validates that the position and related accounts are properly configured for PDA collection
fn validate_position_accounts_pda(ctx: &Context<ClaimFeesToPDA>) -> Result<()> {
    // Validate that the position account is not empty
    require!(
        ctx.accounts.position.data_is_empty() == false,
        ErrorCode::InvalidPosition
    );
    
    // Validate that the pool account is not empty
    require!(
        ctx.accounts.pool.data_is_empty() == false,
        ErrorCode::InvalidPosition
    );
    
    // Validate that the position NFT account is not empty
    require!(
        ctx.accounts.position_nft_account.data_is_empty() == false,
        ErrorCode::InvalidPosition
    );
    
    msg!("Position accounts validated successfully for PDA collection");
    Ok(())
}
