import { PublicKey, Keypair } from "@solana/web3.js";
import { LAMPORTS_PER_SOL } from "@solana/web3.js";

export const TEST_CONFIG = {
  // Test amounts
  MIN_SOL_DEPOSIT: 0.001 * LAMPORTS_PER_SOL, // 0.001 SOL
  MAX_SOL_DEPOSIT: 1000 * LAMPORTS_PER_SOL, // 1000 SOL
  MIN_USDC_DEPOSIT: 1000, // 0.001 USDC (6 decimals)
  MAX_USDC_DEPOSIT: 1_000_000_000_000, // 1M USDC (6 decimals)
  
  // Test thresholds
  SOL_AIRDROP_AMOUNT: 2 * LAMPORTS_PER_SOL,
  USDC_MINT_AMOUNT: 1000 * 10**6, // 1000 USDC
  
  // Test scenarios
  TEST_SOL_AMOUNTS: [
    0.001 * LAMPORTS_PER_SOL, // Minimum
    0.1 * LAMPORTS_PER_SOL,   // Small
    1 * LAMPORTS_PER_SOL,     // Medium
    10 * LAMPORTS_PER_SOL,    // Large
    100 * LAMPORTS_PER_SOL,   // Very large
  ],
  
  TEST_USDC_AMOUNTS: [
    1000,        // Minimum (0.001 USDC)
    100 * 10**6, // Small (100 USDC)
    1000 * 10**6, // Medium (1000 USDC)
    10000 * 10**6, // Large (10000 USDC)
    100000 * 10**6, // Very large (100000 USDC)
  ],
  
  // Invalid amounts for testing
  INVALID_AMOUNTS: {
    ZERO_SOL: 0,
    ZERO_USDC: 0,
    BELOW_MIN_SOL: 100_000, // 0.0001 SOL
    BELOW_MIN_USDC: 100,    // 0.0001 USDC
    ABOVE_MAX_SOL: 2000 * LAMPORTS_PER_SOL, // 2000 SOL
    ABOVE_MAX_USDC: 2_000_000_000_000, // 2M USDC
  }
};

export const TEST_ACCOUNTS = {
  // These will be populated during test setup
  admin: null as Keypair | null,
  investor1: null as Keypair | null,
  investor2: null as Keypair | null,
  usdcMint: null as PublicKey | null,
  feeCollectorPDA: null as PublicKey | null,
  solVaultPDA: null as PublicKey | null,
  usdcVaultPDA: null as PublicKey | null,
  vaultStatsPDA: null as PublicKey | null,
};

export const TEST_PDAS = {
  getDepositorRecordPDA: (investor: PublicKey, programId: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("investor_record"), investor.toBuffer()],
      programId
    )[0];
  },
  
  getFeeCollectorPDA: (programId: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("fee_collector")],
      programId
    )[0];
  },
  
  getSolVaultPDA: (programId: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("deposit_vault"), Buffer.from("sol")],
      programId
    )[0];
  },
  
  getUsdcVaultPDA: (usdcMint: PublicKey, programId: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("deposit_vault"), usdcMint.toBuffer()],
      programId
    )[0];
  },
  
  getVaultStatsPDA: (programId: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("deposit_vault"), Buffer.from("stats")],
      programId
    )[0];
  }
};

export const TEST_HELPERS = {
  // Helper to create test investor with SOL and USDC
  createTestInvestor: async (connection: any, admin: Keypair, usdcMint: PublicKey) => {
    const investor = Keypair.generate();
    
    // Airdrop SOL
    await connection.requestAirdrop(investor.publicKey, TEST_CONFIG.SOL_AIRDROP_AMOUNT);
    
    // Create USDC token account
    const { createAssociatedTokenAccount, mintTo } = await import("@solana/spl-token");
    const investorUsdcAccount = await createAssociatedTokenAccount(
      connection,
      admin,
      usdcMint,
      investor.publicKey
    );
    
    // Mint USDC
    await mintTo(
      connection,
      admin,
      usdcMint,
      investorUsdcAccount,
      admin,
      TEST_CONFIG.USDC_MINT_AMOUNT
    );
    
    return { investor, usdcAccount: investorUsdcAccount };
  },
  
  // Helper to get all required accounts for deposit
  getDepositAccounts: (programId: PublicKey, investor: PublicKey, usdcMint: PublicKey) => {
    return {
      feeCollector: TEST_PDAS.getFeeCollectorPDA(programId),
      solVault: TEST_PDAS.getSolVaultPDA(programId),
      usdcVault: TEST_PDAS.getUsdcVaultPDA(usdcMint, programId),
      depositorRecord: TEST_PDAS.getDepositorRecordPDA(investor, programId),
      vaultStats: TEST_PDAS.getVaultStatsPDA(programId),
    };
  }
};

export const EXPECTED_ERRORS = {
  INVALID_DEPOSIT_AMOUNT: "InvalidDepositAmount",
  MATH_OVERFLOW: "MathOverflow",
  INSUFFICIENT_TOKEN_BALANCE: "InsufficientTokenBalance",
};

export const TEST_SCENARIOS = {
  // Basic deposit scenarios
  BASIC_SOL_DEPOSIT: {
    solAmount: 0.1 * LAMPORTS_PER_SOL,
    usdcAmount: 0,
    description: "Basic SOL deposit"
  },
  
  BASIC_USDC_DEPOSIT: {
    solAmount: 0,
    usdcAmount: 100 * 10**6,
    description: "Basic USDC deposit"
  },
  
  MIXED_DEPOSIT: {
    solAmount: 0.05 * LAMPORTS_PER_SOL,
    usdcAmount: 50 * 10**6,
    description: "Mixed SOL and USDC deposit"
  },
  
  // Edge cases
  MINIMUM_DEPOSIT: {
    solAmount: TEST_CONFIG.MIN_SOL_DEPOSIT,
    usdcAmount: TEST_CONFIG.MIN_USDC_DEPOSIT,
    description: "Minimum valid deposit amounts"
  },
  
  MAXIMUM_DEPOSIT: {
    solAmount: TEST_CONFIG.MAX_SOL_DEPOSIT,
    usdcAmount: TEST_CONFIG.MAX_USDC_DEPOSIT,
    description: "Maximum valid deposit amounts"
  },
  
  // Invalid scenarios
  ZERO_AMOUNTS: {
    solAmount: 0,
    usdcAmount: 0,
    description: "Zero amounts (should fail)"
  },
  
  BELOW_MINIMUM: {
    solAmount: TEST_CONFIG.INVALID_AMOUNTS.BELOW_MIN_SOL,
    usdcAmount: TEST_CONFIG.INVALID_AMOUNTS.BELOW_MIN_USDC,
    description: "Below minimum amounts (should fail)"
  },
  
  ABOVE_MAXIMUM: {
    solAmount: TEST_CONFIG.INVALID_AMOUNTS.ABOVE_MAX_SOL,
    usdcAmount: TEST_CONFIG.INVALID_AMOUNTS.ABOVE_MAX_USDC,
    description: "Above maximum amounts (should fail)"
  }
};
