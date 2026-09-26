"use client";

import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useMemo,
} from "react";
import {
  isConnected as checkFreighterConnected,
  getNetwork as getFreighterNetwork,
  getNetworkDetails,
  getAddress as getFreighterAddress,
  requestAccess as requestFreighterAccess,
  signTransaction as signFreighterTransaction,
} from "@stellar/freighter-api";
import {
  WalletNetwork,
  WalletState,
  TESTNET_PASSPHRASE,
} from "@/types/wallet";
import { signWithFreighterGuard } from "@/lib/sign-transaction-guard";

const WalletContext = createContext<WalletState | undefined>(undefined);

const STORAGE_KEY_WALLET_CONNECTED = "cangupay_freighter_connected";

type FreighterNetworkDetails = {
  network?: unknown;
  networkPassphrase?: unknown;
  networkDetails?: unknown;
};

/** Freighter 6's runtime wraps this result in `networkDetails`, despite its declarations. */
export function normalizeFreighterNetworkDetails(value: unknown): {
  network: WalletNetwork;
  passphrase: string | null;
} | null {
  if (!value || typeof value !== "object") return null;
  const result = value as FreighterNetworkDetails;
  const details = result.networkDetails && typeof result.networkDetails === "object"
    ? result.networkDetails as FreighterNetworkDetails
    : result;
  const rawNet = typeof details.network === "string" ? details.network.toUpperCase() : "";
  const passphrase = typeof details.networkPassphrase === "string" ? details.networkPassphrase : null;

  let network: WalletNetwork = "UNKNOWN";
  if (rawNet.includes("PUBLIC") || rawNet.includes("MAIN")) network = "PUBLIC";
  else if (rawNet.includes("FUTURE")) network = "FUTURENET";
  else if (rawNet.includes("STANDALONE")) network = "STANDALONE";
  else if (rawNet.includes("TESTNET") || rawNet.includes("TEST")) network = "TESTNET";

  return { network, passphrase };
}

/**
 * Fail-closed network and passphrase detection.
 * Never defaults to TESTNET in catch blocks.
 */
async function queryFreighterNetwork(): Promise<{
  network: WalletNetwork;
  passphrase: string | null;
}> {
  try {
    const netDetails = await getNetworkDetails();
    const details = normalizeFreighterNetworkDetails(netDetails);
    if (details) return details;
  } catch {
    // Fail-closed: do not assume or fallback to TESTNET
  }

  try {
    const rawNetwork = await getFreighterNetwork();
    const netStr = (
      typeof rawNetwork === "string"
        ? rawNetwork
        : (rawNetwork as { network?: string })?.network || ""
    ).toUpperCase();

    let netType: WalletNetwork = "UNKNOWN";
    if (netStr.includes("PUBLIC") || netStr.includes("MAIN")) {
      netType = "PUBLIC";
    } else if (netStr.includes("FUTURE")) {
      netType = "FUTURENET";
    } else if (netStr.includes("STANDALONE")) {
      netType = "STANDALONE";
    } else if (netStr.includes("TESTNET") || netStr.includes("TEST")) {
      netType = "TESTNET";
    }

    return {
      network: netType,
      passphrase: null,
    };
  } catch {
    return {
      network: "UNKNOWN",
      passphrase: null,
    };
  }
}

