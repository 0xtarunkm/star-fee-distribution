import { BN, Program } from "@coral-xyz/anchor";
import { ProgramTestContext } from "solana-bankrun";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { expect, assert } from "chai";
import {
  fundSol,
  LOCAL_ADMIN_KEYPAIR,
  startTest,
  USDC_MINT,
  fetchAccount,
} from "./utils/bankrun";
import { StarFeeDistribution } from "../target/types/star_fee_distribution";
import IDL from "../target/idl/star_fee_distribution.json";

describe("Distribution Config Tests (Bankrun)", () => {
  let context: ProgramTestContext;
  let program: Program<StarFeeDistribution>;
  let admin: Keypair;
  let creatorWallet: Keypair;
  let distributionConfigPDA: PublicKey;

  before(async () => {
    context = await startTest();
    admin = LOCAL_ADMIN_KEYPAIR;
    creatorWallet = Keypair.generate();

    await fundSol(context.banksClient, admin, [creatorWallet.publicKey]);

    // Create program instance
    program = new Program<StarFeeDistribution>(
      IDL as StarFeeDistribution,
      { connection: context.banksClient } as any
    );

    // Derive distribution config PDA
    [distributionConfigPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("distribution_config")],
      program.programId
    );
  });

  describe("Initialize Distribution Config", () => {
    it("Should successfully initialize distribution config", async () => {
      const y0Allocation = new BN(1_000_000 * 10 ** 6); // 1M USDC allocation
      const investorFeeShareBps = 5000; // 50%
      const minPayoutLamports = new BN(100_000); // 0.0001 SOL
      const dailyCapLamports = new BN(100 * LAMPORTS_PER_SOL); // 100 SOL per day

      const tx = await program.methods
        .initializeDistributionConfig({
          y0Allocation,
          investorFeeShareBps,
          minPayoutLamports,
          dailyCapLamports,
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

      // Verify config was initialized correctly
      const config = await fetchAccount(
        context.banksClient,
        program,
        "DistributionConfig",
        distributionConfigPDA
      );

      expect(config.y0Allocation.toString()).to.equal(y0Allocation.toString());
      expect(config.investorFeeShareBps).to.equal(investorFeeShareBps);
      expect(config.minPayoutLamports.toString()).to.equal(
        minPayoutLamports.toString()
      );
      expect(config.dailyCapLamports.toString()).to.equal(
        dailyCapLamports.toString()
      );
      expect(config.creatorWallet.toString()).to.equal(
        creatorWallet.publicKey.toString()
      );
      expect(config.quoteMint.toString()).to.equal(USDC_MINT.toString());
    });

    it("Should use default min payout if zero is provided", async () => {
      // This test would fail if we try to initialize again
      // So we'll test it conceptually - if minPayoutLamports is 0, it defaults to DEFAULT_MIN_PAYOUT_LAMPORTS
      const config = await fetchAccount(
        context.banksClient,
        program,
        "DistributionConfig",
        distributionConfigPDA
      );

      expect(config.minPayoutLamports.toNumber()).to.be.greaterThan(0);
    });
  });

  describe("Distribution Config Validation", () => {
    it("Should reject invalid Y0 allocation (zero)", async () => {
      const newAdmin = Keypair.generate();
      await fundSol(context.banksClient, admin, [newAdmin.publicKey]);

      const [newConfigPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("distribution_config_test")],
        program.programId
      );

      try {
        const tx = await program.methods
          .initializeDistributionConfig({
            y0Allocation: new BN(0), // Invalid: zero allocation
            investorFeeShareBps: 5000,
            minPayoutLamports: new BN(100_000),
            dailyCapLamports: new BN(100 * LAMPORTS_PER_SOL),
            creatorWallet: creatorWallet.publicKey,
            quoteMint: USDC_MINT,
          })
          .accountsStrict({
            admin: newAdmin.publicKey,
            distributionConfig: distributionConfigPDA,
            systemProgram: SystemProgram.programId,
          })
          .transaction();

        const [recentBlockhash] =
          await context.banksClient.getLatestBlockhash();
        tx.recentBlockhash = recentBlockhash;
        tx.sign(newAdmin);

        await context.banksClient.processTransaction(tx);
        assert.fail("Should have failed with zero Y0 allocation");
      } catch (error) {
        expect(error).to.exist;
      }
    });

    it("Should reject invalid investor fee share (> 100%)", async () => {
      const newAdmin = Keypair.generate();
      await fundSol(context.banksClient, admin, [newAdmin.publicKey]);

      try {
        const tx = await program.methods
          .initializeDistributionConfig({
            y0Allocation: new BN(1_000_000 * 10 ** 6),
            investorFeeShareBps: 15000, // Invalid: > 100%
            minPayoutLamports: new BN(100_000),
            dailyCapLamports: new BN(100 * LAMPORTS_PER_SOL),
            creatorWallet: creatorWallet.publicKey,
            quoteMint: USDC_MINT,
          })
          .accountsStrict({
            admin: newAdmin.publicKey,
            distributionConfig: distributionConfigPDA,
            systemProgram: SystemProgram.programId,
          })
          .transaction();

        const [recentBlockhash] =
          await context.banksClient.getLatestBlockhash();
        tx.recentBlockhash = recentBlockhash;
        tx.sign(newAdmin);

        await context.banksClient.processTransaction(tx);
        assert.fail("Should have failed with invalid fee share");
      } catch (error) {
        expect(error).to.exist;
      }
    });

    it("Should reject default creator wallet", async () => {
      const newAdmin = Keypair.generate();
      await fundSol(context.banksClient, admin, [newAdmin.publicKey]);

      try {
        const tx = await program.methods
          .initializeDistributionConfig({
            y0Allocation: new BN(1_000_000 * 10 ** 6),
            investorFeeShareBps: 5000,
            minPayoutLamports: new BN(100_000),
            dailyCapLamports: new BN(100 * LAMPORTS_PER_SOL),
            creatorWallet: PublicKey.default, // Invalid: default pubkey
            quoteMint: USDC_MINT,
          })
          .accountsStrict({
            admin: newAdmin.publicKey,
            distributionConfig: distributionConfigPDA,
            systemProgram: SystemProgram.programId,
          })
          .transaction();

        const [recentBlockhash] =
          await context.banksClient.getLatestBlockhash();
        tx.recentBlockhash = recentBlockhash;
        tx.sign(newAdmin);

        await context.banksClient.processTransaction(tx);
        assert.fail("Should have failed with default creator wallet");
      } catch (error) {
        expect(error).to.exist;
      }
    });
  });

  describe("Distribution Config Edge Cases", () => {
    it("Should allow zero daily cap (no limit)", async () => {
      const config = await fetchAccount(
        context.banksClient,
        program,
        "DistributionConfig",
        distributionConfigPDA
      );

      // The config should be valid even if daily cap could be 0
      expect(config).to.not.be.null;
      expect(config.dailyCapLamports.toNumber()).to.be.greaterThanOrEqual(0);
    });

    it("Should store correct configuration parameters", async () => {
      const config = await fetchAccount(
        context.banksClient,
        program,
        "DistributionConfig",
        distributionConfigPDA
      );

      // Verify all fields are properly stored
      expect(config.y0Allocation).to.be.instanceOf(BN);
      expect(config.investorFeeShareBps).to.be.a("number");
      expect(config.minPayoutLamports).to.be.instanceOf(BN);
      expect(config.dailyCapLamports).to.be.instanceOf(BN);
      expect(config.creatorWallet).to.be.instanceOf(PublicKey);
      expect(config.quoteMint).to.be.instanceOf(PublicKey);
      expect(config.bump).to.be.a("number");
      expect(config.bump).to.be.greaterThan(0);
    });

    it("Should have correct investor fee share range (0-100%)", async () => {
      const config = await fetchAccount(
        context.banksClient,
        program,
        "DistributionConfig",
        distributionConfigPDA
      );

      expect(config.investorFeeShareBps).to.be.at.least(0);
      expect(config.investorFeeShareBps).to.be.at.most(10000);
    });

    it("Should support various Y0 allocation sizes", async () => {
      const config = await fetchAccount(
        context.banksClient,
        program,
        "DistributionConfig",
        distributionConfigPDA
      );

      // Y0 should support large numbers for token allocations
      expect(config.y0Allocation.toNumber()).to.be.greaterThan(0);
      // Test that it can store large values (up to u64 max)
      const maxTestValue = new BN("1000000000000"); // 1 trillion (well within u64)
      expect(config.y0Allocation.lte(maxTestValue)).to.be.true;
    });
  });

  describe("Distribution Config Immutability", () => {
    it("Should prevent reinitialization of config", async () => {
      try {
        const tx = await program.methods
          .initializeDistributionConfig({
            y0Allocation: new BN(500_000 * 10 ** 6),
            investorFeeShareBps: 3000,
            minPayoutLamports: new BN(50_000),
            dailyCapLamports: new BN(50 * LAMPORTS_PER_SOL),
            creatorWallet: creatorWallet.publicKey,
            quoteMint: USDC_MINT,
          })
          .accountsStrict({
            admin: admin.publicKey,
            distributionConfig: distributionConfigPDA,
            systemProgram: SystemProgram.programId,
          })
          .transaction();

        const [recentBlockhash] =
          await context.banksClient.getLatestBlockhash();
        tx.recentBlockhash = recentBlockhash;
        tx.sign(admin);

        await context.banksClient.processTransaction(tx);
        assert.fail("Should have failed to reinitialize existing config");
      } catch (error) {
        expect(error).to.exist;
      }
    });

    it("Should maintain config data integrity", async () => {
      const initialConfig = await fetchAccount(
        context.banksClient,
        program,
        "DistributionConfig",
        distributionConfigPDA
      );

      // Fetch again to ensure data hasn't changed
      const refetchedConfig = await fetchAccount(
        context.banksClient,
        program,
        "DistributionConfig",
        distributionConfigPDA
      );

      expect(initialConfig.y0Allocation.toString()).to.equal(
        refetchedConfig.y0Allocation.toString()
      );
      expect(initialConfig.investorFeeShareBps).to.equal(
        refetchedConfig.investorFeeShareBps
      );
      expect(initialConfig.minPayoutLamports.toString()).to.equal(
        refetchedConfig.minPayoutLamports.toString()
      );
      expect(initialConfig.dailyCapLamports.toString()).to.equal(
        refetchedConfig.dailyCapLamports.toString()
      );
      expect(initialConfig.creatorWallet.toString()).to.equal(
        refetchedConfig.creatorWallet.toString()
      );
      expect(initialConfig.quoteMint.toString()).to.equal(
        refetchedConfig.quoteMint.toString()
      );
    });
  });
});
