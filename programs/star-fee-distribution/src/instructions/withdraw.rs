use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::errors::ErrorCode;
use crate::constants::{FEE_COLLECTOR_SEED, DEPOSIT_VAULT_SEED, INVESTOR_RECORD_SEED};
use crate::states::{DepositorRecord, VaultStats};

/// Withdrawal instruction for investors to withdraw SOL/USDC from vaults
#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// The investor making the withdrawal
    #[account(mut)]
    pub investor: Signer<'info>,
    
    /// CHECK: Program authority (our program)
    #[account(
        mut,
        seeds = [FEE_COLLECTOR_SEED],
        bump
    )]
    pub fee_collector: UncheckedAccount<'info>,
    
    /// Program's SOL vault for deposits
    #[account(
        mut,
        seeds = [DEPOSIT_VAULT_SEED, b"sol"],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    /// Program's USDC vault for deposits
    #[account(
        init_if_needed,
        payer = investor,
        seeds = [DEPOSIT_VAULT_SEED, usdc_mint.key().as_ref()],
        bump,
        token::mint = usdc_mint,
        token::authority = fee_collector
    )]
    pub usdc_vault: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: USDC mint
    #[account(mut)]
    pub usdc_mint: UncheckedAccount<'info>,
    
    /// Investor's USDC token account
    #[account(
        mut,
        token::mint = usdc_mint,
        token::authority = investor
    )]
    pub investor_usdc_account: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: Token program
    pub token_program: Program<'info, Token>,
    
    /// CHECK: System program
    pub system_program: Program<'info, System>,
    
    /// Depositor record for this investor
    #[account(
        mut,
        seeds = [INVESTOR_RECORD_SEED, investor.key().as_ref()],
        bump = depositor_record.bump,
        has_one = investor
    )]
    pub depositor_record: Account<'info, DepositorRecord>,
    
    /// Global vault statistics
    #[account(
        mut,
        seeds = [DEPOSIT_VAULT_SEED, b"stats"],
        bump = vault_stats.bump
    )]
    pub vault_stats: Account<'info, VaultStats>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct WithdrawParams {
    /// Amount of SOL to withdraw (in lamports)
    pub sol_amount: u64,
    /// Amount of USDC to withdraw (in smallest unit)
    pub usdc_amount: u64,
}

impl<'info> Withdraw<'info> {
    pub fn handle(mut ctx: Context<Withdraw>, params: WithdrawParams) -> Result<()> {
        msg!("Processing withdrawal for investor: {}", ctx.accounts.investor.key());
        msg!("SOL amount: {} lamports", params.sol_amount);
        msg!("USDC amount: {} units", params.usdc_amount);
        
        // Validate withdrawal amounts
        validate_withdrawal_amounts(&ctx, &params)?;
        
        // Process SOL withdrawal if amount > 0
        if params.sol_amount > 0 {
            process_sol_withdrawal(&ctx, params.sol_amount)?;
        }
        
        // Process USDC withdrawal if amount > 0
        if params.usdc_amount > 0 {
            process_usdc_withdrawal(&ctx, params.usdc_amount)?;
        }
        
        // Update depositor record
        update_depositor_record_withdrawal(&mut ctx, params.sol_amount, params.usdc_amount)?;
        
        // Update vault stats
        update_vault_stats_withdrawal(&mut ctx, params.sol_amount, params.usdc_amount)?;
        
        msg!("Withdrawal completed successfully!");
        
        // Emit event
        let depositor_record = &ctx.accounts.depositor_record;
        emit!(crate::events::WithdrawalMade {
            investor: ctx.accounts.investor.key(),
            sol_amount: params.sol_amount,
            usdc_amount: params.usdc_amount,
            total_sol_withdrawn: depositor_record.total_sol_withdrawn,
            total_usdc_withdrawn: depositor_record.total_usdc_withdrawn,
            current_sol_balance: depositor_record.current_sol_balance,
            current_usdc_balance: depositor_record.current_usdc_balance,
            withdrawal_count: depositor_record.withdrawal_count,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }
}

/// Validates that withdrawal amounts are valid and available
fn validate_withdrawal_amounts(ctx: &Context<Withdraw>, params: &WithdrawParams) -> Result<()> {
    // Check that at least one withdrawal amount is provided
    require!(
        params.sol_amount > 0 || params.usdc_amount > 0,
        ErrorCode::InvalidDepositAmount
    );
    
    // Check minimum withdrawal amounts
    require!(
        params.sol_amount == 0 || params.sol_amount >= 1_000_000, // Minimum 0.001 SOL
        ErrorCode::InvalidDepositAmount
    );
    
    require!(
        params.usdc_amount == 0 || params.usdc_amount >= 1_000, // Minimum 0.001 USDC
        ErrorCode::InvalidDepositAmount
    );
    
    // Check that vault has sufficient balance for SOL withdrawal
    if params.sol_amount > 0 {
        require!(
            ctx.accounts.sol_vault.lamports() >= params.sol_amount,
            ErrorCode::InsufficientTokenBalance
        );
    }
    
    // Check that vault has sufficient balance for USDC withdrawal
    if params.usdc_amount > 0 {
        require!(
            ctx.accounts.usdc_vault.amount >= params.usdc_amount,
            ErrorCode::InsufficientTokenBalance
        );
    }
    
    Ok(())
}

/// Processes SOL withdrawal by transferring from vault to investor
fn process_sol_withdrawal(ctx: &Context<Withdraw>, amount: u64) -> Result<()> {
    msg!("Processing SOL withdrawal of {} lamports", amount);
    
    // Transfer SOL from vault to investor
    anchor_lang::system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.sol_vault.to_account_info(),
                to: ctx.accounts.investor.to_account_info(),
            },
            &[&[
                FEE_COLLECTOR_SEED,
                &[ctx.bumps.fee_collector]
            ]]
        ),
        amount,
    )?;
    
    msg!("SOL withdrawal successful: {} lamports transferred to investor", amount);
    Ok(())
}

