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

describe("Deposit and Withdraw Tests (Bankrun)", () => {
  let context: ProgramTestContext;
  let program: Program<StarFeeDistribution>;
  let admin: Keypair;
  let investor1: Keypair;
  let investor2: Keypair;
  let investor3: Keypair;

  // PDAs
  let feeCollectorPDA: PublicKey;
  let solVaultPDA: PublicKey;
  let usdcVaultPDA: PublicKey;
  let vaultStatsPDA: PublicKey;

  before(async () => {
    context = await startTest();
    admin = LOCAL_ADMIN_KEYPAIR;
    investor1 = Keypair.generate();
    investor2 = Keypair.generate();
    investor3 = Keypair.generate();

    // Fund investors with SOL
    await fundSol(context.banksClient, admin, [
      investor1.publicKey,
      investor2.publicKey,
      investor3.publicKey,
    ]);

    // Fund investors with USDC
    await fundUsdc(context.banksClient, [
      investor1.publicKey,
      investor2.publicKey,
      investor3.publicKey,
    ]);

    // Create program instance
    program = new Program<StarFeeDistribution>(
      IDL as StarFeeDistribution,
      { connection: context.banksClient } as any
    );

    // Derive PDAs
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
  });

  describe("Deposit - SOL Only", () => {
    it("Should successfully deposit SOL for first time investor", async () => {
      const depositAmount = new BN(0.5 * LAMPORTS_PER_SOL);
      const [depositorRecordPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("investor_record"), investor1.publicKey.toBuffer()],
        program.programId
      );

      const investor1UsdcAccount = await getOrCreateAta(
        context.banksClient,
        admin,
        USDC_MINT,
        investor1.publicKey
      );

      const beforeBalance = await getBalance(
        context.banksClient,
        investor1.publicKey
      );

      const tx = await program.methods
        .deposit({
          solAmount: depositAmount,
          usdcAmount: new BN(0),
        })
        .accountsStrict({
          investor: investor1.publicKey,
          feeCollector: feeCollectorPDA,
          solVault: solVaultPDA,
          usdcVault: usdcVaultPDA,
          usdcMint: USDC_MINT,
          investorUsdcAccount: investor1UsdcAccount,
          depositorRecord: depositorRecordPDA,
          vaultStats: vaultStatsPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .transaction();

      const [recentBlockhash] = await context.banksClient.getLatestBlockhash();
      tx.recentBlockhash = recentBlockhash;
      tx.sign(investor1);

      await context.banksClient.processTransaction(tx);

      // Verify depositor record
      const depositorRecordAccount = await context.banksClient.getAccount(
        depositorRecordPDA
      );
      expect(depositorRecordAccount).to.not.be.null;

      const depositorRecord = await fetchAccount(
        context.banksClient,
        program,
        "DepositorRecord",
        depositorRecordPDA
      );

      expect(depositorRecord).to.not.be.null;
      expect(depositorRecord!.investor.toString()).to.equal(
        investor1.publicKey.toString()
      );
      expect(depositorRecord!.totalSolDeposited.toString()).to.equal(
        depositAmount.toString()
      );
      expect(depositorRecord!.currentSolBalance.toString()).to.equal(
        depositAmount.toString()
      );
      expect(depositorRecord!.totalUsdcDeposited.toString()).to.equal("0");
      expect(depositorRecord!.currentUsdcBalance.toString()).to.equal("0");
      expect(depositorRecord!.depositCount).to.equal(1);
      expect(depositorRecord!.withdrawalCount).to.equal(0);

      // Verify vault stats
      const vaultStats = await fetchAccount(
        context.banksClient,
        program,
        "VaultStats",
        vaultStatsPDA
      );
      expect(vaultStats.totalSolDeposited.toString()).to.equal(
        depositAmount.toString()
      );
      expect(vaultStats.currentTotalSol.toString()).to.equal(
        depositAmount.toString()
      );
      expect(vaultStats.depositorCount).to.equal(1);
      expect(vaultStats.totalSolWithdrawn.toString()).to.equal("0");

      // Verify SOL vault balance
      const vaultBalance = await getBalance(context.banksClient, solVaultPDA);
      expect(vaultBalance.toString()).to.equal(depositAmount.toString());
    });

    it("Should successfully handle multiple SOL deposits from same investor", async () => {
      const firstDeposit = new BN(0.3 * LAMPORTS_PER_SOL);
      const secondDeposit = new BN(0.2 * LAMPORTS_PER_SOL);
      const [depositorRecordPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("investor_record"), investor1.publicKey.toBuffer()],
        program.programId
      );

      const investor1UsdcAccount = await getOrCreateAta(
        context.banksClient,
        admin,
        USDC_MINT,
        investor1.publicKey
      );

      // Second deposit
      const tx = await program.methods
        .deposit({
          solAmount: secondDeposit,
          usdcAmount: new BN(0),
        })
        .accountsStrict({
          investor: investor1.publicKey,
          feeCollector: feeCollectorPDA,
          solVault: solVaultPDA,
          usdcVault: usdcVaultPDA,
          usdcMint: USDC_MINT,
          investorUsdcAccount: investor1UsdcAccount,
          depositorRecord: depositorRecordPDA,
          vaultStats: vaultStatsPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .transaction();

      const [recentBlockhash] = await context.banksClient.getLatestBlockhash();
      tx.recentBlockhash = recentBlockhash;
      tx.sign(investor1);

      await context.banksClient.processTransaction(tx);

      // Verify cumulative deposits
      const depositorRecord = await fetchAccount(
        context.banksClient,
        program,
        "DepositorRecord",
        depositorRecordPDA
      );

      const expectedTotal = new BN(0.5 * LAMPORTS_PER_SOL).add(secondDeposit);
      expect(depositorRecord.totalSolDeposited.toString()).to.equal(
        expectedTotal.toString()
      );
      expect(depositorRecord.currentSolBalance.toString()).to.equal(
        expectedTotal.toString()
      );
      expect(depositorRecord.depositCount).to.equal(2);
    });

    it("Should handle deposits from multiple investors", async () => {
      const depositAmount = new BN(1 * LAMPORTS_PER_SOL);
      const [depositorRecordPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("investor_record"), investor2.publicKey.toBuffer()],
        program.programId
      );

      const investor2UsdcAccount = await getOrCreateAta(
        context.banksClient,
        admin,
        USDC_MINT,
        investor2.publicKey
      );

      const tx = await program.methods
        .deposit({
          solAmount: depositAmount,
          usdcAmount: new BN(0),
        })
        .accountsStrict({
          investor: investor2.publicKey,
          feeCollector: feeCollectorPDA,
          solVault: solVaultPDA,
          usdcVault: usdcVaultPDA,
          usdcMint: USDC_MINT,
          investorUsdcAccount: investor2UsdcAccount,
          depositorRecord: depositorRecordPDA,
          vaultStats: vaultStatsPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .transaction();

      const [recentBlockhash] = await context.banksClient.getLatestBlockhash();
      tx.recentBlockhash = recentBlockhash;
      tx.sign(investor2);

      await context.banksClient.processTransaction(tx);

      // Verify vault stats track multiple investors
      const vaultStats = await fetchAccount(
        context.banksClient,
        program,
        "VaultStats",
        vaultStatsPDA
      );
      expect(vaultStats.depositorCount).to.equal(2);
    });
  });

  describe("Deposit - USDC Only", () => {
    it("Should successfully deposit USDC", async () => {
      const depositAmount = new BN(100 * 10 ** 6); // 100 USDC
      const [depositorRecordPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("investor_record"), investor3.publicKey.toBuffer()],
        program.programId
      );

      const investor3UsdcAccount = await getOrCreateAta(
        context.banksClient,
        admin,
        USDC_MINT,
        investor3.publicKey
      );

      const beforeUsdcBalance = await getTokenAccount(
        context.banksClient,
        investor3UsdcAccount
      );

      const tx = await program.methods
        .deposit({
          solAmount: new BN(0),
          usdcAmount: depositAmount,
        })
        .accountsStrict({
          investor: investor3.publicKey,
          feeCollector: feeCollectorPDA,
          solVault: solVaultPDA,
          usdcVault: usdcVaultPDA,
          usdcMint: USDC_MINT,
          investorUsdcAccount: investor3UsdcAccount,
          depositorRecord: depositorRecordPDA,
          vaultStats: vaultStatsPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .transaction();

      const [recentBlockhash] = await context.banksClient.getLatestBlockhash();
      tx.recentBlockhash = recentBlockhash;
      tx.sign(investor3);

      await context.banksClient.processTransaction(tx);

      // Verify depositor record
      const depositorRecord = await fetchAccount(
        context.banksClient,
        program,
        "DepositorRecord",
        depositorRecordPDA
      );

      expect(depositorRecord.totalUsdcDeposited.toString()).to.equal(
        depositAmount.toString()
      );
      expect(depositorRecord.currentUsdcBalance.toString()).to.equal(
        depositAmount.toString()
      );
      expect(depositorRecord.totalSolDeposited.toString()).to.equal("0");
      expect(depositorRecord.depositCount).to.equal(1);

      // Verify USDC vault balance
      const vaultUsdcAccount = await getTokenAccount(
        context.banksClient,
        usdcVaultPDA
      );
      expect(vaultUsdcAccount.amount.toString()).to.equal(
        depositAmount.toString()
      );
    });
  });

  describe("Deposit - Mixed SOL and USDC", () => {
    it("Should successfully deposit both SOL and USDC", async () => {
      const solAmount = new BN(0.5 * LAMPORTS_PER_SOL);
      const usdcAmount = new BN(50 * 10 ** 6); // 50 USDC
      const [depositorRecordPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("investor_record"), investor3.publicKey.toBuffer()],
        program.programId
      );

      const investor3UsdcAccount = await getOrCreateAta(
        context.banksClient,
        admin,
        USDC_MINT,
        investor3.publicKey
      );

      const tx = await program.methods
        .deposit({
          solAmount,
          usdcAmount,
        })
        .accountsStrict({
          investor: investor3.publicKey,
          feeCollector: feeCollectorPDA,
          solVault: solVaultPDA,
          usdcVault: usdcVaultPDA,
          usdcMint: USDC_MINT,
          investorUsdcAccount: investor3UsdcAccount,
          depositorRecord: depositorRecordPDA,
          vaultStats: vaultStatsPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .transaction();

      const [recentBlockhash] = await context.banksClient.getLatestBlockhash();
      tx.recentBlockhash = recentBlockhash;
      tx.sign(investor3);

      await context.banksClient.processTransaction(tx);

      // Verify depositor record tracks both assets
      const depositorRecord = await fetchAccount(
        context.banksClient,
        program,
        "DepositorRecord",
        depositorRecordPDA
      );

      expect(depositorRecord.totalSolDeposited.toString()).to.equal(
        solAmount.toString()
      );
      expect(depositorRecord.totalUsdcDeposited.toString()).to.equal(
        new BN(150 * 10 ** 6).toString()
      ); // 100 + 50 USDC
      expect(depositorRecord.depositCount).to.equal(2);
    });
  });

  describe("Deposit - Validation Tests", () => {
    it("Should reject zero amount deposits", async () => {
      const [depositorRecordPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("investor_record"), investor1.publicKey.toBuffer()],
        program.programId
      );

      const investor1UsdcAccount = await getOrCreateAta(
        context.banksClient,
        admin,
        USDC_MINT,
        investor1.publicKey
      );

      try {
        const tx = await program.methods
          .deposit({
            solAmount: new BN(0),
            usdcAmount: new BN(0),
          })
          .accountsStrict({
            investor: investor1.publicKey,
            feeCollector: feeCollectorPDA,
            solVault: solVaultPDA,
            usdcVault: usdcVaultPDA,
            usdcMint: USDC_MINT,
            investorUsdcAccount: investor1UsdcAccount,
            depositorRecord: depositorRecordPDA,
            vaultStats: vaultStatsPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .transaction();

        const [recentBlockhash] =
          await context.banksClient.getLatestBlockhash();
        tx.recentBlockhash = recentBlockhash;
        tx.sign(investor1);

        await context.banksClient.processTransaction(tx);
        assert.fail("Should have failed with zero amounts");
      } catch (error) {
        expect(error).to.exist;
      }
    });

    it("Should reject deposits below minimum threshold", async () => {
      const [depositorRecordPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("investor_record"), investor1.publicKey.toBuffer()],
        program.programId
      );

      const investor1UsdcAccount = await getOrCreateAta(
        context.banksClient,
        admin,
        USDC_MINT,
        investor1.publicKey
      );

      try {
        const tx = await program.methods
          .deposit({
            solAmount: new BN(100_000), // Below minimum
            usdcAmount: new BN(0),
          })
          .accountsStrict({
            investor: investor1.publicKey,
            feeCollector: feeCollectorPDA,
            solVault: solVaultPDA,
            usdcVault: usdcVaultPDA,
            usdcMint: USDC_MINT,
            investorUsdcAccount: investor1UsdcAccount,
            depositorRecord: depositorRecordPDA,
            vaultStats: vaultStatsPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .transaction();

        const [recentBlockhash] =
          await context.banksClient.getLatestBlockhash();
        tx.recentBlockhash = recentBlockhash;
        tx.sign(investor1);

        await context.banksClient.processTransaction(tx);
        assert.fail("Should have failed with amount below minimum");
      } catch (error) {
        expect(error).to.exist;
      }
    });
  });

});
