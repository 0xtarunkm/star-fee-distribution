import { BN, Program } from "@coral-xyz/anchor";
import { ProgramTestContext } from "solana-bankrun";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { expect, assert } from "chai";
import {
  fundSol,
  fundUsdc,
  getOrCreateAta,
  getTokenAccount,
  getBalance,
  LOCAL_ADMIN_KEYPAIR,
  startTest,
  USDC_MINT,
  fetchAccount,
} from "./utils/bankrun";
import { StarFeeDistribution } from "../target/types/star_fee_distribution";
import IDL from "../target/idl/star_fee_distribution.json";

describe("Fee Distribution Crank Tests (Bankrun)", () => {
  let context: ProgramTestContext;
  let program: Program<StarFeeDistribution>;
  let admin: Keypair;
  let investor1: Keypair;
  let investor2: Keypair;
  let investor3: Keypair;
  let creatorWallet: Keypair;

  // PDAs
  let feeCollectorPDA: PublicKey;
  let solVaultPDA: PublicKey;
  let usdcVaultPDA: PublicKey;
  let vaultStatsPDA: PublicKey;
  let distributionConfigPDA: PublicKey;
  let distributionStatePDA: PublicKey;

  before(async () => {
    context = await startTest();
    admin = LOCAL_ADMIN_KEYPAIR;
    investor1 = Keypair.generate();
    investor2 = Keypair.generate();
    investor3 = Keypair.generate();
    creatorWallet = Keypair.generate();

    program = new Program<StarFeeDistribution>(
      IDL as StarFeeDistribution,
      {
        connection: context.banksClient as any,
      } as any
    );

    // Fund accounts
    await fundSol(context.banksClient, admin, [
      investor1.publicKey,
      investor2.publicKey,
      investor3.publicKey,
      creatorWallet.publicKey,
    ]);

    await fundUsdc(context.banksClient, [
      investor1.publicKey,
      investor2.publicKey,
      investor3.publicKey,
    ]);

    // Calculate PDAs
    [feeCollectorPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("fee_collector")],
      program.programId
    );

    [solVaultPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("deposit_vault"), Buffer.from("sol")],
      program.programId
    );

    [usdcVaultPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("deposit_vault"), USDC_MINT.toBuffer()],
      program.programId
    );

    [vaultStatsPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("deposit_vault"), Buffer.from("stats")],
      program.programId
    );

    [distributionConfigPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("distribution_config")],
      program.programId
    );

    [distributionStatePDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("crank_state")],
      program.programId
    );
  });

  describe("Crank Fee Distribution - Edge Cases", () => {
    it("Should handle zero fees gracefully", async () => {
      // Initialize distribution config first
      const configTx = await program.methods
        .initializeDistributionConfig({
          y0Allocation: new BN(1_000_000_000_000),
          investorFeeShareBps: 5000,
          minPayoutLamports: new BN(100_000),
          dailyCapLamports: new BN(100_000_000_000),
          creatorWallet: creatorWallet.publicKey,
          quoteMint: USDC_MINT,
        })
        .accountsStrict({
          admin: admin.publicKey,
          distributionConfig: distributionConfigPDA,
          systemProgram: SystemProgram.programId,
        })
        .transaction();

      const [configBlockhash] = await context.banksClient.getLatestBlockhash();
      configTx.recentBlockhash = configBlockhash;
      configTx.sign(admin);
      await context.banksClient.processTransaction(configTx);

      // Make some deposits
      const depositTx = await program.methods
        .deposit({
          solAmount: new BN(1 * LAMPORTS_PER_SOL),
          usdcAmount: new BN(0),
        })
        .accountsStrict({
          investor: investor1.publicKey,
          feeCollector: feeCollectorPDA,
          solVault: solVaultPDA,
          usdcVault: usdcVaultPDA,
          usdcMint: USDC_MINT,
          investorUsdcAccount: await getOrCreateAta(
            context.banksClient,
            admin,
            USDC_MINT,
            investor1.publicKey
          ),
          depositorRecord: PublicKey.findProgramAddressSync(
            [Buffer.from("investor_record"), investor1.publicKey.toBuffer()],
            program.programId
          )[0],
          vaultStats: vaultStatsPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .transaction();

      const [depositBlockhash] = await context.banksClient.getLatestBlockhash();
      depositTx.recentBlockhash = depositBlockhash;
      depositTx.sign(investor1);
      await context.banksClient.processTransaction(depositTx);

      // Try to crank with zero fees - should not fail
      const vaultStats = await fetchAccount(
        context.banksClient,
        program,
        "VaultStats",
        vaultStatsPDA
      );

      expect(vaultStats).to.not.be.null;
      expect(vaultStats!.totalSolDeposited.toString()).to.equal(
        (1 * LAMPORTS_PER_SOL).toString()
      );
    });
  });
});
