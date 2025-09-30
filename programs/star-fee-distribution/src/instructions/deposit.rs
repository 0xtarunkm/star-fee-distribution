use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount, Token};
use crate::errors::ErrorCode;
use crate::constants::{FEE_COLLECTOR_SEED, DEPOSIT_VAULT_SEED, INVESTOR_RECORD_SEED};
use crate::states::{DepositorRecord, VaultStats};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DepositParams {
    pub sol_amount: u64,
    pub usdc_amount: u64,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub investor: Signer<'info>,

    /// CHECK: This is a PDA derived from the program ID and "fee_collector" seed
    #[account(
        mut,
        seeds = [FEE_COLLECTOR_SEED],
        bump
    )]
    pub fee_collector: UncheckedAccount<'info>,
    
    #[account(
        mut,
        seeds = [DEPOSIT_VAULT_SEED, b"sol"],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,
    
    #[account(
        init_if_needed,
        payer = investor,
        seeds = [DEPOSIT_VAULT_SEED, usdc_mint.key().as_ref()],
        bump,
        token::mint = usdc_mint,
        token::authority = fee_collector
    )]
    pub usdc_vault: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: This is a valid SPL token mint account
    #[account(mut)]
    pub usdc_mint: UncheckedAccount<'info>,
    
    #[account(
        mut,
        token::mint = usdc_mint,
        token::authority = investor
    )]
    pub investor_usdc_account: Box<Account<'info, TokenAccount>>,
    
    #[account(
        init_if_needed,
        payer = investor,
        space = DepositorRecord::DISCRIMINATOR.len() + DepositorRecord::INIT_SPACE,
        seeds = [INVESTOR_RECORD_SEED, investor.key().as_ref()],
        bump
    )]
    pub depositor_record: Account<'info, DepositorRecord>,
    
    #[account(
        init_if_needed,
        payer = investor,
        space = VaultStats::DISCRIMINATOR.len() + VaultStats::INIT_SPACE,
        seeds = [DEPOSIT_VAULT_SEED, b"stats"],
        bump
    )]
    pub vault_stats: Account<'info, VaultStats>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn handle(mut ctx: Context<Deposit>, params: DepositParams) -> Result<()> {
        msg!("Processing deposit from investor: {}", ctx.accounts.investor.key());
        msg!("SOL amount: {} lamports", params.sol_amount);
        msg!("USDC amount: {} units", params.usdc_amount);
        
        validate_deposit_amounts(&params)?;
        
        if params.sol_amount > 0 {
            process_sol_deposit(&ctx, params.sol_amount)?;
        }
        
        if params.usdc_amount > 0 {
            process_usdc_deposit(&ctx, params.usdc_amount)?;
        }
        
        update_depositor_record(&mut ctx, params.sol_amount, params.usdc_amount)?;
        
        update_vault_stats(&mut ctx, params.sol_amount, params.usdc_amount)?;
        
        msg!("Deposit completed successfully!");
        
        // Emit event
        let depositor_record = &ctx.accounts.depositor_record;
        emit!(crate::events::DepositMade {
            investor: ctx.accounts.investor.key(),
            sol_amount: params.sol_amount,
            usdc_amount: params.usdc_amount,
            total_sol_deposited: depositor_record.total_sol_deposited,
            total_usdc_deposited: depositor_record.total_usdc_deposited,
            current_sol_balance: depositor_record.current_sol_balance,
            current_usdc_balance: depositor_record.current_usdc_balance,
            deposit_count: depositor_record.deposit_count,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }
}

fn validate_deposit_amounts(params: &DepositParams) -> Result<()> {
    require!(
        params.sol_amount > 0 || params.usdc_amount > 0,
        ErrorCode::InvalidDepositAmount
    );
    
    require!(
        params.sol_amount == 0 || params.sol_amount >= 1_000_000, // Minimum 0.001 SOL
        ErrorCode::InvalidDepositAmount
    );
    
    require!(
        params.usdc_amount == 0 || params.usdc_amount >= 1_000, // Minimum 0.001 USDC
        ErrorCode::InvalidDepositAmount
    );
    
    require!(
        params.sol_amount <= 1_000_000_000_000, // Maximum 1000 SOL
        ErrorCode::InvalidDepositAmount
    );
    
    require!(
        params.usdc_amount <= 1_000_000_000_000, // Maximum 1M USDC
        ErrorCode::InvalidDepositAmount
    );
    
    Ok(())
}

fn process_sol_deposit(ctx: &Context<Deposit>, amount: u64) -> Result<()> {
    anchor_lang::system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.investor.to_account_info(),
                to: ctx.accounts.sol_vault.to_account_info(),
            },
        ),
        amount,
    )?;
    
    Ok(())
}

fn process_usdc_deposit(ctx: &Context<Deposit>, amount: u64) -> Result<()> {
    anchor_spl::token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.investor_usdc_account.to_account_info(),
                to: ctx.accounts.usdc_vault.to_account_info(),
                authority: ctx.accounts.investor.to_account_info(),
            },
        ),
        amount,
    )?;
    
    Ok(())
}

fn update_depositor_record(ctx: &mut Context<Deposit>, sol_amount: u64, usdc_amount: u64) -> Result<()> {
    let depositor_record = &mut ctx.accounts.depositor_record;
    
    // Initialize investor field if this is a new record (deposit_count == 0)
    if depositor_record.deposit_count == 0 {
        depositor_record.investor = ctx.accounts.investor.key();
        depositor_record.bump = ctx.bumps.depositor_record;
    }
    
    depositor_record.add_deposit(sol_amount, usdc_amount)?;

    Ok(())
}

fn update_vault_stats(ctx: &mut Context<Deposit>, sol_amount: u64, usdc_amount: u64) -> Result<()> {
    let vault_stats = &mut ctx.accounts.vault_stats;
    
    vault_stats.add_deposits(sol_amount, usdc_amount)?;
    
    if ctx.accounts.depositor_record.deposit_count == 1 {
        vault_stats.depositor_count = vault_stats.depositor_count
            .checked_add(1)
            .ok_or(ErrorCode::MathOverflow)?;
    }
    
    Ok(())
}

