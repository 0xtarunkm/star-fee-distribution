# Star Fee Distribution - DAMM v2 Honorary Quote-Only Fee Position

A Solana program for managing honorary DAMM v2 LP positions and distributing quote-only fees to investors based on their locked token balances.

## Overview

This program implements a permissionless 24-hour distribution crank that:
- Creates and manages an honorary DAMM v2 LP position owned by a program PDA
- Accrues fees **exclusively in the quote mint** (enforced)
- Distributes fees to investors pro-rata based on their locked amounts (tracked via DepositorRecord)
- Routes remaining fees to the creator wallet after investor distribution

## Architecture

### Core Components

1. **Honorary Position Management**
   - Creates empty DAMM v2 position owned by program PDA
   - Validates quote-only fee accrual through tick range and weight configuration
   - Rejects any configuration that could accrue base token fees

2. **DepositorRecord System** (Streamflow Alternative)
   - Tracks individual investor deposits and balances
   - Stores locked amounts (current_usdc_balance) used for distribution weights
   - Maintains withdrawal history and share percentages

3. **24h Distribution Crank**
   - Permissionless execution once per 24 hours
   - Supports pagination for large investor sets
   - Implements idempotent resumption
   - Enforces daily caps and dust thresholds

4. **Distribution Math**
   ```
   Y0 = total investor allocation at TGE
   locked_total(t) = sum of current_usdc_balance across all investors
   f_locked(t) = locked_total(t) / Y0
   eligible_investor_share_bps = min(investor_fee_share_bps, floor(f_locked(t) * 10000))
   investor_fee_quote = floor(claimed_quote * eligible_investor_share_bps / 10000)
   
   For each investor:
   weight_i(t) = investor.current_usdc_balance / locked_total(t)
   payout_i = floor(investor_fee_quote * weight_i(t))
   ```

## Instructions

### 1. initialize_distribution_config
Initialize the distribution policy configuration.

**Parameters:**
- `y0_allocation`: Total investor allocation at TGE (used for f_locked calculation)
- `investor_fee_share_bps`: Maximum investor share (e.g., 5000 = 50%)
- `min_payout_lamports`: Minimum payout threshold (dust handling)
- `daily_cap_lamports`: Daily distribution limit (0 = no cap)
- `creator_wallet`: Creator's wallet for remainder routing
- `quote_mint`: Quote token mint (for validation)

**Accounts:**
- `admin`: Signer who initializes the config
- `distribution_config`: PDA [b"distribution_config"]

### 2. initialize_honorary_position
Create an honorary DAMM v2 LP position that accrues quote-only fees.

**Config Validation:**
- `base_weight_bps`: Must be 0
- `quote_weight_bps`: Must be 10000 (100%)
- `lower_tick`: Must be <= -443636
- `upper_tick`: Must be >= 443636
- `fee_tier`: Must be 100, 500, 3000, or 10000 bps

**Accounts:**
- `signer`: Position owner (program PDA)
- `amm_program`: DAMM v2 program
- `pool`, `position`, `position_nft_mint`, `position_nft_account`: Position accounts
- `base_mint`, `quote_mint`: Token mints
- Token vaults and accounts

### 3. deposit
Investors deposit SOL/USDC to establish their locked balances.

**Parameters:**
- `sol_amount`: Amount of SOL to deposit (lamports)
- `usdc_amount`: Amount of USDC to deposit (smallest unit)

**Accounts:**
- `investor`: Signer making the deposit
- `sol_vault`: Program SOL vault PDA [b"deposit_vault", b"sol"]
- `usdc_vault`: Program USDC vault PDA [b"deposit_vault", usdc_mint]
- `depositor_record`: PDA [b"investor_record", investor]
- `vault_stats`: PDA [b"deposit_vault", b"stats"]

### 4. withdraw
Investors withdraw their deposited amounts.

**Parameters:**
- `sol_amount`: Amount of SOL to withdraw
- `usdc_amount`: Amount of USDC to withdraw

**Accounts:** Same as deposit, plus investor token accounts

