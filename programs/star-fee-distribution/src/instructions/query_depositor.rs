use anchor_lang::prelude::*;
use crate::constants::{INVESTOR_RECORD_SEED, DEPOSIT_VAULT_SEED, FEE_COLLECTOR_SEED};
use crate::states::{DepositorRecord, VaultStats};
use super::depositor_record::{DepositorInfo, VaultInfo};

/// Query instruction to get depositor information and share calculations
#[derive(Accounts)]
pub struct QueryDepositor<'info> {
    /// The investor to query
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

impl<'info> QueryDepositor<'info> {
    pub fn handle(ctx: Context<QueryDepositor>) -> Result<DepositorInfo> {
        let depositor_record = &ctx.accounts.depositor_record;
        let vault_stats = &ctx.accounts.vault_stats;
        
        // Calculate share percentage
        let share_percentage = depositor_record.calculate_share_percentage(
            vault_stats.current_total_sol,
            vault_stats.current_total_usdc
        )?;
        
        let depositor_info = DepositorInfo {
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
        };
        
        msg!("Depositor info for investor: {}", depositor_info.investor);
        msg!("Total SOL deposited: {} lamports", depositor_info.total_sol_deposited);
        msg!("Total USDC deposited: {} units", depositor_info.total_usdc_deposited);
        msg!("Current SOL balance: {} lamports", depositor_info.current_sol_balance);
        msg!("Current USDC balance: {} units", depositor_info.current_usdc_balance);
        msg!("SOL share: {} bps", depositor_info.sol_share_percentage);
        msg!("USDC share: {} bps", depositor_info.usdc_share_percentage);
        msg!("Deposit count: {}", depositor_info.deposit_count);
        msg!("Withdrawal count: {}", depositor_info.withdrawal_count);
        
        Ok(depositor_info)
    }
}

/// Query instruction to get global vault information
#[derive(Accounts)]
pub struct QueryVault<'info> {
    /// CHECK: Program authority (our program)
    #[account(
        seeds = [FEE_COLLECTOR_SEED],
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

impl<'info> QueryVault<'info> {
    pub fn handle(ctx: Context<QueryVault>) -> Result<VaultInfo> {
        let vault_stats = &ctx.accounts.vault_stats;
        
        let vault_info = VaultInfo {
            total_sol_deposited: vault_stats.total_sol_deposited,
            total_usdc_deposited: vault_stats.total_usdc_deposited,
            current_total_sol: vault_stats.current_total_sol,
            current_total_usdc: vault_stats.current_total_usdc,
            total_sol_withdrawn: vault_stats.total_sol_withdrawn,
            total_usdc_withdrawn: vault_stats.total_usdc_withdrawn,
            depositor_count: vault_stats.depositor_count,
            last_update_timestamp: vault_stats.last_update_timestamp,
        };
        
        msg!("Vault info:");
        msg!("Total SOL deposited: {} lamports", vault_info.total_sol_deposited);
        msg!("Total USDC deposited: {} units", vault_info.total_usdc_deposited);
        msg!("Current total SOL: {} lamports", vault_info.current_total_sol);
        msg!("Current total USDC: {} units", vault_info.current_total_usdc);
        msg!("Total SOL withdrawn: {} lamports", vault_info.total_sol_withdrawn);
        msg!("Total USDC withdrawn: {} units", vault_info.total_usdc_withdrawn);
        msg!("Number of depositors: {}", vault_info.depositor_count);
        
        Ok(vault_info)
    }
}