export function WalletProvider({ children }: { children: React.ReactNode }) {
  const [isFreighterInstalled, setIsFreighterInstalled] = useState(false);
  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [address, setAddress] = useState<string | null>(null);
  const [network, setNetwork] = useState<WalletNetwork>("UNKNOWN");
  const [networkPassphrase, setNetworkPassphrase] = useState<string | null>(null);

  // Check Freighter extension availability and network state
  const refreshFreighterState = useCallback(async () => {
    if (typeof window === "undefined") return;

    try {
      const installedRes = await checkFreighterConnected();
      const installed =
        typeof installedRes === "boolean"
          ? installedRes
          : (installedRes as { isConnected?: boolean })?.isConnected ?? false;

      setIsFreighterInstalled(installed);

      if (installed) {
        // Read network configuration fail-closed
        const netInfo = await queryFreighterNetwork();
        setNetwork(netInfo.network);
        setNetworkPassphrase(netInfo.passphrase);

        // Only restore connection if user explicitly chose to connect (stops auto-reconnect on disconnect)
        let isAllowedToConnect = false;
        try {
          isAllowedToConnect =
            localStorage.getItem(STORAGE_KEY_WALLET_CONNECTED) === "true";
        } catch {}

        if (isAllowedToConnect) {
          try {
            const addrRes = await getFreighterAddress();
            const pubKey =
              typeof addrRes === "string"
                ? addrRes
                : (addrRes as { address?: string })?.address;

            if (pubKey && pubKey.startsWith("G")) {
              setAddress(pubKey);
              setIsConnected(true);
            } else {
              setAddress(null);
              setIsConnected(false);
            }
          } catch {
            setAddress(null);
            setIsConnected(false);
          }
        } else {
          setAddress(null);
          setIsConnected(false);
        }
      } else {
        setAddress(null);
        setIsConnected(false);
        setNetwork("UNKNOWN");
        setNetworkPassphrase(null);
      }
    } catch {
      setIsFreighterInstalled(false);
      setAddress(null);
      setIsConnected(false);
      setNetwork("UNKNOWN");
      setNetworkPassphrase(null);
    }
  }, []);

  useEffect(() => {
    let isMounted = true;

    const timer = setTimeout(() => {
      if (isMounted) {
        void refreshFreighterState();
      }
    }, 0);

    const interval = setInterval(() => {
      if (isMounted) {
        void refreshFreighterState();
      }
    }, 4000);

    return () => {
      isMounted = false;
      clearTimeout(timer);
      clearInterval(interval);
    };
  }, [refreshFreighterState]);

  // Connect user Freighter wallet
  const connectFreighter = useCallback(async () => {
    setIsConnecting(true);
    try {
      const accessRes = await requestFreighterAccess();
      const grantedAddress =
        typeof accessRes === "string"
          ? accessRes
          : (accessRes as { address?: string })?.address;

      if (grantedAddress && grantedAddress.startsWith("G")) {
        try {
          localStorage.setItem(STORAGE_KEY_WALLET_CONNECTED, "true");
        } catch {}
        setAddress(grantedAddress);
        setIsConnected(true);
        await refreshFreighterState();
      }
    } catch (err) {
      console.warn("Freighter connection dismissed or failed:", err);
    } finally {
      setIsConnecting(false);
    }
  }, [refreshFreighterState]);

  const disconnectFreighter = useCallback(() => {
    try {
      localStorage.setItem(STORAGE_KEY_WALLET_CONNECTED, "false");
    } catch {}
    setIsConnected(false);
    setAddress(null);
  }, []);

  // Fail-closed network validation: only exact Testnet passphrase allows signing
  const isExactTestnet = networkPassphrase === TESTNET_PASSPHRASE;
  const isNetworkAllowed = isExactTestnet;
  const isSigningBlocked = !isExactTestnet;
  const isMainnetBlocked = network === "PUBLIC" || isSigningBlocked;

  const signTransactionGuard = useCallback(
    async (
      xdr: string
    ): Promise<{ success: boolean; error?: string; signedXdr?: string }> => {
      return signWithFreighterGuard({
        xdr,
        connectedAddress: address,
        isConnected,
        expectedPassphrase: TESTNET_PASSPHRASE,
        queryNetwork: queryFreighterNetwork,
        onNetworkChecked: (freshNetwork, freshPassphrase) => {
          setNetwork(freshNetwork);
          setNetworkPassphrase(freshPassphrase);
        },
        signTransaction: signFreighterTransaction,
      });
    },
    [isConnected, address]
  );

  const value = useMemo<WalletState>(
    () => ({
      isFreighterInstalled,
      isConnected,
      isConnecting,
      address,
      network,
      networkPassphrase,
      isExactTestnet,
      isNetworkAllowed,
      isSigningBlocked,
      isMainnetBlocked,
      connectFreighter,
      disconnectFreighter,
      signTransactionGuard,
    }),
    [
      isFreighterInstalled,
      isConnected,
      isConnecting,
      address,
      network,
      networkPassphrase,
      isExactTestnet,
      isNetworkAllowed,
      isSigningBlocked,
      isMainnetBlocked,
      connectFreighter,
      disconnectFreighter,
      signTransactionGuard,
    ]
  );

  return (
    <WalletContext.Provider value={value}>{children}</WalletContext.Provider>
  );
}

export function useWallet(): WalletState {
  const context = useContext(WalletContext);
  if (!context) {
    throw new Error("useWallet must be used within a WalletProvider");
  }
  return context;
}