### 5. claim_fees_to_pda
Claim fees from the honorary position to program vaults.

**Quote-Only Enforcement:**
- Records balance before/after claim
- **Fails if ANY base fees are detected**
- Only proceeds if base_claimed == 0

**Accounts:**
- `fee_collector`: Program authority PDA [b"fee_collector"]
- `amm_program`: DAMM v2 program
- `pool`, `position`: Position accounts
- `program_token_a_vault`: Base token vault (must remain at 0)
- `program_token_b_vault`: Quote token vault (receives fees)

### 6. crank_fee_distribution
Initiate or continue daily fee distribution (permissionless).

**Flow:**
1. Start new day if 24h elapsed since last distribution
2. Validate no base fees (fail if base_vault.amount > 0)
3. Calculate eligible investor share using f_locked formula
4. Advance pagination cursor
5. Track daily distributed and carry-over

**Parameters:**
- `page_index`: Current page (must match cursor for idempotency)
- `investors_count`: Number of investors in this page
- `is_final_page`: Whether this is the last page

**Accounts:**
- `payer`: Transaction payer
- `fee_collector`: Program authority PDA
- `program_token_a_vault`: Base vault (must be 0)
- `program_token_b_vault`: Quote vault (source of fees)
- `vault_stats`: Global vault statistics
- `distribution_config`: Distribution policy
- `crank_state`: Pagination and timing state PDA [b"crank_state"]

### 7. distribute_to_investor
Distribute quote fees to a specific investor (called per investor during crank).

**Math:**
- Calculates weight based on investor's current_usdc_balance
- Applies dust threshold (min_payout_lamports)
- Updates carry-over for dust amounts
- Checks daily cap before transfer

**Parameters:**
- `total_investor_fee`: Total investor allocation for this distribution

**Accounts:**
- `fee_collector`: Program authority
- `program_quote_vault`: Quote fee vault
- `investor_quote_account`: Investor's quote token account
- `depositor_record`: Investor's record
- `vault_stats`: Global statistics
- `distribution_config`: Policy config
- `crank_state`: Distribution state
- `investor`: Investor signer

### 8. route_creator_remainder
Close the distribution day and route remaining fees to creator.

**Flow:**
1. Validate day is in progress
2. Transfer all remaining quote tokens to creator
3. Close the day (day_state = 2)
4. Reset for next 24h period

**Accounts:**
- `fee_collector`: Program authority
- `program_quote_vault`: Quote fee vault
- `creator_quote_account`: Creator's quote token account (must match config)
- `distribution_config`: Policy config
- `crank_state`: Distribution state

## PDAs and Seeds

| Account | Seeds |
|---------|-------|
| fee_collector | `[b"fee_collector"]` |
| fee_vault (base) | `[b"fee_vault", base_mint]` |
| fee_vault (quote) | `[b"fee_vault", quote_mint]` |
| deposit_vault (SOL) | `[b"deposit_vault", b"sol"]` |
| deposit_vault (USDC) | `[b"deposit_vault", usdc_mint]` |
| vault_stats | `[b"deposit_vault", b"stats"]` |
| investor_record | `[b"investor_record", investor_pubkey]` |
| crank_state | `[b"crank_state"]` |
| distribution_config | `[b"distribution_config"]` |

## State Accounts

### DistributionConfig
```rust
pub struct DistributionConfig {
    pub y0_allocation: u64,              // TGE allocation for f_locked calc
    pub investor_fee_share_bps: u16,     // Max investor share (0-10000)
    pub min_payout_lamports: u64,        // Dust threshold
    pub daily_cap_lamports: u64,         // Daily limit (0 = unlimited)
    pub creator_wallet: Pubkey,          // Remainder destination
    pub quote_mint: Pubkey,              // Quote token mint
    pub bump: u8,
}
```

### CrankState
```rust
pub struct CrankState {
    pub last_distribution_timestamp: i64,
    pub current_day: u32,
    pub distribution_count: u32,
    pub pagination_cursor: u32,          // For idempotent resumption
    pub investors_processed_today: u32,
    pub daily_distributed: u64,
    pub carry_over: u64,                 // Accumulated dust
    pub day_state: u8,                   // 0=not started, 1=in progress, 2=closed
    pub bump: u8,
}
```

