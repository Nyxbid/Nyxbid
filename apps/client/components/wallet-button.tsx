"use client";

import { useWallet } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";

export function WalletButton() {
  const { publicKey, disconnect, connected } = useWallet();
  const { setVisible } = useWalletModal();

  if (connected && publicKey) {
    const short = publicKey.toBase58();
    const display = `${short.slice(0, 4)}...${short.slice(-4)}`;

    return (
      <div className="space-y-2">
        <p className="truncate font-mono text-xs text-muted" title={short}>
          {display}
        </p>
        <button
          onClick={() => disconnect()}
          className="w-full rounded-md border border-border px-3 py-1.5 text-xs font-medium text-muted transition-colors hover:bg-accent/5 hover:text-foreground"
        >
          Disconnect
        </button>
      </div>
    );
  }

  return (
    <button
      onClick={() => setVisible(true)}
      className="w-full rounded-md bg-accent px-3 py-2 text-xs font-medium text-white transition-colors hover:bg-accent/90"
    >
      Connect Wallet
    </button>
  );
}
