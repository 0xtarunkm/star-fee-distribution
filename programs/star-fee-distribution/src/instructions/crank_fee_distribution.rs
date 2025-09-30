use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::errors::ErrorCode;
use crate::constants::{FEE_COLLECTOR_SEED, FEE_VAULT_SEED, DEPOSIT_VAULT_SEED, INVESTOR_RECORD_SEED, CRANK_STATE_SEED, DISTRIBUTION_CONFIG_SEED};
use crate::states::{DepositorRecord, VaultStats, DistributionConfig, CrankState};


/// Crank instruction to distribute fees to all investors based on their shares
#[derive(Accounts)]
pub struct CrankFeeDistribution<'info> {
    /// Payer for account initialization
    #[account(mut)]
    pub payer: Signer<'info>,

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
    
    /// Global vault statistics
    #[account(
        mut,
        seeds = [DEPOSIT_VAULT_SEED, b"stats"],
        bump = vault_stats.bump
    )]
    pub vault_stats: Account<'info, VaultStats>,
    
    /// Distribution configuration
    #[account(
        seeds = [DISTRIBUTION_CONFIG_SEED],
        bump = distribution_config.bump
    )]
    pub distribution_config: Account<'info, DistributionConfig>,
    
    /// Crank state to track distribution timing
    #[account(
        init_if_needed,
        payer = payer,
        space = CrankState::DISCRIMINATOR.len() + CrankState::INIT_SPACE,
        seeds = [CRANK_STATE_SEED],
        bump
    )]
    pub crank_state: Account<'info, CrankState>,
    
    /// CHECK: Token program
    pub token_program: Program<'info, Token>,
    
    /// CHECK: System program
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DistributionParams {
    /// Page index for pagination
    pub page_index: u32,
    /// Number of investors in this page
    pub investors_count: u32,
    /// Is this the final page of the day?
    pub is_final_page: bool,
}

