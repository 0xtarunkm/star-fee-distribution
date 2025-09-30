use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Base weight must be zero for quote-only fee collection")]
    BaseWeightMustBeZero,
    #[msg("Quote weight must be 10000 bps for quote-only fee collection")]
    QuoteWeightMustBe10000,
    #[msg("Lower tick must be <= -443636 for wide range")]
    LowerTickTooHigh,
    #[msg("Upper tick must be >= 443636 for wide range")]
    UpperTickTooLow,
    #[msg("Invalid fee tier - must be 100, 500, 3000, or 10000 bps")]
    InvalidFeeTier,
    #[msg("Position range must be wide enough for quote-only collection")]
    PositionRangeTooNarrow,
    #[msg("Position must span full range for maximum quote fee capture")]
    PositionMustSpanFullRange,
    #[msg("No fees available to claim from position")]
    NoFeesToClaim,
    #[msg("Position not found or invalid")]
    InvalidPosition,
    #[msg("Insufficient token account balance for fee transfer")]
    InsufficientTokenBalance,
    #[msg("Invalid deposit amount - must be within valid range")]
    InvalidDepositAmount,
    #[msg("Deposit amount too small - below minimum threshold")]
    DepositAmountTooSmall,
    #[msg("Deposit amount too large - exceeds maximum limit")]
    DepositAmountTooLarge,
    #[msg("Math overflow occurred during calculation")]
    MathOverflow,
    #[msg("Depositor record not found")]
    DepositorRecordNotFound,
    #[msg("Vault stats not found")]
    VaultStatsNotFound,
    #[msg("Distribution too frequent - must wait 24 hours")]
    DistributionTooFrequent,
    #[msg("Crank state not found")]
    CrankStateNotFound,
    #[msg("No investors to distribute to")]
    NoInvestorsToDistribute,
    #[msg("Base fees detected - quote-only position violated")]
    BaseFeesDetected,
    #[msg("Daily distribution cap exceeded")]
    DailyCapExceeded,
    #[msg("Payout below minimum threshold")]
    PayoutBelowMinimum,
    #[msg("Invalid pagination cursor")]
    InvalidPaginationCursor,
    #[msg("Day already closed - cannot distribute")]
    DayAlreadyClosed,
    #[msg("Distribution not started for this day")]
    DistributionNotStarted,
    #[msg("Distribution config not found")]
    DistributionConfigNotFound,
    #[msg("Invalid Y0 allocation amount")]
    InvalidY0Allocation,
    #[msg("Creator wallet not provided")]
    CreatorWalletNotProvided,
    #[msg("Insufficient balance for operation")]
    InsufficientBalance,
}