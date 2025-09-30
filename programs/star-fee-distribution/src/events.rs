use anchor_lang::prelude::*;

/// Event emitted when the honorary DAMM v2 position is initialized
#[event]
pub struct HonoraryPositionInitialized {
    /// The pool address
    pub pool: Pubkey,
    /// The position address
    pub position: Pubkey,
    /// The position NFT mint
    pub position_nft_mint: Pubkey,
    /// Base token mint
    pub base_mint: Pubkey,
    /// Quote token mint
    pub quote_mint: Pubkey,
    /// Base weight in basis points (should be 0 for quote-only)
    pub base_weight_bps: u16,
    /// Quote weight in basis points (should be 10000 for quote-only)
    pub quote_weight_bps: u16,
    /// Lower tick of the position range
    pub lower_tick: i32,
    /// Upper tick of the position range
    pub upper_tick: i32,
    /// Fee tier
    pub fee_tier: u16,
    /// Timestamp of initialization
    pub timestamp: i64,
}

/// Event emitted when quote fees are claimed from the DAMM v2 position
#[event]
pub struct QuoteFeesClaimed {
    /// The pool address
    pub pool: Pubkey,
    /// The position address
    pub position: Pubkey,
    /// Amount of base fees claimed (should be 0 for quote-only)
    pub base_fees_claimed: u64,
    /// Amount of quote fees claimed
    pub quote_fees_claimed: u64,
    /// Program's base token vault
    pub program_base_vault: Pubkey,
    /// Program's quote token vault
    pub program_quote_vault: Pubkey,
    /// Timestamp of claim
    pub timestamp: i64,
}

/// Event emitted for each page of investor payouts during distribution
#[event]
pub struct InvestorPayoutPage {
    /// Current distribution day number
    pub day: u32,
    /// Page index
    pub page_index: u32,
    /// Number of investors processed in this page
    pub investors_count: u32,
    /// Total investors processed so far today
    pub total_investors_processed_today: u32,
    /// Amount of quote fees available for distribution
    pub quote_fees_available: u64,
    /// Total locked amount across all investors
    pub total_locked: u64,
    /// Y0 allocation (total investor allocation at TGE)
    pub y0_allocation: u64,
    /// f_locked ratio in basis points
    pub f_locked_bps: u16,
    /// Eligible investor share in basis points
    pub eligible_investor_share_bps: u16,
    /// Total investor fee allocation for this distribution
    pub investor_fee_quote: u64,
    /// Amount distributed in this page
    pub page_distributed: u64,
    /// Dust carried over
    pub carry_over: u64,
    /// Total distributed so far today
    pub daily_distributed: u64,
    /// Daily cap (0 = no cap)
    pub daily_cap: u64,
    /// Is this the final page of the day?
    pub is_final_page: bool,
    /// Timestamp of distribution
    pub timestamp: i64,
}

/// Event emitted when an individual investor receives their payout
#[event]
pub struct InvestorPayout {
    /// Current distribution day number
    pub day: u32,
    /// Investor's wallet address
    pub investor: Pubkey,
    /// Investor's locked balance (from DepositorRecord)
    pub investor_locked_balance: u64,
    /// Total locked across all investors
    pub total_locked: u64,
    /// Investor's weight in basis points
    pub weight_bps: u64,
    /// Total investor fee pool for this distribution
    pub total_investor_fee: u64,
    /// Calculated payout before dust threshold
    pub calculated_payout: u64,
    /// Actual payout after dust threshold
    pub actual_payout: u64,
    /// Dust amount (payout below minimum)
    pub dust: u64,
    /// Minimum payout threshold
    pub min_payout: u64,
    /// Investor's quote token account
    pub investor_quote_account: Pubkey,
    /// Timestamp of payout
    pub timestamp: i64,
}

/// Event emitted when the distribution day is closed and creator receives remainder
#[event]
pub struct CreatorPayoutDayClosed {
    /// Distribution day number that was closed
    pub day: u32,
    /// Creator's wallet address
    pub creator_wallet: Pubkey,
    /// Creator's quote token account
    pub creator_quote_account: Pubkey,
    /// Amount of quote fees sent to creator (remainder)
    pub creator_remainder: u64,
    /// Total amount distributed to investors this day
    pub total_distributed_to_investors: u64,
    /// Total investors processed this day
    pub total_investors_processed: u32,
    /// Carry-over dust from this day
    pub final_carry_over: u64,
    /// Timestamp when day was closed
    pub timestamp: i64,
}

/// Event emitted when distribution config is initialized
#[event]
pub struct DistributionConfigInitialized {
    /// Distribution config PDA
    pub config: Pubkey,
    /// Y0 allocation (total investor allocation at TGE)
    pub y0_allocation: u64,
    /// Investor fee share in basis points
    pub investor_fee_share_bps: u16,
    /// Minimum payout threshold
    pub min_payout_lamports: u64,
    /// Daily distribution cap (0 = no cap)
    pub daily_cap_lamports: u64,
    /// Creator wallet address
    pub creator_wallet: Pubkey,
    /// Quote mint address
    pub quote_mint: Pubkey,
    /// Timestamp of initialization
    pub timestamp: i64,
}

/// Event emitted when a deposit is made
#[event]
pub struct DepositMade {
    /// Investor's wallet address
    pub investor: Pubkey,
    /// Amount of SOL deposited
    pub sol_amount: u64,
    /// Amount of USDC deposited
    pub usdc_amount: u64,
    /// Investor's new total SOL deposited
    pub total_sol_deposited: u64,
    /// Investor's new total USDC deposited
    pub total_usdc_deposited: u64,
    /// Investor's current SOL balance
    pub current_sol_balance: u64,
    /// Investor's current USDC balance
    pub current_usdc_balance: u64,
    /// Investor's deposit count
    pub deposit_count: u32,
    /// Timestamp of deposit
    pub timestamp: i64,
}

/// Event emitted when a withdrawal is made
#[event]
pub struct WithdrawalMade {
    /// Investor's wallet address
    pub investor: Pubkey,
    /// Amount of SOL withdrawn
    pub sol_amount: u64,
    /// Amount of USDC withdrawn
    pub usdc_amount: u64,
    /// Investor's new total SOL withdrawn
    pub total_sol_withdrawn: u64,
    /// Investor's new total USDC withdrawn
    pub total_usdc_withdrawn: u64,
    /// Investor's current SOL balance
    pub current_sol_balance: u64,
    /// Investor's current USDC balance
    pub current_usdc_balance: u64,
    /// Investor's withdrawal count
    pub withdrawal_count: u32,
    /// Timestamp of withdrawal
    pub timestamp: i64,
}