impl<'info> CrankFeeDistribution<'info> {
    pub fn handle(ctx: Context<CrankFeeDistribution>, params: DistributionParams) -> Result<()> {
        msg!("Starting crank fee distribution - Page: {}", params.page_index);
        
        let config = &ctx.accounts.distribution_config;
        let crank_state = &mut ctx.accounts.crank_state;
        let vault_stats = &ctx.accounts.vault_stats;
        
        // Initialize crank state if needed
        if crank_state.last_distribution_timestamp == 0 {
            crank_state.last_distribution_timestamp = 0;
            crank_state.current_day = 0;
            crank_state.distribution_count = 0;
            crank_state.pagination_cursor = 0;
            crank_state.investors_processed_today = 0;
            crank_state.daily_distributed = 0;
            crank_state.carry_over = 0;
            crank_state.day_state = 0;
            crank_state.bump = ctx.bumps.crank_state;
        }
        
        // Start new day if needed
        if !crank_state.is_day_in_progress() {
            require!(!crank_state.is_day_closed(), ErrorCode::DayAlreadyClosed);
            crank_state.start_new_day()?;
        }
        
        // Validate pagination cursor
        require!(
            params.page_index == crank_state.pagination_cursor,
            ErrorCode::InvalidPaginationCursor
        );
        
        // QUOTE-ONLY ENFORCEMENT: Fail if base fees detected
        let base_fees_available = ctx.accounts.program_token_a_vault.amount;
        let quote_fees_available = ctx.accounts.program_token_b_vault.amount;
        
        msg!("Available base fees: {} units", base_fees_available);
        msg!("Available quote fees: {} units", quote_fees_available);
        
        // Hard requirement: Reject any base fees
        require!(
            base_fees_available == 0,
            ErrorCode::BaseFeesDetected
        );
        
        // Check if there are quote fees to distribute
        require!(
            quote_fees_available > 0,
            ErrorCode::NoFeesToClaim
        );
        
        // Validate quote mint matches config
        require!(
            ctx.accounts.quote_mint.key() == config.quote_mint,
            ErrorCode::InvalidPosition
        );
        
        // Calculate total locked amounts (from current depositor balances)
        let locked_total = vault_stats.current_total_usdc; // Using USDC as quote token
        
        msg!("Total locked (depositor balances): {} units", locked_total);
        msg!("Y0 allocation: {} units", config.y0_allocation);
        
        // Calculate f_locked(t) = locked_total(t) / Y0
        let f_locked_bps = if config.y0_allocation > 0 {
            ((locked_total as u128 * 10000) / config.y0_allocation as u128) as u16
        } else {
            0
        };
        
        msg!("f_locked: {} bps", f_locked_bps);
        
        // Calculate eligible_investor_share_bps = min(investor_fee_share_bps, f_locked_bps)
        let eligible_investor_share_bps = std::cmp::min(config.investor_fee_share_bps, f_locked_bps);
        
        msg!("Eligible investor share: {} bps (max: {} bps)", 
            eligible_investor_share_bps, config.investor_fee_share_bps);
        
        // Calculate investor_fee_quote = floor(claimed_quote * eligible_investor_share_bps / 10000)
        let investor_fee_quote = ((quote_fees_available as u128 * eligible_investor_share_bps as u128) / 10000) as u64;
        
        msg!("Total investor allocation: {} units", investor_fee_quote);
        
        // Add carry-over from previous page
        let total_distributable = investor_fee_quote.checked_add(crank_state.carry_over)
            .ok_or(ErrorCode::MathOverflow)?;
        
        msg!("Total distributable (with carry-over): {} units", total_distributable);
        
        // Check daily cap if configured
        if config.daily_cap_lamports > 0 {
            let remaining_cap = config.daily_cap_lamports
                .checked_sub(crank_state.daily_distributed)
                .ok_or(ErrorCode::DailyCapExceeded)?;
            
            require!(
                remaining_cap > 0,
                ErrorCode::DailyCapExceeded
            );
            
            msg!("Remaining daily cap: {} units", remaining_cap);
        }
        
        // Advance cursor
        crank_state.advance_cursor(params.investors_count)?;
        
        msg!("Crank fee distribution page completed!");
        msg!("Investors processed this page: {}", params.investors_count);
        msg!("Total investors processed today: {}", crank_state.investors_processed_today);
        msg!("Day state: {}", crank_state.day_state);
        
        // Emit event
        emit!(crate::events::InvestorPayoutPage {
            day: crank_state.current_day,
            page_index: params.page_index,
            investors_count: params.investors_count,
            total_investors_processed_today: crank_state.investors_processed_today,
            quote_fees_available,
            total_locked: locked_total,
            y0_allocation: config.y0_allocation,
            f_locked_bps,
            eligible_investor_share_bps,
            investor_fee_quote,
            page_distributed: 0, // This will be updated by individual investor payouts
            carry_over: crank_state.carry_over,
            daily_distributed: crank_state.daily_distributed,
            daily_cap: config.daily_cap_lamports,
            is_final_page: params.is_final_page,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }
}

/// Calculates investor payout with dust handling
pub fn calculate_investor_payout(
    investor_balance: u64,
    total_locked: u64,
    total_investor_fee: u64,
    min_payout: u64,
) -> Result<(u64, u64)> {
    // Calculate weight_i(t) = locked_i(t) / locked_total(t)
    let weight_bps = if total_locked > 0 {
        ((investor_balance as u128 * 10000) / total_locked as u128) as u64
    } else {
        0
    };
    
    // Calculate payout = floor(investor_fee_quote * weight_i(t))
    let payout = ((total_investor_fee as u128 * weight_bps as u128) / 10000) as u64;
    
    // Apply dust threshold
    let (actual_payout, dust) = if payout < min_payout {
        msg!("Payout {} below minimum {}, carrying as dust", payout, min_payout);
        (0, payout)
    } else {
        (payout, 0)
    };
    
    Ok((actual_payout, dust))
}

/// Individual fee distribution instruction for a specific investor
#[derive(Accounts)]
pub struct DistributeToInvestor<'info> {
    /// CHECK: Program authority (our program)
    #[account(
        mut,
        seeds = [FEE_COLLECTOR_SEED],
        bump
    )]
    pub fee_collector: UncheckedAccount<'info>,
    
    /// Program's quote token vault for fee collection (QUOTE ONLY)
    #[account(
        mut,
        seeds = [FEE_VAULT_SEED, quote_mint.key().as_ref()],
        bump,
        token::mint = quote_mint,
        token::authority = fee_collector
    )]
    pub program_quote_vault: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: Quote token mint
    #[account(mut)]
    pub quote_mint: UncheckedAccount<'info>,
    
    /// Investor's quote token account
    #[account(mut)]
    pub investor_quote_account: Box<Account<'info, TokenAccount>>,
    
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
        seeds = [DEPOSIT_VAULT_SEED, b"stats"],
        bump = vault_stats.bump
    )]
    pub vault_stats: Account<'info, VaultStats>,
    
    /// Distribution configuration
    #[account(
        seeds = [DISTRIBUTION_CONFIG_SEED],
        bump = distribution_config.bump
    )]
    pub distribution_config: Account<'info, DistributionConfig>,
    
    /// Crank state for tracking
    #[account(
        mut,
        seeds = [CRANK_STATE_SEED],
        bump = crank_state.bump
    )]
    pub crank_state: Account<'info, CrankState>,
    
    /// The investor receiving the distribution
    pub investor: Signer<'info>,
    
    /// CHECK: Token program
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InvestorDistributionParams {
    /// Total investor fee amount for this page (from crank calculation)
    pub total_investor_fee: u64,
}

