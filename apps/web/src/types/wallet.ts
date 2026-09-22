export type WalletNetwork = "TESTNET" | "PUBLIC" | "FUTURENET" | "STANDALONE" | "UNKNOWN";

export type UserRole = "buyer" | "supplier" | "resolver" | "engine" | "observer";

export interface UserProfile {
  id: string;
  role: UserRole;
  roleLabel: string;
  name: string;
  address: string;
  balance: string;
  assetCode: string;
  isExternalWallet?: boolean;
}

export interface WalletState {
  isFreighterInstalled: boolean;
  isConnected: boolean;
  isConnecting: boolean;
  address: string | null;
  network: WalletNetwork;
  networkPassphrase?: string;
  isMainnetBlocked: boolean;
  activeProfile: UserProfile;
  availableProfiles: UserProfile[];
  connectFreighter: () => Promise<void>;
  disconnectFreighter: () => void;
  switchProfile: (role: UserRole) => void;
  signTransactionGuard: (xdr: string) => Promise<{
    success: boolean;
    error?: string;
    signedXdr?: string;
  }>;
}
