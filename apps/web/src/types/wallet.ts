export type WalletNetwork = "TESTNET" | "PUBLIC" | "FUTURENET" | "STANDALONE" | "UNKNOWN";

export const TESTNET_PASSPHRASE = "Test SDF Network ; September 2015";

/**
 * Human roles derived strictly from connected wallet address against escrow parties.
 * Engine is not a human role and is not selectable.
 */
export type UserRole = "buyer" | "supplier" | "resolver" | "observer";

export interface WalletState {
  isFreighterInstalled: boolean;
  isConnected: boolean;
  isConnecting: boolean;
  address: string | null;
  network: WalletNetwork;
  networkPassphrase: string | null;
  isExactTestnet: boolean;
  isNetworkAllowed: boolean;
  isSigningBlocked: boolean;
  isMainnetBlocked: boolean;
  connectFreighter: () => Promise<void>;
  disconnectFreighter: () => void;
  signTransactionGuard: (xdr: string) => Promise<{
    success: boolean;
    error?: string;
    signedXdr?: string;
  }>;
}

