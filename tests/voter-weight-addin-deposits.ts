import * as assert from "assert";
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { createMintAndVault } from "@project-serum/common";
import BN from "bn.js";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  Token,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { VoterWeightAddinDeposits } from "../target/types/voter_weight_addin_deposits";

const SYSVAR_INSTRUCTIONS_PUBKEY = new PublicKey(
  "Sysvar1nstructions1111111111111111111111111"
);

describe("voting-rights", () => {
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace
    .VoterWeightAddinDeposits as Program<VoterWeightAddinDeposits>;

  // Initialized variables shared across tests.
  const governanceProgramId = new PublicKey("GovernanceProgram11111111111111111111111111");
  const realm = Keypair.generate().publicKey;
  const votingMintDecimals = 6;
  const tokenProgram = TOKEN_PROGRAM_ID;
  const associatedTokenProgram = ASSOCIATED_TOKEN_PROGRAM_ID;
  const rent = SYSVAR_RENT_PUBKEY;
  const systemProgram = SystemProgram.programId;

  // Uninitialized variables shared across tests.
  let registrar: PublicKey,
    voter: PublicKey,
    voterWeightRecord: PublicKey,
    exchangeVault: PublicKey;
  let registrarBump: number,
    voterBump: number,
    voterWeightRecordBump: number;
  let mintA: PublicKey,
    godA: PublicKey,
    realmCommunityMint: PublicKey;
  let tokenAClient: Token;

  it("Creates tokens and mints", async () => {
    const [_mintA, _godA] = await createMintAndVault(
      program.provider,
      new BN("1000000000000000000"),
      undefined,
      6
    );

    mintA = _mintA;
    godA = _godA;
    realmCommunityMint = mintA;
  });

  it("Creates PDAs", async () => {
    const [_registrar, _registrarBump] = await PublicKey.findProgramAddress(
      [realm.toBuffer()],
      program.programId
    );
    const [_voter, _voterBump] = await PublicKey.findProgramAddress(
      [_registrar.toBuffer(), program.provider.wallet.publicKey.toBuffer()],
      program.programId
    );
    const [_voterWeightRecord, _voterWeightRecordBump] =
      await PublicKey.findProgramAddress(
        [
          anchor.utils.bytes.utf8.encode("voter-weight-record"),
          _registrar.toBuffer(),
          program.provider.wallet.publicKey.toBuffer(),
        ],
        program.programId
      );
    exchangeVault = await Token.getAssociatedTokenAddress(
      associatedTokenProgram,
      tokenProgram,
      mintA,
      _registrar,
      true
    );

    registrar = _registrar;
    voter = _voter;

    registrarBump = _registrarBump;
    voterBump = _voterBump;
    voterWeightRecord = _voterWeightRecord;
    voterWeightRecordBump = _voterWeightRecordBump;
  });

  it("Creates token clients", async () => {
    tokenAClient = new Token(
      program.provider.connection,
      mintA,
      TOKEN_PROGRAM_ID,
      // @ts-ignore
      program.provider.wallet.payer
    );
  });

  it("Initializes a registrar", async () => {
    await program.rpc.createRegistrar(registrarBump, {
      accounts: {
        registrar,
        governanceProgramId,
        realm,
        realmCommunityMint,
        authority: program.provider.wallet.publicKey,
        exchangeVault,
        depositMint: mintA,
        payer: program.provider.wallet.publicKey,
        systemProgram,
        tokenProgram,
        associatedTokenProgram,
        rent,
      },
    });
  });

  it("Initializes a voter", async () => {
    await program.rpc.createVoter(voterBump, voterWeightRecordBump, {
      accounts: {
        voter,
        voterWeightRecord,
        registrar,
        authority: program.provider.wallet.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram,
        associatedTokenProgram,
        tokenProgram,
        rent,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      },
    });
  });

  it("Deposits tokens", async () => {
    const amount = new BN(10);
    await program.rpc.deposit(amount, {
      accounts: {
        registrar,
        voter,
        exchangeVault: exchangeVault,
        depositMint: mintA,
        depositToken: godA,
        authority: program.provider.wallet.publicKey,
        tokenProgram,
        associatedTokenProgram,
        systemProgram,
        rent,
      },
    });

    const voterAccount = await program.account.voter.fetch(voter);
    assert.ok(voterAccount.amountDeposited.toNumber() === 10);
  });

  it("Deposits more tokens", async () => {
    const amount = new BN(11);
    await program.rpc.deposit(amount, {
      accounts: {
        registrar,
        voter,
        exchangeVault: exchangeVault,
        depositMint: mintA,
        depositToken: godA,
        authority: program.provider.wallet.publicKey,
        tokenProgram,
        associatedTokenProgram,
        systemProgram,
        rent,
      },
    });

    const voterAccount = await program.account.voter.fetch(voter);
    assert.ok(voterAccount.amountDeposited.toNumber() === 21);
  });

  /*
  it("Withdraws cliff locked A tokens", async () => {
    const depositId = 0;
    const amount = new BN(10);
    await program.rpc.withdraw(depositId, amount, {
      accounts: {
        registrar,
        voter,
        exchangeVault: exchangeVault,
        withdrawMint: mintA,
        votingToken,
        votingMint,
        destination: godA,
        authority: program.provider.wallet.publicKey,
        tokenProgram,
      },
    });

    const voterAccount = await program.account.voter.fetch(voter);
    const deposit = voterAccount.deposits[0];
    assert.ok(deposit.isUsed);
    assert.ok(deposit.amount.toNumber() === 0);
    assert.ok(deposit.rateIdx === 0);

    const vtAccount = await votingTokenClient.getAccountInfo(votingToken);
    assert.ok(vtAccount.amount.toNumber() === 0);
  });
  */


  it("Updates a vote weight record", async () => {
    await program.rpc.updateVoterWeightRecord({
      accounts: {
        registrar,
        voter,
        voterWeightRecord,
        authority: program.provider.wallet.publicKey,
        systemProgram,
      },
    });

    // TODO: Check it!
  });
});