impl<'info> DistributeToInvestor<'info> {
    pub fn handle(ctx: Context<DistributeToInvestor>, params: InvestorDistributionParams) -> Result<()> {
        msg!("Distributing quote fees to investor: {}", ctx.accounts.investor.key());
        
        let depositor_record = &ctx.accounts.depositor_record;
        let vault_stats = &ctx.accounts.vault_stats;
        let config = &ctx.accounts.distribution_config;
        let crank_state = &mut ctx.accounts.crank_state;
        
        // Ensure distribution is in progress
        require!(
            crank_state.is_day_in_progress(),
            ErrorCode::DistributionNotStarted
        );
        
        // Get investor's current balance (locked amount)
        let investor_balance = depositor_record.current_usdc_balance;
        let total_locked = vault_stats.current_total_usdc;
        
        msg!("Investor balance: {} units", investor_balance);
        msg!("Total locked: {} units", total_locked);
        
        // Calculate investor payout with dust handling
        let (payout, dust) = calculate_investor_payout(
            investor_balance,
            total_locked,
            params.total_investor_fee,
            config.min_payout_lamports,
        )?;
        
        msg!("Calculated payout: {} units", payout);
        msg!("Dust amount: {} units", dust);
        
        // Distribute quote tokens if payout > 0
        if payout > 0 {
            // Check daily cap if configured
            if config.daily_cap_lamports > 0 {
                let new_total = crank_state.daily_distributed
                    .checked_add(payout)
                    .ok_or(ErrorCode::MathOverflow)?;
                
                require!(
                    new_total <= config.daily_cap_lamports,
                    ErrorCode::DailyCapExceeded
                );
            }
            
            // Transfer quote tokens
            anchor_spl::token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    anchor_spl::token::Transfer {
                        from: ctx.accounts.program_quote_vault.to_account_info(),
                        to: ctx.accounts.investor_quote_account.to_account_info(),
                        authority: ctx.accounts.fee_collector.to_account_info(),
                    },
                    &[&[
                        FEE_COLLECTOR_SEED,
                        &[ctx.bumps.fee_collector]
                    ]]
                ),
                payout,
            )?;
            
            // Update daily distributed
            crank_state.daily_distributed = crank_state.daily_distributed
                .checked_add(payout)
                .ok_or(ErrorCode::MathOverflow)?;
            