### DepositorRecord
```rust
pub struct DepositorRecord {
    pub investor: Pubkey,
    pub total_sol_deposited: u64,
    pub total_usdc_deposited: u64,
    pub current_sol_balance: u64,       // Used for distribution weight
    pub current_usdc_balance: u64,      // Used for distribution weight
    pub total_sol_withdrawn: u64,
    pub total_usdc_withdrawn: u64,
    pub first_deposit_timestamp: i64,
    pub last_activity_timestamp: i64,
    pub deposit_count: u32,
    pub withdrawal_count: u32,
    pub bump: u8,
}
```

### VaultStats
```rust
pub struct VaultStats {
    pub total_sol_deposited: u64,
    pub total_usdc_deposited: u64,
    pub current_total_sol: u64,         // Sum of all current_sol_balance
    pub current_total_usdc: u64,        // Used for locked_total(t)
    pub total_sol_withdrawn: u64,
    pub total_usdc_withdrawn: u64,
    pub depositor_count: u32,
    pub last_update_timestamp: i64,
    pub bump: u8,
}
```

## Error Codes

| Code | Message |
|------|---------|
| BaseFeesDetected | Base fees detected - quote-only position violated |
| DistributionTooFrequent | Distribution too frequent - must wait 24 hours |
| DailyCapExceeded | Daily distribution cap exceeded |
| PayoutBelowMinimum | Payout below minimum threshold |
| InvalidPaginationCursor | Invalid pagination cursor |
| DayAlreadyClosed | Day already closed - cannot distribute |
| DistributionNotStarted | Distribution not started for this day |
| InvalidY0Allocation | Invalid Y0 allocation amount |

## Acceptance Criteria Compliance

### ✅ Honorary Position
- [x] Owned by program PDA (fee_collector)
- [x] Quote-only validation via config params
- [x] Deterministic preflight checks
- [x] Rejects base fee configurations
- [x] Wide tick range (-443636 to +443636)

### ✅ Quote-Only Enforcement
- [x] Balance tracking before/after claim
- [x] Transaction fails if base_claimed > 0
- [x] Quote mint validation in config
- [x] Crank fails if base vault has any balance

### ✅ 24h Distribution Crank
- [x] 86400 second cooldown enforced
- [x] Pagination support with cursor tracking
- [x] Idempotent resumption (page_index must match cursor)
- [x] Day state machine (0=not started, 1=in progress, 2=closed)

### ✅ Distribution Math
- [x] f_locked(t) = locked_total(t) / Y0
- [x] eligible_investor_share = min(investor_fee_share_bps, f_locked_bps)
- [x] Pro-rata weights per investor
- [x] Floor division for all calculations

### ✅ Caps and Dust
- [x] Daily cap enforcement
- [x] Min payout threshold
- [x] Carry-over tracking
- [x] Dust accumulation across pages

### ✅ Creator Remainder
- [x] Separate instruction after final page
- [x] Transfers remaining balance to creator
- [x] Validates creator wallet from config
- [x] Closes day state

### ✅ DepositorRecord Integration
- [x] Replaces Streamflow with custom tracking
- [x] current_usdc_balance = locked amount
- [x] Supports deposits and withdrawals
- [x] Share percentage calculations

## Integration Guide

### Step 1: Initialize Config
```typescript
await program.methods
  .initializeDistributionConfig({
    y0Allocation: new anchor.BN(1_000_000_000_000), // 1M USDC (6 decimals)
    investorFeeShareBps: 5000, // 50%
    minPayoutLamports: new anchor.BN(10_000),
    dailyCapLamports: new anchor.BN(0), // No cap
    creatorWallet: creatorPublicKey,
    quoteMint: usdcMint,
  })
  .accounts({
    admin: adminKeypair.publicKey,
    distributionConfig: distributionConfigPDA,
    systemProgram: SystemProgram.programId,
  })
  .signers([adminKeypair])
  .rpc();
```

