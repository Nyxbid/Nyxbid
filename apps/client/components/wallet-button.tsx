"use client";

import { useWallet } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";

export function WalletButton() {
  const { publicKey, disconnect, connected } = useWallet();
  const { setVisible } = useWalletModal();

  if (connected && publicKey) {
    const pk = publicKey.toBase58();
    const display = `${pk.slice(0, 4)}…${pk.slice(-4)}`;
    return (
      <div className="space-y-2">
        <div className="flex items-center gap-2">
          <span className="inline-block h-1.5 w-1.5 rounded-full bg-[var(--buy)]" />
          <p className="truncate font-mono text-[11px] text-foreground" title={pk}>
            {display}
          </p>
        </div>
        <button
          onClick={() => disconnect()}
          className="h-7 w-full rounded-[var(--r-sm)] border border-[var(--hairline-strong)] px-2 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:bg-[var(--surface)] hover:text-foreground"
        >
          disconnect
        </button>
      </div>
    );
  }

  return (
    <button
      onClick={() => setVisible(true)}
      className="h-9 w-full rounded-[var(--r-sm)] bg-[var(--accent)] px-3 text-[12px] font-medium tracking-tight text-[var(--accent-fg)] transition-colors hover:bg-[var(--accent-soft)]"
    >
      Connect wallet
    </button>
  );
}
