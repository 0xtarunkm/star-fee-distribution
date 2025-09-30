use anchor_lang::prelude::*;
use crate::errors::ErrorCode;

/// Crank state to track the last distribution time
#[account]
#[derive(InitSpace)]
pub struct CrankState {
    /// Last distribution timestamp
    pub last_distribution_timestamp: i64,
    /// Current day number (incremented each distribution)
    pub current_day: u32,
    /// Total distributions made
    pub distribution_count: u32,
    /// Current pagination cursor within the day
    pub pagination_cursor: u32,
    /// Total investors processed in current day
    pub investors_processed_today: u32,
    /// Amount distributed in current day
    pub daily_distributed: u64,
    /// Carry-over from previous distribution (dust)
    pub carry_over: u64,
    /// Day state: 0=not started, 1=in progress, 2=closed
    pub day_state: u8,
    /// Bump seed for the PDA
    pub bump: u8,
}

impl CrankState {
    /// Creates new crank state
    pub fn new(bump: u8) -> Self {
        Self {
            last_distribution_timestamp: 0,
            current_day: 0,
            distribution_count: 0,
            pagination_cursor: 0,
            investors_processed_today: 0,
            daily_distributed: 0,
            carry_over: 0,
            day_state: 0, // not started
            bump,
        }
    }

    /// Checks if 24 hours have passed since last distribution
    pub fn can_start_new_day(&self) -> Result<bool> {
        let now = Clock::get()?.unix_timestamp;
        let time_since_last = now - self.last_distribution_timestamp;
        
        // 24 hours = 86400 seconds
        Ok(time_since_last >= 86400 || self.last_distribution_timestamp == 0)
    }

    /// Starts a new distribution day
    pub fn start_new_day(&mut self) -> Result<()> {
        require!(self.can_start_new_day()?, ErrorCode::DistributionTooFrequent);
        
        let now = Clock::get()?.unix_timestamp;
        self.last_distribution_timestamp = now;
        self.current_day = self.current_day.checked_add(1).ok_or(ErrorCode::MathOverflow)?;
        self.pagination_cursor = 0;
        self.investors_processed_today = 0;
        self.daily_distributed = 0;
        self.day_state = 1; // in progress
        
        msg!("Started new distribution day: {}", self.current_day);
        Ok(())
    }

    /// Advances pagination cursor
    pub fn advance_cursor(&mut self, investors_processed: u32) -> Result<()> {
        self.pagination_cursor = self.pagination_cursor.checked_add(1).ok_or(ErrorCode::MathOverflow)?;
        self.investors_processed_today = self.investors_processed_today
            .checked_add(investors_processed)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Closes the current day
    pub fn close_day(&mut self) -> Result<()> {
        self.day_state = 2; // closed
        self.distribution_count = self.distribution_count.checked_add(1).ok_or(ErrorCode::MathOverflow)?;
        msg!("Closed distribution day: {}", self.current_day);
        Ok(())
    }

    /// Checks if day is in progress
    pub fn is_day_in_progress(&self) -> bool {
        self.day_state == 1
    }

    /// Checks if day is closed
    pub fn is_day_closed(&self) -> bool {
        self.day_state == 2
    }
}