### Step 2: Create Honorary Position
```typescript
await program.methods
  .initializeHonoraryPosition({
    baseWeightBps: 0,
    quoteWeightBps: 10000,
    lowerTick: -443636,
    upperTick: 443636,
    feeTier: 100,
  })
  .accounts({
    signer: pdaOwner,
    ammProgram: DAMM_V2_PROGRAM_ID,
    pool: poolPublicKey,
    // ... other accounts
  })
  .rpc();
```

### Step 3: Investors Deposit
```typescript
await program.methods
  .deposit({
    solAmount: new anchor.BN(0),
    usdcAmount: new anchor.BN(100_000_000), // 100 USDC
  })
  .accounts({
    investor: investorKeypair.publicKey,
    feeCollector: feeCollectorPDA,
    solVault: solVaultPDA,
    usdcVault: usdcVaultPDA,
    usdcMint: usdcMint,
    investorUsdcAccount: investorUsdcAccount,
    depositorRecord: depositorRecordPDA,
    vaultStats: vaultStatsPDA,
    // ...
  })
  .signers([investorKeypair])
  .rpc();
```

### Step 4: Claim Fees (Permissionless)
```typescript
await program.methods
  .claimFeesToPda()
  .accounts({
    feeCollector: feeCollectorPDA,
    ammProgram: DAMM_V2_PROGRAM_ID,
    pool: poolPublicKey,
    position: positionPublicKey,
    programTokenAVault: baseVaultPDA, // Must stay at 0
    programTokenBVault: quoteVaultPDA, // Receives fees
    // ...
  })
  .rpc();
```

### Step 5: Run Distribution Crank (Permissionless)
```typescript
// Start crank (first page)
await program.methods
  .crankFeeDistribution({
    pageIndex: 0,
    investorsCount: 10,
    isFinalPage: false,
  })
  .accounts({
    payer: payerKeypair.publicKey,
    feeCollector: feeCollectorPDA,
    programTokenAVault: baseVaultPDA,
    programTokenBVault: quoteVaultPDA,
    vaultStats: vaultStatsPDA,
    distributionConfig: distributionConfigPDA,
    crankState: crankStatePDA,
    // ...
  })
  .signers([payerKeypair])
  .rpc();

// Distribute to each investor in page
for (const investor of investorsInPage) {
  await program.methods
    .distributeToInvestor({
      totalInvestorFee: calculatedInvestorFee,
    })
    .accounts({
      feeCollector: feeCollectorPDA,
      programQuoteVault: quoteVaultPDA,
      investorQuoteAccount: investor.quoteAccount,
      depositorRecord: investor.recordPDA,
      investor: investor.publicKey,
      // ...
    })
    .signers([investor.keypair])
    .rpc();
}
```

### Step 6: Close Day and Route Remainder
```typescript
await program.methods
  .routeCreatorRemainder()
  .accounts({
    feeCollector: feeCollectorPDA,
    programQuoteVault: quoteVaultPDA,
    creatorQuoteAccount: creatorQuoteAccount,
    distributionConfig: distributionConfigPDA,
    crankState: crankStatePDA,
    // ...
  })
  .rpc();
```

## Testing

The program includes comprehensive tests covering:
- Honorary position creation and validation
- Quote-only fee enforcement
- Deposit/withdrawal flows
- Distribution math with various locked amounts
- Pagination and cursor tracking
- Daily cap and dust handling
- Creator remainder routing

Run tests:
```bash
anchor test
```

## Security Considerations

1. **Quote-Only Enforcement**: The program fails deterministically if ANY base fees are detected
2. **24h Gating**: Enforced via timestamp comparison with 86400 second cooldown
3. **Pagination Idempotency**: Cursor validation prevents double-payment
4. **Daily Caps**: Checked before each transfer to prevent over-distribution
5. **PDA Ownership**: All sensitive operations require PDA signer
6. **Dust Handling**: Small amounts carried over instead of lost
7. **Creator Validation**: Ensures creator wallet matches config

## License

MIT