            msg!("Distributed {} quote tokens to investor", payout);
            msg!("Total distributed today: {} units", crank_state.daily_distributed);
        }
        
        // Update carry-over with dust
        if dust > 0 {
            crank_state.carry_over = crank_state.carry_over
                .checked_add(dust)
                .ok_or(ErrorCode::MathOverflow)?;
            msg!("Updated carry-over: {} units", crank_state.carry_over);
        }
        
        msg!("Quote fee distribution to investor completed!");
        
        // Emit event
        let weight_bps = if total_locked > 0 {
            ((investor_balance as u128 * 10000) / total_locked as u128) as u64
        } else {
            0
        };
        
        emit!(crate::events::InvestorPayout {
            day: crank_state.current_day,
            investor: ctx.accounts.investor.key(),
            investor_locked_balance: investor_balance,
            total_locked,
            weight_bps,
            total_investor_fee: params.total_investor_fee,
            calculated_payout: payout + dust,
            actual_payout: payout,
            dust,
            min_payout: config.min_payout_lamports,
            investor_quote_account: ctx.accounts.investor_quote_account.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }
}

/// Close day and route remainder to creator
#[derive(Accounts)]
pub struct RouteCreatorRemainder<'info> {
    /// CHECK: Program authority (our program)
    #[account(
        mut,
        seeds = [FEE_COLLECTOR_SEED],
        bump
    )]
    pub fee_collector: UncheckedAccount<'info>,
    
    /// Program's quote token vault for fee collection
    #[account(
        mut,
        seeds = [FEE_VAULT_SEED, quote_mint.key().as_ref()],
        bump,
        token::mint = quote_mint,
        token::authority = fee_collector
    )]
    pub program_quote_vault: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: Quote token mint
    #[account(mut)]
    pub quote_mint: UncheckedAccount<'info>,
    
    /// Creator's quote token account
    #[account(
        mut,
        constraint = creator_quote_account.owner == distribution_config.creator_wallet
    )]
    pub creator_quote_account: Box<Account<'info, TokenAccount>>,
    
    /// Distribution configuration
    #[account(
        seeds = [DISTRIBUTION_CONFIG_SEED],
        bump = distribution_config.bump
    )]
    pub distribution_config: Account<'info, DistributionConfig>,
    
    /// Crank state for tracking
    #[account(
        mut,
        seeds = [CRANK_STATE_SEED],
        bump = crank_state.bump
    )]
    pub crank_state: Account<'info, CrankState>,
    
    /// CHECK: Token program
    pub token_program: Program<'info, Token>,
}

impl<'info> RouteCreatorRemainder<'info> {
    pub fn handle(ctx: Context<RouteCreatorRemainder>) -> Result<()> {
        msg!("Routing creator remainder and closing day");
        
        let crank_state = &mut ctx.accounts.crank_state;
        
        // Ensure day is in progress
        require!(
            crank_state.is_day_in_progress(),
            ErrorCode::DistributionNotStarted
        );
        
        // Get remaining balance (this is the creator's remainder)
        let remainder = ctx.accounts.program_quote_vault.amount;
        
        msg!("Creator remainder: {} units", remainder);
        msg!("Carry-over dust: {} units", crank_state.carry_over);
        
        if remainder > 0 {
            // Transfer remainder to creator
            anchor_spl::token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    anchor_spl::token::Transfer {
                        from: ctx.accounts.program_quote_vault.to_account_info(),
                        to: ctx.accounts.creator_quote_account.to_account_info(),
                        authority: ctx.accounts.fee_collector.to_account_info(),
                    },
                    &[&[
                        FEE_COLLECTOR_SEED,
                        &[ctx.bumps.fee_collector]
                    ]]
                ),
                remainder,
            )?;
            
            msg!("Distributed {} quote tokens to creator", remainder);
        }
        
        // Close the day
        crank_state.close_day()?;
        
        msg!("Day {} closed successfully", crank_state.current_day);
        msg!("Total investors processed: {}", crank_state.investors_processed_today);
        msg!("Total distributed to investors: {} units", crank_state.daily_distributed);
        msg!("Creator received: {} units", remainder);
        
        // Emit event
        emit!(crate::events::CreatorPayoutDayClosed {
            day: crank_state.current_day,
            creator_wallet: ctx.accounts.distribution_config.creator_wallet,
            creator_quote_account: ctx.accounts.creator_quote_account.key(),
            creator_remainder: remainder,
            total_distributed_to_investors: crank_state.daily_distributed,
            total_investors_processed: crank_state.investors_processed_today,
            final_carry_over: crank_state.carry_over,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }
}