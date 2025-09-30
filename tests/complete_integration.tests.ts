import { BN, Program } from "@coral-xyz/anchor";
import { ProgramTestContext } from "solana-bankrun";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, NATIVE_MINT } from "@solana/spl-token";
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
  warpSlotBy,
  fetchAccount,
} from "./utils/bankrun";
import { StarFeeDistribution } from "../target/types/star_fee_distribution";
import IDL from "../target/idl/star_fee_distribution.json";

describe("Complete Integration Tests (Bankrun)", () => {
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
  });

  describe("Complete Flow: Setup to Distribution", () => {
    it("Step 1: Initialize distribution config", async () => {
      const tx = await program.methods
        .initializeDistributionConfig({
          y0Allocation: new BN(1_000_000_000_000), // 1T units
          investorFeeShareBps: 6000, // 60%
          minPayoutLamports: new BN(100_000), // 0.0001 SOL
          dailyCapLamports: new BN(1_000_000_000_000), // 1000 SOL
          creatorWallet: creatorWallet.publicKey,
          quoteMint: USDC_MINT,
        })
        .accountsStrict({
          admin: admin.publicKey,
          distributionConfig: distributionConfigPDA,
          systemProgram: SystemProgram.programId,
        })
        .transaction();

      const [recentBlockhash] = await context.banksClient.getLatestBlockhash();
      tx.recentBlockhash = recentBlockhash;
      tx.sign(admin);

      await context.banksClient.processTransaction(tx);

      const config = await fetchAccount(
        context.banksClient,
        program,
        "DistributionConfig",
        distributionConfigPDA
      );

      expect(config).to.not.be.null;
      expect(config!.y0Allocation.toString()).to.equal("1000000000000");
      expect(config!.investorFeeShareBps).to.equal(6000);
      expect(config!.creatorWallet.toString()).to.equal(
        creatorWallet.publicKey.toString()
      );
    });

    it("Step 2: Multiple investors deposit funds", async () => {
      // Investor 1: SOL only
      const deposit1Tx = await program.methods
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

      const [blockhash1] = await context.banksClient.getLatestBlockhash();
      deposit1Tx.recentBlockhash = blockhash1;
      deposit1Tx.sign(investor1);
      await context.banksClient.processTransaction(deposit1Tx);

      // Investor 2: SOL only
      const deposit2Tx = await program.methods
        .deposit({
          solAmount: new BN(2 * LAMPORTS_PER_SOL),
          usdcAmount: new BN(0),
        })
        .accountsStrict({
          investor: investor2.publicKey,
          feeCollector: feeCollectorPDA,
          solVault: solVaultPDA,
          usdcVault: usdcVaultPDA,
          usdcMint: USDC_MINT,
          investorUsdcAccount: await getOrCreateAta(
            context.banksClient,
            admin,
            USDC_MINT,
            investor2.publicKey
          ),
          depositorRecord: PublicKey.findProgramAddressSync(
            [Buffer.from("investor_record"), investor2.publicKey.toBuffer()],
            program.programId
          )[0],
          vaultStats: vaultStatsPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .transaction();

      const [blockhash2] = await context.banksClient.getLatestBlockhash();
      deposit2Tx.recentBlockhash = blockhash2;
      deposit2Tx.sign(investor2);
      await context.banksClient.processTransaction(deposit2Tx);

      // Investor 3: SOL only
      const deposit3Tx = await program.methods
        .deposit({
          solAmount: new BN(3 * LAMPORTS_PER_SOL),
          usdcAmount: new BN(0),
        })
        .accountsStrict({
          investor: investor3.publicKey,
          feeCollector: feeCollectorPDA,
          solVault: solVaultPDA,
          usdcVault: usdcVaultPDA,
          usdcMint: USDC_MINT,
          investorUsdcAccount: await getOrCreateAta(
            context.banksClient,
            admin,
            USDC_MINT,
            investor3.publicKey
          ),
          depositorRecord: PublicKey.findProgramAddressSync(
            [Buffer.from("investor_record"), investor3.publicKey.toBuffer()],
            program.programId
          )[0],
          vaultStats: vaultStatsPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .transaction();

      const [blockhash3] = await context.banksClient.getLatestBlockhash();
      deposit3Tx.recentBlockhash = blockhash3;
      deposit3Tx.sign(investor3);
      await context.banksClient.processTransaction(deposit3Tx);

      // Verify vault stats
      const vaultStats = await fetchAccount(
        context.banksClient,
        program,
        "VaultStats",
        vaultStatsPDA
      );

      expect(vaultStats).to.not.be.null;
      expect(vaultStats!.totalSolDeposited.toString()).to.equal(
        (6 * LAMPORTS_PER_SOL).toString()
      );
      expect(vaultStats!.depositorCount).to.equal(3);
    });

    it("Step 3: Simulate fee collection", async () => {
      // Simulate fees being sent to the fee collector
      const feeAmount = new BN(1 * LAMPORTS_PER_SOL);
      const transferTx = new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: admin.publicKey,
          toPubkey: feeCollectorPDA,
          lamports: feeAmount,
        })
      );

      const [recentBlockhash] = await context.banksClient.getLatestBlockhash();
      transferTx.recentBlockhash = recentBlockhash;
      transferTx.sign(admin);

      await context.banksClient.processTransaction(transferTx);

      const feeCollectorBalance = await getBalance(
        context.banksClient,
        feeCollectorPDA
      );
      expect(feeCollectorBalance).to.be.greaterThan(0);
    });
  });

  describe("System Integrity Tests", () => {
    it("Should maintain accurate accounting across all operations", async () => {
      const vaultStats = await fetchAccount(
        context.banksClient,
        program,
        "VaultStats",
        vaultStatsPDA
      );

      expect(vaultStats).to.not.be.null;
      expect(vaultStats!.totalSolDeposited.toString()).to.equal(
        (6 * LAMPORTS_PER_SOL).toString()
      );
      expect(vaultStats!.depositorCount).to.equal(3);
    });

    it("Should track all investor records correctly", async () => {
      const [depositorRecord1] = PublicKey.findProgramAddressSync(
        [Buffer.from("investor_record"), investor1.publicKey.toBuffer()],
        program.programId
      );
      const [depositorRecord2] = PublicKey.findProgramAddressSync(
        [Buffer.from("investor_record"), investor2.publicKey.toBuffer()],
        program.programId
      );
      const [depositorRecord3] = PublicKey.findProgramAddressSync(
        [Buffer.from("investor_record"), investor3.publicKey.toBuffer()],
        program.programId
      );

      const record1 = await fetchAccount(
        context.banksClient,
        program,
        "DepositorRecord",
        depositorRecord1
      );
      const record2 = await fetchAccount(
        context.banksClient,
        program,
        "DepositorRecord",
        depositorRecord2
      );
      const record3 = await fetchAccount(
        context.banksClient,
        program,
        "DepositorRecord",
        depositorRecord3
      );

      expect(record1).to.not.be.null;
      expect(record2).to.not.be.null;
      expect(record3).to.not.be.null;

      expect(record1!.totalSolDeposited.toString()).to.equal(
        (1 * LAMPORTS_PER_SOL).toString()
      );
      expect(record2!.totalSolDeposited.toString()).to.equal(
        (2 * LAMPORTS_PER_SOL).toString()
      );
      expect(record3!.totalSolDeposited.toString()).to.equal(
        (3 * LAMPORTS_PER_SOL).toString()
      );
    });
  });
});