/// Processes USDC withdrawal by transferring from vault to investor
fn process_usdc_withdrawal(ctx: &Context<Withdraw>, amount: u64) -> Result<()> {
    msg!("Processing USDC withdrawal of {} units", amount);
    
    // Transfer USDC from vault to investor
    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.usdc_vault.to_account_info(),
                to: ctx.accounts.investor_usdc_account.to_account_info(),
                authority: ctx.accounts.fee_collector.to_account_info(),
            },
            &[&[
                FEE_COLLECTOR_SEED,
                &[ctx.bumps.fee_collector]
            ]]
        ),
        amount,
    )?;
    
    msg!("USDC withdrawal successful: {} units transferred to investor", amount);
    Ok(())
}

/// Updates the depositor record with withdrawal information
fn update_depositor_record_withdrawal(ctx: &mut Context<Withdraw>, sol_amount: u64, usdc_amount: u64) -> Result<()> {
    let depositor_record = &mut ctx.accounts.depositor_record;
    
    // Add withdrawal to record
    depositor_record.add_withdrawal(sol_amount, usdc_amount)?;
    
    msg!("Updated depositor record for investor: {}", ctx.accounts.investor.key());
    msg!("Total SOL withdrawn: {} lamports", depositor_record.total_sol_withdrawn);
    msg!("Total USDC withdrawn: {} units", depositor_record.total_usdc_withdrawn);
    msg!("Current SOL balance: {} lamports", depositor_record.current_sol_balance);
    msg!("Current USDC balance: {} units", depositor_record.current_usdc_balance);
    
    Ok(())
}

/// Updates the global vault statistics with withdrawal information
fn update_vault_stats_withdrawal(ctx: &mut Context<Withdraw>, sol_amount: u64, usdc_amount: u64) -> Result<()> {
    let vault_stats = &mut ctx.accounts.vault_stats;
    
    // Add withdrawals to stats
    vault_stats.add_withdrawals(sol_amount, usdc_amount)?;
    
    msg!("Updated vault stats:");
    msg!("Total SOL withdrawn: {} lamports", vault_stats.total_sol_withdrawn);
    msg!("Total USDC withdrawn: {} units", vault_stats.total_usdc_withdrawn);
    msg!("Current total SOL: {} lamports", vault_stats.current_total_sol);
    msg!("Current total USDC: {} units", vault_stats.current_total_usdc);
    
    Ok(())
}

