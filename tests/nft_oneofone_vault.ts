import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import { readFileSync } from "fs";
import { createMint, getOrCreateAssociatedTokenAccount, getAccount } from "@solana/spl-token";

// Load IDLs
const nftOneOfOneIdl = JSON.parse(readFileSync("target/idl/nft_oneofone.json", "utf8"));
const vaultProgramIdl = JSON.parse(readFileSync("target/idl/vault_program.json", "utf8"));

// Program IDs
const NFT_ONEOFONE_ID = "7J8MaTphGYZQ5mpjyvRUURPWwWWpxsCgaNi5oRRnNPV";
const VAULT_PROGRAM_ID = "9MfkrraCMGedGbs9gvZ7JUXt1ghGpAzATh5PdEWrc665";

describe("nft_oneofone + vault integration", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const connection = provider.connection;
  const wallet = provider.wallet;

  // Load programs
  const nftOneOfOne = new anchor.Program(
    nftOneOfOneIdl,
    NFT_ONEOFONE_ID,
    provider
  );
  const vaultProgram = new anchor.Program(
    vaultProgramIdl,
    VAULT_PROGRAM_ID,
    provider
  );

  let mint: anchor.web3.PublicKey;
  let userTokenAccount: anchor.web3.PublicKey;
  let vaultTokenAccount: anchor.web3.PublicKey;

  it("mints a 1-of-1 NFT and deposits to vault", async () => {
    // 1. Create a new mint (NFT)
    mint = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      0 // 0 decimals for NFT
    );

    // 2. Create user's associated token account for the NFT
    const userAta = await getOrCreateAssociatedTokenAccount(
      connection,
      wallet.payer,
      mint,
      wallet.publicKey
    );
    userTokenAccount = userAta.address;

    // 3. Mint the NFT to the user using the program
    await nftOneOfOne.methods
      .mintOneOfOne()
      .accounts({
        mint,
        recipient: userTokenAccount,
        authority: wallet.publicKey,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([])
      .rpc();

    // Assert user received the NFT
    let userAccount = await getAccount(connection, userTokenAccount);
    assert.strictEqual(Number(userAccount.amount), 1, "User should have 1 NFT");

    // 4. Create the vault's token account (owned by the wallet for this test)
    const vaultAta = await getOrCreateAssociatedTokenAccount(
      connection,
      wallet.payer,
      mint,
      wallet.publicKey // In production, this would be a PDA or vault authority
    );
    vaultTokenAccount = vaultAta.address;

    // 5. Deposit the NFT into the vault
    await vaultProgram.methods
      .deposit(new anchor.BN(1))
      .accounts({
        userTokenAccount: userTokenAccount,
        vaultTokenAccount: vaultTokenAccount,
        user: wallet.publicKey,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([])
      .rpc();

    // Assert vault received the NFT
    let vaultAccount = await getAccount(connection, vaultTokenAccount);
    userAccount = await getAccount(connection, userTokenAccount);
    assert.strictEqual(Number(vaultAccount.amount), 1, "Vault should have 1 NFT");
    assert.strictEqual(Number(userAccount.amount), 0, "User should have 0 NFT after deposit");

    // 6. Withdraw the NFT back to the user
    await vaultProgram.methods
      .withdraw(new anchor.BN(1))
      .accounts({
        vaultTokenAccount: vaultTokenAccount,
        userTokenAccount: userTokenAccount,
        vaultAuthority: wallet.publicKey,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([])
      .rpc();

    // Assert user has the NFT again
    vaultAccount = await getAccount(connection, vaultTokenAccount);
    userAccount = await getAccount(connection, userTokenAccount);
    assert.strictEqual(Number(userAccount.amount), 1, "User should have 1 NFT after withdraw");
    assert.strictEqual(Number(vaultAccount.amount), 0, "Vault should have 0 NFT after withdraw");
  });
}); 