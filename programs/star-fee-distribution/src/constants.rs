// PDA Seeds
pub const FEE_COLLECTOR_SEED: &[u8] = b"fee_collector";
pub const FEE_VAULT_SEED: &[u8] = b"fee_vault";
pub const DEPOSIT_VAULT_SEED: &[u8] = b"deposit_vault";
pub const INVESTOR_RECORD_SEED: &[u8] = b"investor_record";
pub const CRANK_STATE_SEED: &[u8] = b"crank_state";
pub const DISTRIBUTION_CONFIG_SEED: &[u8] = b"distribution_config";

// Default policy parameters
pub const DEFAULT_INVESTOR_FEE_SHARE_BPS: u16 = 5000; // 50%
pub const DEFAULT_MIN_PAYOUT_LAMPORTS: u64 = 10_000; // 0.00001 SOL minimum
pub const DEFAULT_DAILY_CAP_LAMPORTS: u64 = 0; // 0 = no cap

// Validation constants
pub const MIN_SOL_DEPOSIT: u64 = 1_000_000; // 0.001 SOL minimum
pub const MAX_SOL_DEPOSIT: u64 = 1_000_000_000_000; // 1000 SOL maximum
pub const MIN_USDC_DEPOSIT: u64 = 1_000; // 0.001 USDC minimum (6 decimals)
pub const MAX_USDC_DEPOSIT: u64 = 1_000_000_000_000; // 1M USDC maximum

// Fee distribution constants
pub const MAX_INVESTOR_FEE_SHARE_BPS: u16 = 10000; // 100% maximum
pub const MIN_INVESTOR_FEE_SHARE_BPS: u16 = 0; // 0% minimum
pub const DISTRIBUTION_BATCH_SIZE: u32 = 10; // Process 10 investors per batch
pub const SECONDS_PER_DAY: i64 = 86400; // 24 hours in seconds

// Error codes
pub const ERROR_INVALID_DEPOSIT_AMOUNT: u32 = 0x0;
pub const ERROR_INSUFFICIENT_BALANCE: u32 = 0x1;
pub const ERROR_MATH_OVERFLOW: u32 = 0x2;
pub const ERROR_DISTRIBUTION_TOO_FREQUENT: u32 = 0x3;
