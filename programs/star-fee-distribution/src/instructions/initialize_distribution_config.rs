use anchor_lang::prelude::*;
use crate::errors::ErrorCode;
use crate::constants::{DISTRIBUTION_CONFIG_SEED, DEFAULT_MIN_PAYOUT_LAMPORTS};
use crate::states::DistributionConfig;

#[derive(Accounts)]
pub struct InitializeDistributionConfig<'info> {
    /// Admin who can initialize the config
    #[account(mut)]
    pub admin: Signer<'info>,
    
    /// Distribution configuration PDA
    #[account(
        init,
        payer = admin,
        space = DistributionConfig::DISCRIMINATOR.len() + DistributionConfig::INIT_SPACE,
        seeds = [DISTRIBUTION_CONFIG_SEED],
        bump
    )]
    pub distribution_config: Account<'info, DistributionConfig>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeDistributionConfigParams {
    /// Total investor allocation at TGE (Y0)
    pub y0_allocation: u64,
    /// Investor fee share in basis points (max share, e.g., 5000 = 50%)
    pub investor_fee_share_bps: u16,
    /// Minimum payout amount in lamports (dust threshold)
    pub min_payout_lamports: u64,
    /// Daily distribution cap in lamports (0 = no cap)
    pub daily_cap_lamports: u64,
    /// Creator wallet address for remainder routing
    pub creator_wallet: Pubkey,
    /// Quote mint address (for validation)
    pub quote_mint: Pubkey,
}

impl<'info> InitializeDistributionConfig<'info> {
    pub fn handle(ctx: Context<InitializeDistributionConfig>, params: InitializeDistributionConfigParams) -> Result<()> {
        msg!("Initializing distribution configuration");
        
        // Validate Y0 allocation
        require!(
            params.y0_allocation > 0,
            ErrorCode::InvalidY0Allocation
        );
        
        // Validate investor fee share (max 10000 bps = 100%)
        require!(
            params.investor_fee_share_bps <= 10000,
            ErrorCode::InvalidDepositAmount
        );
        
        // Validate creator wallet
        require!(
            params.creator_wallet != Pubkey::default(),
            ErrorCode::CreatorWalletNotProvided
        );
        
        // Validate quote mint
        require!(
            params.quote_mint != Pubkey::default(),
            ErrorCode::InvalidPosition
        );
        
        let config_key = ctx.accounts.distribution_config.key();
        let distribution_config = &mut ctx.accounts.distribution_config;
        
        distribution_config.y0_allocation = params.y0_allocation;
        distribution_config.investor_fee_share_bps = params.investor_fee_share_bps;
        distribution_config.min_payout_lamports = if params.min_payout_lamports == 0 {
            DEFAULT_MIN_PAYOUT_LAMPORTS
        } else {
            params.min_payout_lamports
        };
        distribution_config.daily_cap_lamports = params.daily_cap_lamports;
        distribution_config.creator_wallet = params.creator_wallet;
        distribution_config.quote_mint = params.quote_mint;
        distribution_config.bump = ctx.bumps.distribution_config;
        
        msg!("Distribution configuration initialized successfully");
        msg!("Y0 allocation: {} units", params.y0_allocation);
        msg!("Investor fee share: {} bps", params.investor_fee_share_bps);
        msg!("Min payout: {} lamports", distribution_config.min_payout_lamports);
        msg!("Daily cap: {} lamports", params.daily_cap_lamports);
        msg!("Creator wallet: {}", params.creator_wallet);
        msg!("Quote mint: {}", params.quote_mint);
        
        // Emit event
        let y0 = params.y0_allocation;
        let fee_share = params.investor_fee_share_bps;
        let min_payout = distribution_config.min_payout_lamports;
        let daily_cap = params.daily_cap_lamports;
        let creator = params.creator_wallet;
        let quote = params.quote_mint;
        
        emit!(crate::events::DistributionConfigInitialized {
            config: config_key,
            y0_allocation: y0,
            investor_fee_share_bps: fee_share,
            min_payout_lamports: min_payout,
            daily_cap_lamports: daily_cap,
            creator_wallet: creator,
            quote_mint: quote,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }
}
