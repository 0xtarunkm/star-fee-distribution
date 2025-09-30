use anchor_lang::prelude::*;
use crate::errors::ErrorCode;

/// Depositor record to track individual investor's deposits and shares
#[account]
#[derive(InitSpace)]
pub struct DepositorRecord {
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
    /// Timestamp of first deposit
    pub first_deposit_timestamp: i64,
    /// Timestamp of last activity
    pub last_activity_timestamp: i64,
    /// Number of deposits made
    pub deposit_count: u32,
    /// Number of withdrawals made
    pub withdrawal_count: u32,
    /// Bump seed for the PDA
    pub bump: u8,
}

impl DepositorRecord {
    /// Creates a new depositor record
    pub fn new(investor: Pubkey, bump: u8) -> Self {
        let now = Clock::get().unwrap().unix_timestamp;
        Self {
            investor,
            total_sol_deposited: 0,
            total_usdc_deposited: 0,
            current_sol_balance: 0,
            current_usdc_balance: 0,
            total_sol_withdrawn: 0,
            total_usdc_withdrawn: 0,
            first_deposit_timestamp: now,
            last_activity_timestamp: now,
            deposit_count: 0,
            withdrawal_count: 0,
            bump,
        }
    }

    /// Updates the record with a new deposit
    pub fn add_deposit(&mut self, sol_amount: u64, usdc_amount: u64) -> Result<()> {
        let now = Clock::get().unwrap().unix_timestamp;
        
        // Update totals
        self.total_sol_deposited = self.total_sol_deposited
            .checked_add(sol_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        self.total_usdc_deposited = self.total_usdc_deposited
            .checked_add(usdc_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        // Update current balances
        self.current_sol_balance = self.current_sol_balance
            .checked_add(sol_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        self.current_usdc_balance = self.current_usdc_balance
            .checked_add(usdc_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        // Update timestamps
        self.last_activity_timestamp = now;
        if self.deposit_count == 0 {
            self.first_deposit_timestamp = now;
        }
        
        // Increment deposit count
        self.deposit_count = self.deposit_count
            .checked_add(1)
            .ok_or(ErrorCode::MathOverflow)?;
        
        Ok(())
    }

    /// Updates the record with a withdrawal
    pub fn add_withdrawal(&mut self, sol_amount: u64, usdc_amount: u64) -> Result<()> {
        let now = Clock::get().unwrap().unix_timestamp;
        
        // Validate sufficient balance
        require!(
            sol_amount <= self.current_sol_balance,
            ErrorCode::InsufficientBalance
        );
        require!(
            usdc_amount <= self.current_usdc_balance,
            ErrorCode::InsufficientBalance
        );
        
        // Update totals
        self.total_sol_withdrawn = self.total_sol_withdrawn
            .checked_add(sol_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        self.total_usdc_withdrawn = self.total_usdc_withdrawn
            .checked_add(usdc_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        // Update current balances
        self.current_sol_balance = self.current_sol_balance
            .checked_sub(sol_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        self.current_usdc_balance = self.current_usdc_balance
            .checked_sub(usdc_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        // Update timestamp
        self.last_activity_timestamp = now;
        
        // Increment withdrawal count
        self.withdrawal_count = self.withdrawal_count
            .checked_add(1)
            .ok_or(ErrorCode::MathOverflow)?;
        
        Ok(())
    }

    /// Calculates the investor's share percentage based on their deposits
    pub fn calculate_share_percentage(&self, total_sol: u64, total_usdc: u64) -> Result<u16> {
        if total_sol == 0 && total_usdc == 0 {
            return Ok(0);
        }
        
        // Calculate weighted share based on both SOL and USDC deposits
        let sol_weight = if total_sol > 0 {
            (self.total_sol_deposited as u128 * 10000) / (total_sol as u128)
        } else {
            0
        };
        
        let usdc_weight = if total_usdc > 0 {
            (self.total_usdc_deposited as u128 * 10000) / (total_usdc as u128)
        } else {
            0
        };
        
        // Use the higher of the two weights (investor gets credit for their stronger position)
        let share_percentage = sol_weight.max(usdc_weight);
        
        // Cap at 100% (10000 basis points)
        Ok(share_percentage.min(10000) as u16)
    }

    /// Checks if the investor has any deposits
    pub fn has_deposits(&self) -> bool {
        self.total_sol_deposited > 0 || self.total_usdc_deposited > 0
    }

    /// Gets the total value of deposits (simplified calculation)
    pub fn get_total_deposit_value(&self) -> u64 {
        // This is a simplified calculation - in practice you'd want to use price feeds
        // For now, we'll just return the SOL amount as the primary value
        self.total_sol_deposited
    }
}
