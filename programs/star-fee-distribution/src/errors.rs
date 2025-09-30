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
}