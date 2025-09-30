use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct DistributionConfig {
    /// Total investor allocation at TGE (Y0)
    pub y0_allocation: u64,
    /// Investor fee share in basis points (max share)
    pub investor_fee_share_bps: u16,
    /// Minimum payout amount in lamports (dust threshold)
    pub min_payout_lamports: u64,
    /// Daily distribution cap in lamports (0 = no cap)
    pub daily_cap_lamports: u64,
    /// Creator wallet address for remainder routing
    pub creator_wallet: Pubkey,
    /// Quote mint address (for validation)
    pub quote_mint: Pubkey,
    /// Bump seed for the PDA
    pub bump: u8,
}