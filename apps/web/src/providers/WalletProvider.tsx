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
  UserProfile,
  UserRole,
  WalletNetwork,
  WalletState,
} from "@/types/wallet";

const DEMO_PROFILES: UserProfile[] = [
  {
    id: "buyer-profile",
    role: "buyer",
    roleLabel: "Comprador (Buyer)",
    name: "Acme Industrial Corp",
    address: "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z",
    balance: "50,000.0000000",
    assetCode: "CPUSD",
  },
  {
    id: "supplier-profile",
    role: "supplier",
    roleLabel: "Proveedor (Supplier)",
    name: "Valle Logistics S.A.",
    address: "GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L",
    balance: "12,500.0000000",
    assetCode: "CPUSD",
  },
  {
    id: "resolver-profile",
    role: "resolver",
    roleLabel: "Árbitro (Resolver)",
    name: "Tribunal Arbitral de Comercio",
    address: "GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R",
    balance: "5,000.0000000",
    assetCode: "CPUSD",
  },
  {
    id: "engine-profile",
    role: "engine",
    roleLabel: "Motor de Reglas (Engine)",
    name: "CanguPay Verification Bot",
    address: "GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M",
    balance: "10,000.0000000",
    assetCode: "CPUSD",
  },
];

const WalletContext = createContext<WalletState | undefined>(undefined);

const STORAGE_KEY_WALLET_CONNECTED = "cangupay_freighter_connected";

export function WalletProvider({ children }: { children: React.ReactNode }) {
  const [isFreighterInstalled, setIsFreighterInstalled] = useState(false);
  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [address, setAddress] = useState<string | null>(null);
  const [network, setNetwork] = useState<WalletNetwork>("TESTNET");
  const [networkPassphrase, setNetworkPassphrase] = useState<string | undefined>(
    "Test SDF Network ; September 2015"
  );
  const [activeRole, setActiveRole] = useState<UserRole>("buyer");

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
        // Read network configuration
        try {
          const netDetails = await getNetworkDetails();
          if (netDetails && netDetails.network) {
            const rawNet = (netDetails.network || "").toUpperCase();
            if (rawNet.includes("PUBLIC") || rawNet.includes("MAIN")) {
              setNetwork("PUBLIC");
            } else if (rawNet.includes("FUTURE")) {
              setNetwork("FUTURENET");
            } else {
              setNetwork("TESTNET");
            }
            if (netDetails.networkPassphrase) {
              setNetworkPassphrase(netDetails.networkPassphrase);
            }
          } else {
            const rawNetwork = await getFreighterNetwork();
            const netStr = (
              typeof rawNetwork === "string"
                ? rawNetwork
                : (rawNetwork as { network?: string })?.network || ""
            ).toUpperCase();

            if (netStr.includes("PUBLIC") || netStr.includes("MAIN")) {
              setNetwork("PUBLIC");
            } else {
              setNetwork("TESTNET");
            }
          }
        } catch {
          setNetwork("TESTNET");
        }

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
      }
    } catch {
      setIsFreighterInstalled(false);
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

  const switchProfile = useCallback((role: UserRole) => {
    setActiveRole(role);
  }, []);

  // Compute active profile (combines connected Freighter wallet or deterministic role)
  const activeProfile = useMemo<UserProfile>(() => {
    const baseProfile =
      DEMO_PROFILES.find((p) => p.role === activeRole) || DEMO_PROFILES[0];

    if (isConnected && address) {
      return {
        ...baseProfile,
        address,
        name: `${baseProfile.name} (Freighter)`,
        isExternalWallet: true,
      };
    }

    return baseProfile;
  }, [activeRole, isConnected, address]);

  // Guard: explicitly block signing on Mainnet
  const isMainnetBlocked = network === "PUBLIC";

  const signTransactionGuard = useCallback(
    async (
      xdr: string
    ): Promise<{ success: boolean; error?: string; signedXdr?: string }> => {
      // Re-verify network immediately before signing
      if (network === "PUBLIC") {
        return {
          success: false,
          error:
            "ACCION BLOQUEADA POR SEGURIDAD: La wallet está conectada a PUBLIC (Mainnet). Por especificación P0, está estrictamente prohibido firmar en mainnet. Cambia la red a Testnet en Freighter.",
        };
      }

      if (!isConnected || !address) {
        return {
          success: false,
          error: "Conecta tu wallet Freighter para firmar en Testnet.",
        };
      }

      try {
        const signedRes = await signFreighterTransaction(xdr, {
          networkPassphrase,
        });
        const signedXdr =
          typeof signedRes === "string"
            ? signedRes
            : (signedRes as { signedTxXdr?: string })?.signedTxXdr;

        return {
          success: true,
          signedXdr: signedXdr || xdr,
        };
      } catch (err: unknown) {
        return {
          success: false,
          error:
            err instanceof Error ? err.message : "Firma rechazada por el usuario en Freighter.",
        };
      }
    },
    [network, isConnected, address, networkPassphrase]
  );

  const value = useMemo<WalletState>(
    () => ({
      isFreighterInstalled,
      isConnected,
      isConnecting,
      address,
      network,
      networkPassphrase,
      isMainnetBlocked,
      activeProfile,
      availableProfiles: DEMO_PROFILES,
      connectFreighter,
      disconnectFreighter,
      switchProfile,
      signTransactionGuard,
    }),
    [
      isFreighterInstalled,
      isConnected,
      isConnecting,
      address,
      network,
      networkPassphrase,
      isMainnetBlocked,
      activeProfile,
      connectFreighter,
      disconnectFreighter,
      switchProfile,
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
