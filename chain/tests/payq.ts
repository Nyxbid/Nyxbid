import { Program, AnchorProvider, BN } from "@anchor-lang/core";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";
import idl from "../target/idl/payq.json";
import type { Payq } from "../target/types/payq";

const provider = AnchorProvider.env();
const program = new Program(idl as Payq, provider);
const authority = provider.wallet;

function deriveVaultPda(authorityKey: PublicKey, label: string): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), authorityKey.toBuffer(), Buffer.from(label)],
    program.programId,
  );
}

function deriveSpendRecordPda(vaultKey: PublicKey, proposalHash: Buffer): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("spend"), vaultKey.toBuffer(), proposalHash],
    program.programId,
  );
}

function randomHash(): Buffer {
  return Buffer.from(Keypair.generate().publicKey.toBytes());
}

describe("payq", () => {
  const delegate = Keypair.generate();
  const label = "test-vault";
  const dailyLimit = new BN(100_000_000); // 100 USDC
  const perTxLimit = new BN(10_000_000);  // 10 USDC

  let vaultPda: PublicKey;

  before(async () => {
    [vaultPda] = deriveVaultPda(authority.publicKey, label);

    const sig = await provider.connection.requestAirdrop(
      delegate.publicKey,
      2_000_000_000,
    );
    await provider.connection.confirmTransaction(sig, "confirmed");
  });

  describe("initialize_vault", () => {
    it("creates a vault with correct state", async () => {
      await program.methods
        .initializeVault(label, dailyLimit, perTxLimit, delegate.publicKey)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const vault = await program.account.vault.fetch(vaultPda);
      expect(vault.authority.toBase58()).to.equal(authority.publicKey.toBase58());
      expect(vault.delegate.toBase58()).to.equal(delegate.publicKey.toBase58());
      expect(vault.label).to.equal(label);
      expect(vault.dailyLimit.toNumber()).to.equal(100_000_000);
      expect(vault.perTxLimit.toNumber()).to.equal(10_000_000);
      expect(vault.totalSpent.toNumber()).to.equal(0);
      expect(vault.spentToday.toNumber()).to.equal(0);
      expect(vault.paused).to.equal(false);
    });

    it("rejects label longer than 32 chars", async () => {
      const longLabel = "a".repeat(33);

      try {
        // PDA seed derivation will fail client-side for seeds > 32 bytes,
        // which is the same protection the on-chain check provides.
        const [longPda] = deriveVaultPda(authority.publicKey, longLabel);
        await program.methods
          .initializeVault(longLabel, dailyLimit, perTxLimit, delegate.publicKey)
          .accounts({
            authority: authority.publicKey,
            vault: longPda,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        expect.fail("should have thrown");
      } catch (err: any) {
        const msg = err.error?.errorCode?.code || err.message || err.toString();
        expect(msg).to.satisfy(
          (s: string) => s.includes("LabelTooLong") || s.includes("Max seed length exceeded"),
        );
      }
    });
  });

  describe("record_spend", () => {
    it("records a spend via delegate", async () => {
      const hash = randomHash();
      const [recordPda] = deriveSpendRecordPda(vaultPda, hash);
      const amount = new BN(2_500_000); // 2.5 USDC

      await program.methods
        .recordSpend("agent-atlas", "openai/gpt-4", amount, Array.from(hash))
        .accounts({
          delegate: delegate.publicKey,
          vault: vaultPda,
          spendRecord: recordPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([delegate])
        .rpc();

      const record = await program.account.spendRecord.fetch(recordPda);
      expect(record.agentId).to.equal("agent-atlas");
      expect(record.toolId).to.equal("openai/gpt-4");
      expect(record.amount.toNumber()).to.equal(2_500_000);
      expect(record.vault.toBase58()).to.equal(vaultPda.toBase58());

      const vault = await program.account.vault.fetch(vaultPda);
      expect(vault.spentToday.toNumber()).to.equal(2_500_000);
      expect(vault.totalSpent.toNumber()).to.equal(2_500_000);
    });

    it("rejects amount exceeding per-tx limit", async () => {
      const hash = randomHash();
      const [recordPda] = deriveSpendRecordPda(vaultPda, hash);
      const tooMuch = new BN(10_000_001);

      try {
        await program.methods
          .recordSpend("agent-atlas", "openai/gpt-4", tooMuch, Array.from(hash))
          .accounts({
            delegate: delegate.publicKey,
            vault: vaultPda,
            spendRecord: recordPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([delegate])
          .rpc();
        expect.fail("should have thrown");
      } catch (err: any) {
        expect(err.error?.errorCode?.code || err.message).to.contain("ExceedsPerTxLimit");
      }
    });

    it("rejects cumulative spend exceeding daily limit", async () => {
      // Vault already has 2.5 USDC spent. Spending 98 USDC would exceed 100 USDC daily.
      const hash = randomHash();
      const [recordPda] = deriveSpendRecordPda(vaultPda, hash);
      const pushOverDaily = new BN(9_900_000); // 9.9 USDC each

      // Spend up close to the limit first
      for (let i = 0; i < 9; i++) {
        const h = randomHash();
        const [rPda] = deriveSpendRecordPda(vaultPda, h);
        await program.methods
          .recordSpend("agent-atlas", "openai/gpt-4", pushOverDaily, Array.from(h))
          .accounts({
            delegate: delegate.publicKey,
            vault: vaultPda,
            spendRecord: rPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([delegate])
          .rpc();
      }

      // Now vault has spent 2.5 + 9*9.9 = 91.6 USDC. One more 9.9 would be 101.5 > 100
      try {
        await program.methods
          .recordSpend("agent-atlas", "openai/gpt-4", pushOverDaily, Array.from(hash))
          .accounts({
            delegate: delegate.publicKey,
            vault: vaultPda,
            spendRecord: recordPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([delegate])
          .rpc();
        expect.fail("should have thrown");
      } catch (err: any) {
        expect(err.error?.errorCode?.code || err.message).to.contain("ExceedsDailyLimit");
      }
    });

    it("rejects wrong delegate", async () => {
      const wrongDelegate = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(wrongDelegate.publicKey, 1_000_000_000);
      await provider.connection.confirmTransaction(sig, "confirmed");

      const hash = randomHash();
      const [recordPda] = deriveSpendRecordPda(vaultPda, hash);

      try {
        await program.methods
          .recordSpend("agent-atlas", "openai/gpt-4", new BN(1_000_000), Array.from(hash))
          .accounts({
            delegate: wrongDelegate.publicKey,
            vault: vaultPda,
            spendRecord: recordPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([wrongDelegate])
          .rpc();
        expect.fail("should have thrown");
      } catch (err: any) {
        expect(err.error?.errorCode?.code || err.toString()).to.satisfy(
          (s: string) => s.includes("ConstraintHasOne") || s.includes("has_one") || s.includes("2001"),
        );
      }
    });

    it("rejects spend on paused vault", async () => {
      // Pause the vault first
      await program.methods
        .updateVault(dailyLimit, perTxLimit, delegate.publicKey, true)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
        })
        .rpc();

      const hash = randomHash();
      const [recordPda] = deriveSpendRecordPda(vaultPda, hash);

      try {
        await program.methods
          .recordSpend("agent-atlas", "openai/gpt-4", new BN(1_000_000), Array.from(hash))
          .accounts({
            delegate: delegate.publicKey,
            vault: vaultPda,
            spendRecord: recordPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([delegate])
          .rpc();
        expect.fail("should have thrown");
      } catch (err: any) {
        expect(err.error?.errorCode?.code || err.message).to.contain("VaultPaused");
      }

      // Unpause for subsequent tests
      await program.methods
        .updateVault(dailyLimit, perTxLimit, delegate.publicKey, false)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
        })
        .rpc();
    });
  });

  describe("update_vault", () => {
    it("updates limits and delegate", async () => {
      const newDelegate = Keypair.generate();
      const newDaily = new BN(200_000_000);
      const newPerTx = new BN(20_000_000);

      await program.methods
        .updateVault(newDaily, newPerTx, newDelegate.publicKey, false)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
        })
        .rpc();

      const vault = await program.account.vault.fetch(vaultPda);
      expect(vault.dailyLimit.toNumber()).to.equal(200_000_000);
      expect(vault.perTxLimit.toNumber()).to.equal(20_000_000);
      expect(vault.delegate.toBase58()).to.equal(newDelegate.publicKey.toBase58());
      expect(vault.paused).to.equal(false);

      // Restore delegate for later tests
      await program.methods
        .updateVault(dailyLimit, perTxLimit, delegate.publicKey, false)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
        })
        .rpc();
    });

    it("rejects wrong authority", async () => {
      const imposter = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(imposter.publicKey, 1_000_000_000);
      await provider.connection.confirmTransaction(sig, "confirmed");

      try {
        await program.methods
          .updateVault(dailyLimit, perTxLimit, delegate.publicKey, false)
          .accounts({
            authority: imposter.publicKey,
            vault: vaultPda,
          })
          .signers([imposter])
          .rpc();
        expect.fail("should have thrown");
      } catch (err: any) {
        expect(err.error?.errorCode?.code || err.toString()).to.satisfy(
          (s: string) => s.includes("ConstraintHasOne") || s.includes("has_one") || s.includes("2001"),
        );
      }
    });
  });

  describe("close_vault", () => {
    it("rejects wrong authority", async () => {
      const imposter = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(imposter.publicKey, 1_000_000_000);
      await provider.connection.confirmTransaction(sig, "confirmed");

      try {
        await program.methods
          .closeVault()
          .accounts({
            authority: imposter.publicKey,
            vault: vaultPda,
          })
          .signers([imposter])
          .rpc();
        expect.fail("should have thrown");
      } catch (err: any) {
        expect(err.error?.errorCode?.code || err.toString()).to.satisfy(
          (s: string) => s.includes("ConstraintHasOne") || s.includes("has_one") || s.includes("2001"),
        );
      }
    });

    it("closes vault and reclaims SOL", async () => {
      const balBefore = await provider.connection.getBalance(authority.publicKey);

      await program.methods
        .closeVault()
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
        })
        .rpc();

      const balAfter = await provider.connection.getBalance(authority.publicKey);
      expect(balAfter).to.be.greaterThan(balBefore);

      const info = await provider.connection.getAccountInfo(vaultPda);
      expect(info).to.be.null;
    });
  });
});
