use anchor_lang::prelude::*;
use crate::errors::ErrorCode;

/// Global vault statistics to track total deposits across all investors
#[account]
#[derive(InitSpace)]
pub struct VaultStats {
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
    /// Bump seed for the PDA
    pub bump: u8,
}

impl VaultStats {
    /// Creates new vault stats
    pub fn new(bump: u8) -> Self {
        Self {
            total_sol_deposited: 0,
            total_usdc_deposited: 0,
            current_total_sol: 0,
            current_total_usdc: 0,
            total_sol_withdrawn: 0,
            total_usdc_withdrawn: 0,
            depositor_count: 0,
            last_update_timestamp: 0,
            bump,
        }
    }

    /// Adds a new deposit to the vault stats
    pub fn add_deposits(&mut self, sol_amount: u64, usdc_amount: u64) -> Result<()> {
        let now = Clock::get().unwrap().unix_timestamp;
        
        // Update totals
        self.total_sol_deposited = self.total_sol_deposited
            .checked_add(sol_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        self.total_usdc_deposited = self.total_usdc_deposited
            .checked_add(usdc_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        // Update current balances
        self.current_total_sol = self.current_total_sol
            .checked_add(sol_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        self.current_total_usdc = self.current_total_usdc
            .checked_add(usdc_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        // Update timestamp
        self.last_update_timestamp = now;
        
        Ok(())
    }

    /// Adds a new withdrawal to the vault stats
    pub fn add_withdrawals(&mut self, sol_amount: u64, usdc_amount: u64) -> Result<()> {
        let now = Clock::get().unwrap().unix_timestamp;
        
        // Update totals
        self.total_sol_withdrawn = self.total_sol_withdrawn
            .checked_add(sol_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        self.total_usdc_withdrawn = self.total_usdc_withdrawn
            .checked_add(usdc_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        // Update current balances
        self.current_total_sol = self.current_total_sol
            .checked_sub(sol_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        self.current_total_usdc = self.current_total_usdc
            .checked_sub(usdc_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        // Update timestamp
        self.last_update_timestamp = now;
        
        Ok(())
    }

    /// Gets the current SOL balance in the vault
    pub fn get_current_sol_balance(&self) -> u64 {
        self.current_total_sol
    }

    /// Gets the current USDC balance in the vault
    pub fn get_current_usdc_balance(&self) -> u64 {
        self.current_total_usdc
    }

    /// Checks if the vault has any deposits
    pub fn has_deposits(&self) -> bool {
        self.total_sol_deposited > 0 || self.total_usdc_deposited > 0
    }

    /// Gets the total value of all deposits (simplified calculation)
    pub fn get_total_deposit_value(&self) -> u64 {
        // This is a simplified calculation - in practice you'd want to use price feeds
        // For now, we'll just return the SOL amount as the primary value
        self.total_sol_deposited
    }
}
