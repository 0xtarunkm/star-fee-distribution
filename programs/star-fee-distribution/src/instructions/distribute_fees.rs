use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::errors::ErrorCode;
use crate::constants::{FEE_COLLECTOR_SEED, FEE_VAULT_SEED};

#[derive(Accounts)]
pub struct DistributeFees<'info> {
    /// CHECK: Program authority (our program)
    #[account(
        mut,
        seeds = [FEE_COLLECTOR_SEED],
        bump
    )]
    pub fee_collector: UncheckedAccount<'info>,
    
    /// Program's base token vault for fee collection
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
    
    /// CHECK: Base token mint
    #[account(mut)]
    pub base_mint: UncheckedAccount<'info>,
    
    /// CHECK: Quote token mint
    #[account(mut)]
    pub quote_mint: UncheckedAccount<'info>,
    
    /// Recipient's base token account
    #[account(mut)]
    pub recipient_token_a_account: Box<Account<'info, TokenAccount>>,
    
    /// Recipient's quote token account
    #[account(mut)]
    pub recipient_token_b_account: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: Token program
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct FeeDistributionParams {
    /// Amount of base token to distribute (0 = distribute all)
    pub base_amount: u64,
    /// Amount of quote token to distribute (0 = distribute all)
    pub quote_amount: u64,
}

impl<'info> DistributeFees<'info> {
    pub fn handle(ctx: Context<DistributeFees>, params: FeeDistributionParams) -> Result<()> {
        msg!("Distributing fees from program PDA");
        
        let base_amount = if params.base_amount == 0 {
            ctx.accounts.program_token_a_vault.amount
        } else {
            params.base_amount
        };
        
        let quote_amount = if params.quote_amount == 0 {
            ctx.accounts.program_token_b_vault.amount
        } else {
            params.quote_amount
        };
        
        // Transfer base tokens if amount > 0
        if base_amount > 0 {
            require!(
                ctx.accounts.program_token_a_vault.amount >= base_amount,
                ErrorCode::InsufficientTokenBalance
            );
            
            anchor_spl::token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    anchor_spl::token::Transfer {
                        from: ctx.accounts.program_token_a_vault.to_account_info(),
                        to: ctx.accounts.recipient_token_a_account.to_account_info(),
                        authority: ctx.accounts.fee_collector.to_account_info(),
                    },
                    &[&[
                        FEE_COLLECTOR_SEED,
                        &[ctx.bumps.fee_collector]
                    ]]
                ),
                base_amount,
            )?;
            
            msg!("Distributed {} base tokens", base_amount);
        }
        
        // Transfer quote tokens if amount > 0
        if quote_amount > 0 {
            require!(
                ctx.accounts.program_token_b_vault.amount >= quote_amount,
                ErrorCode::InsufficientTokenBalance
            );
            
            anchor_spl::token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    anchor_spl::token::Transfer {
                        from: ctx.accounts.program_token_b_vault.to_account_info(),
                        to: ctx.accounts.recipient_token_b_account.to_account_info(),
                        authority: ctx.accounts.fee_collector.to_account_info(),
                    },
                    &[&[
                        FEE_COLLECTOR_SEED,
                        &[ctx.bumps.fee_collector]
                    ]]
                ),
                quote_amount,
            )?;
            
            msg!("Distributed {} quote tokens", quote_amount);
        }
        
        msg!("Fee distribution completed successfully!");
        Ok(())
    }
}
