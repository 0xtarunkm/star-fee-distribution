import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import type { BanksClient, ProgramTestContext } from "solana-bankrun";
import { startAnchor } from "solana-bankrun";
import {
  ACCOUNT_SIZE,
  AccountLayout,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
  MintLayout,
  createTransferInstruction,
} from "@solana/spl-token";
import type { BN } from "bn.js";

export const LOCAL_ADMIN_KEYPAIR = Keypair.fromSecretKey(
  Uint8Array.from([
    230, 207, 238, 109, 95, 154, 47, 93, 183, 250, 147, 189, 87, 15, 117, 184,
    44, 91, 94, 231, 126, 140, 238, 134, 29, 58, 8, 182, 88, 22, 113, 234, 8,
    234, 192, 109, 87, 125, 190, 55, 129, 173, 227, 8, 104, 201, 104, 13, 31,
    178, 74, 80, 54, 14, 77, 78, 226, 57, 47, 122, 166, 165, 57, 144,
  ])
);

export const USDC_MINT = new PublicKey(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
);

export const ADMIN_USDC_ATA = getAssociatedTokenAddressSync(
  USDC_MINT,
  LOCAL_ADMIN_KEYPAIR.publicKey,
  true
);

const usdcToOwn = 1_000_000_000_000;
const tokenAccData = Buffer.alloc(ACCOUNT_SIZE);
AccountLayout.encode(
  {
    mint: USDC_MINT,
    owner: LOCAL_ADMIN_KEYPAIR.publicKey,
    amount: BigInt(usdcToOwn),
    delegateOption: 0,
    delegate: PublicKey.default,
    delegatedAmount: BigInt(0),
    state: 1,
    isNativeOption: 0,
    isNative: BigInt(0),
    closeAuthorityOption: 0,
    closeAuthority: PublicKey.default,
  },
  tokenAccData
);

export async function startTest() {
  return startAnchor(
    "./",
    [
      {
        name: "star_fee_distribution",
        programId: new PublicKey("FAAk54pcwJFvHD76YaB5sZzqXCEhUCVpP3cBvggKofuS"),
      },
    ],
    [
      {
        address: LOCAL_ADMIN_KEYPAIR.publicKey,
        info: {
          executable: false,
          owner: SystemProgram.programId,
          lamports: LAMPORTS_PER_SOL * 1000,
          data: new Uint8Array(),
        },
      },
      {
        address: ADMIN_USDC_ATA,
        info: {
          lamports: 1_000_000_000,
          data: tokenAccData,
          owner: TOKEN_PROGRAM_ID,
          executable: false,
        },
      },
      {
        address: USDC_MINT,
        info: {
          lamports: 1_000_000_000,
          data: createMintData(USDC_MINT, 6, LOCAL_ADMIN_KEYPAIR.publicKey),
          owner: TOKEN_PROGRAM_ID,
          executable: false,
        },
      },
    ]
  );
}

function createMintData(mint: PublicKey, decimals: number, mintAuthority: PublicKey): Uint8Array {
  const mintData = Buffer.alloc(82);
  MintLayout.encode(
    {
      mintAuthorityOption: 1,
      mintAuthority: mintAuthority,
      supply: BigInt(usdcToOwn * 100),
      decimals,
      isInitialized: true,
      freezeAuthorityOption: 0,
      freezeAuthority: PublicKey.default,
    },
    mintData
  );
  return mintData;
}

export async function fundSol(
  banksClient: BanksClient,
  payer: Keypair,
  receivers: PublicKey[]
) {
  const instructions: TransactionInstruction[] = [];
  for (const receiver of receivers) {
    instructions.push(
      SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: receiver,
        lamports: BigInt(100 * LAMPORTS_PER_SOL),
      })
    );
  }

  let transaction = new Transaction();
  const [recentBlockhash] = await banksClient.getLatestBlockhash();
  transaction.recentBlockhash = recentBlockhash;
  transaction.add(...instructions);
  transaction.sign(payer);

  await banksClient.processTransaction(transaction);
}

export async function getOrCreateAta(
  banksClient: BanksClient,
  payer: Keypair,
  mint: PublicKey,
  owner: PublicKey
) {
  const ataKey = getAssociatedTokenAddressSync(mint, owner, true);

  const account = await banksClient.getAccount(ataKey);
  if (account === null) {
    const createAtaIx = createAssociatedTokenAccountInstruction(
      payer.publicKey,
      ataKey,
      owner,
      mint
    );
    let transaction = new Transaction();
    const [recentBlockhash] = await banksClient.getLatestBlockhash();
    transaction.recentBlockhash = recentBlockhash;
    transaction.add(createAtaIx);
    transaction.sign(payer);
    await banksClient.processTransaction(transaction);
  }

  return ataKey;
}

export async function fundUsdc(
  banksClient: BanksClient,
  receivers: PublicKey[]
) {
  const getOrCreatePromise = receivers.map((acc: PublicKey) =>
    getOrCreateAta(banksClient, LOCAL_ADMIN_KEYPAIR, USDC_MINT, acc)
  );

  const atas = await Promise.all(getOrCreatePromise);

  const instructions: TransactionInstruction[] = atas.map((ata: PublicKey) =>
    createTransferInstruction(
      ADMIN_USDC_ATA,
      ata,
      LOCAL_ADMIN_KEYPAIR.publicKey,
      BigInt(100_000 * 10 ** 6)
    )
  );

  let transaction = new Transaction();
  const [recentBlockhash] = await banksClient.getLatestBlockhash();
  transaction.recentBlockhash = recentBlockhash;
  transaction.add(...instructions);
  transaction.sign(LOCAL_ADMIN_KEYPAIR);

  await banksClient.processTransaction(transaction);
}

export async function getTokenAccount(
  banksClient: BanksClient,
  key: PublicKey
) {
  const account = await banksClient.getAccount(key);
  if (!account) {
    return null;
  }
  const tokenAccountState = AccountLayout.decode(account.data);
  return tokenAccountState;
}

export async function getBalance(banksClient: BanksClient, wallet: PublicKey) {
  const account = await banksClient.getAccount(wallet);
  return account.lamports;
}

export async function getCurrentSlot(banksClient: BanksClient): Promise<number> {
  let slot = await banksClient.getSlot();
  return Number(slot.toString());
}

export async function warpSlotBy(context: ProgramTestContext, slots: number) {
  const clock = await context.banksClient.getClock();
  await context.warpToSlot(clock.slot + BigInt(slots.toString()));
}

export async function fetchAccount(
  banksClient: BanksClient,
  program: any,
  accountName: string,
  address: PublicKey
) {
  try {
    const account = await banksClient.getAccount(address);
    if (!account) {
      return null;
    }
    
    // Decode using the program's account coder - try camelCase first
    const accountNameCamel = accountName.charAt(0).toLowerCase() + accountName.slice(1);
    try {
      const decoded = program.coder.accounts.decode(accountNameCamel, Buffer.from(account.data));
      return decoded;
    } catch (e1) {
      // Try with the original name as fallback
      try {
        const decoded = program.coder.accounts.decode(accountName, Buffer.from(account.data));
        return decoded;
      } catch (e2) {
        return null;
      }
    }
  } catch (error: any) {
    return null;
  }
}