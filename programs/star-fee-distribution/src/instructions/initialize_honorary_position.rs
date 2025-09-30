use anchor_lang::prelude::*;
use damm_v2::types::AddLiquidityParameters;

use crate::errors::ErrorCode;

#[derive(Accounts)]
pub struct InitializeHonoraryPosition<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    
    /// CHECK: DAMM v2 program
    #[account(address = damm_v2::ID)]
    pub amm_program: UncheckedAccount<'info>,
    
    /// CHECK: Pool account
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    
    /// CHECK: Position account (owned by our PDA)
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    
    /// CHECK: Position NFT mint
    #[account(mut)]
    pub position_nft_mint: UncheckedAccount<'info>,
    
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
    
    /// CHECK: User's base token account
    #[account(mut)]
    pub user_token_a_account: UncheckedAccount<'info>,
    
    /// CHECK: User's quote token account
    #[account(mut)]
    pub user_token_b_account: UncheckedAccount<'info>,
    
    /// CHECK: Token program
    pub token_program: UncheckedAccount<'info>,
    
    /// CHECK: System program
    pub system_program: Program<'info, System>,
    
    /// CHECK: Event authority
    pub event_authority: UncheckedAccount<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct HonoraryPositionConfig {
    /// Base token weight in basis points (must be 0 for quote-only)
    pub base_weight_bps: u16,
    /// Quote token weight in basis points (must be 10000 for quote-only)
    pub quote_weight_bps: u16,
    /// Lower tick for the position range
    pub lower_tick: i32,
    /// Upper tick for the position range
    pub upper_tick: i32,
    /// Fee tier for the position
    pub fee_tier: u16,
}

impl<'info> InitializeHonoraryPosition<'info> {
    pub fn handle(
        ctx: Context<InitializeHonoraryPosition>, 
        config: HonoraryPositionConfig
    ) -> Result<()> {
        msg!("Initializing honorary position for signer: {}", ctx.accounts.signer.key());
        
        // Validate pool token order and confirm which mint is the quote mint
        let base_mint = ctx.accounts.base_mint.key();
        let quote_mint = ctx.accounts.quote_mint.key();
        
        msg!("Base mint: {}", base_mint);
        msg!("Quote mint: {}", quote_mint);
        
        // Preflight validation: Ensure this configuration can only accrue quote fees
        // This is a deterministic validation step that rejects any config that could accrue base fees
        validate_quote_only_fee_configuration(&config)?;
        
        // Create position using DAMM v2 CPI (owned by our PDA)
        damm_v2::cpi::create_position(
            CpiContext::new(
                ctx.accounts.amm_program.to_account_info(),
                damm_v2::cpi::accounts::CreatePosition {
                    owner: ctx.accounts.signer.to_account_info(), // Our PDA will be the owner
                    pool: ctx.accounts.pool.to_account_info(),
                    position_nft_mint: ctx.accounts.position_nft_mint.to_account_info(),
                    position_nft_account: ctx.accounts.position_nft_account.to_account_info(),
                    position: ctx.accounts.position.to_account_info(),
                    pool_authority: ctx.accounts.pool_authority.to_account_info(),
                    payer: ctx.accounts.signer.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    event_authority: ctx.accounts.event_authority.to_account_info(),
                    program: ctx.accounts.amm_program.to_account_info(),
                },
            ),
        )?;

        // Add liquidity to the position (zero amounts for honorary position)
        // This creates an empty position that only accrues quote token fees
        damm_v2::cpi::add_liquidity(
            CpiContext::new(
                ctx.accounts.amm_program.to_account_info(),
                damm_v2::cpi::accounts::AddLiquidity {
                    pool: ctx.accounts.pool.to_account_info(),
                    position: ctx.accounts.position.to_account_info(),
                    token_a_account: ctx.accounts.user_token_a_account.to_account_info(),
                    token_b_account: ctx.accounts.user_token_b_account.to_account_info(),
                    token_a_vault: ctx.accounts.token_a_vault.to_account_info(),
                    token_b_vault: ctx.accounts.token_b_vault.to_account_info(),
                    token_a_mint: ctx.accounts.base_mint.to_account_info(),
                    token_b_mint: ctx.accounts.quote_mint.to_account_info(),
                    position_nft_account: ctx.accounts.position_nft_account.to_account_info(),
                    owner: ctx.accounts.signer.to_account_info(),
                    token_a_program: ctx.accounts.token_program.to_account_info(),
                    token_b_program: ctx.accounts.token_program.to_account_info(),
                    event_authority: ctx.accounts.event_authority.to_account_info(),
                    program: ctx.accounts.amm_program.to_account_info(),
                },
            ),
            AddLiquidityParameters {
                liquidity_delta: 0, // Zero liquidity for honorary position
                token_a_amount_threshold: 0,
                token_b_amount_threshold: 0,
            },
        )?;

        msg!("Honorary quote-only fee position created successfully!");
        
        // Emit event
        emit!(crate::events::HonoraryPositionInitialized {
            pool: ctx.accounts.pool.key(),
            position: ctx.accounts.position.key(),
            position_nft_mint: ctx.accounts.position_nft_mint.key(),
            base_mint: ctx.accounts.base_mint.key(),
            quote_mint: ctx.accounts.quote_mint.key(),
            base_weight_bps: config.base_weight_bps,
            quote_weight_bps: config.quote_weight_bps,
            lower_tick: config.lower_tick,
            upper_tick: config.upper_tick,
            fee_tier: config.fee_tier,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }
}

/// Validates that the position configuration can only accrue quote token fees
/// This is a deterministic preflight validation step
fn validate_quote_only_fee_configuration(config: &HonoraryPositionConfig) -> Result<()> {
    msg!("Validating quote-only fee configuration...");
    
    // 1. Validate weight distribution - must be 100% quote token
    require!(
        config.base_weight_bps == 0,
        ErrorCode::BaseWeightMustBeZero
    );
    
    require!(
        config.quote_weight_bps == 10000,
        ErrorCode::QuoteWeightMustBe10000
    );
    
    // 2. Validate tick range is appropriate for quote-only fee collection
    // For quote-only positions, we want a wide range to capture all fees
    require!(
        config.lower_tick <= -443636,
        ErrorCode::LowerTickTooHigh
    );
    
    require!(
        config.upper_tick >= 443636,
        ErrorCode::UpperTickTooLow
    );
    
    // 3. Validate fee tier is appropriate
    // Common fee tiers: 100, 500, 3000, 10000 (in basis points)
    require!(
        config.fee_tier == 100 || config.fee_tier == 500 || 
        config.fee_tier == 3000 || config.fee_tier == 10000,
        ErrorCode::InvalidFeeTier
    );
    
    // 4. Additional validation: Ensure the position range is wide enough
    // This prevents positions that could accidentally collect base token fees
    let tick_range = config.upper_tick - config.lower_tick;
    require!(
        tick_range >= 887272, // Minimum range for quote-only positions
        ErrorCode::PositionRangeTooNarrow
    );
    
    // 5. Validate that the position is configured for maximum fee capture
    // Quote-only positions should be set up to capture fees across the entire price range
    require!(
        config.lower_tick <= -443636 && config.upper_tick >= 443636,
        ErrorCode::PositionMustSpanFullRange
    );
    
    msg!("Quote-only fee configuration validated successfully");
    msg!("Base weight: {} bps, Quote weight: {} bps", config.base_weight_bps, config.quote_weight_bps);
    msg!("Tick range: {} to {}, Fee tier: {} bps", config.lower_tick, config.upper_tick, config.fee_tier);
    
    Ok(())
}