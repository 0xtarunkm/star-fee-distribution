use anchor_lang::prelude::*;
use crate::errors::ErrorCode;
use crate::constants::{INVESTOR_RECORD_SEED, DEPOSIT_VAULT_SEED};
use crate::states::{DepositorRecord, VaultStats};

/// Query instruction to get depositor information
#[derive(Accounts)]
pub struct QueryDepositor<'info> {
    /// The investor to query
    #[account(mut)]
    pub investor: Signer<'info>,
    
    /// Depositor record for this investor
    #[account(
        seeds = [INVESTOR_RECORD_SEED, investor.key().as_ref()],
        bump = depositor_record.bump,
        has_one = investor
    )]
    pub depositor_record: Account<'info, DepositorRecord>,
    
    /// Global vault statistics
    #[account(
        seeds = [DEPOSIT_VAULT_SEED, b"stats"],
        bump = vault_stats.bump
    )]
    pub vault_stats: Account<'info, VaultStats>,
}

/// Response structure for depositor query
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DepositorInfo {
    /// The investor's public key
    pub investor: Pubkey,
    /// Total SOL deposited (in lamports)
    pub total_sol_deposited: u64,
    /// Total USDC deposited (in smallest unit)
    pub total_usdc_deposited: u64,
    /// Current SOL balance (in lamports)
    pub current_sol_balance: u64,
    /// Current USDC balance (in smallest unit)
    pub current_usdc_balance: u64,
    /// Total SOL withdrawn (in lamports)
    pub total_sol_withdrawn: u64,
    /// Total USDC withdrawn (in smallest unit)
    pub total_usdc_withdrawn: u64,
    /// SOL share percentage (in basis points)
    pub sol_share_percentage: u16,
    /// USDC share percentage (in basis points)
    pub usdc_share_percentage: u16,
    /// Number of deposits made
    pub deposit_count: u32,
    /// Number of withdrawals made
    pub withdrawal_count: u32,
    /// Timestamp of first deposit
    pub first_deposit_timestamp: i64,
    /// Timestamp of last activity
    pub last_activity_timestamp: i64,
}

/// Response structure for vault query
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct VaultInfo {
    /// Total SOL deposited across all investors (in lamports)
    pub total_sol_deposited: u64,
    /// Total USDC deposited across all investors (in smallest unit)
    pub total_usdc_deposited: u64,
    /// Current total SOL balance in vault (in lamports)
    pub current_total_sol: u64,
    /// Current total USDC balance in vault (in smallest unit)
    pub current_total_usdc: u64,
    /// Total SOL withdrawn across all investors (in lamports)
    pub total_sol_withdrawn: u64,
    /// Total USDC withdrawn across all investors (in smallest unit)
    pub total_usdc_withdrawn: u64,
    /// Number of unique depositors
    pub depositor_count: u32,
    /// Timestamp of last update
    pub last_update_timestamp: i64,
}

/// Query instruction to get vault information
#[derive(Accounts)]
pub struct QueryVault<'info> {
    /// CHECK: Program authority (our program)
    #[account(
        seeds = [b"fee_collector"],
        bump
    )]
    pub fee_collector: UncheckedAccount<'info>,
    
    /// Global vault statistics
    #[account(
        seeds = [DEPOSIT_VAULT_SEED, b"stats"],
        bump = vault_stats.bump
    )]
    pub vault_stats: Account<'info, VaultStats>,
}

impl<'info> QueryDepositor<'info> {
    pub fn handle(ctx: Context<QueryDepositor>) -> Result<DepositorInfo> {
        let depositor_record = &ctx.accounts.depositor_record;
        let vault_stats = &ctx.accounts.vault_stats;
        
        // Calculate share percentage
        let share_percentage = depositor_record.calculate_share_percentage(
            vault_stats.get_current_sol_balance(),
            vault_stats.get_current_usdc_balance()
        )?;
        
        Ok(DepositorInfo {
            investor: depositor_record.investor,
            total_sol_deposited: depositor_record.total_sol_deposited,
            total_usdc_deposited: depositor_record.total_usdc_deposited,
            current_sol_balance: depositor_record.current_sol_balance,
            current_usdc_balance: depositor_record.current_usdc_balance,
            total_sol_withdrawn: depositor_record.total_sol_withdrawn,
            total_usdc_withdrawn: depositor_record.total_usdc_withdrawn,
            sol_share_percentage: share_percentage,
            usdc_share_percentage: share_percentage,
            deposit_count: depositor_record.deposit_count,
            withdrawal_count: depositor_record.withdrawal_count,
            first_deposit_timestamp: depositor_record.first_deposit_timestamp,
            last_activity_timestamp: depositor_record.last_activity_timestamp,
        })
    }
}

impl<'info> QueryVault<'info> {
    pub fn handle(ctx: Context<QueryVault>) -> Result<VaultInfo> {
        let vault_stats = &ctx.accounts.vault_stats;
        
        Ok(VaultInfo {
            total_sol_deposited: vault_stats.total_sol_deposited,
            total_usdc_deposited: vault_stats.total_usdc_deposited,
            current_total_sol: vault_stats.current_total_sol,
            current_total_usdc: vault_stats.current_total_usdc,
            total_sol_withdrawn: vault_stats.total_sol_withdrawn,
            total_usdc_withdrawn: vault_stats.total_usdc_withdrawn,
            depositor_count: vault_stats.depositor_count,
            last_update_timestamp: vault_stats.last_update_timestamp,
        })
    }
}