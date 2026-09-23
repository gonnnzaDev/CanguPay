type NetworkDetails<TNetwork extends string> = { network: TNetwork; passphrase: string | null };
type SigningResult = { success: boolean; error?: string; signedXdr?: string };

type SigningGuardOptions<TNetwork extends string> = {
  xdr: string;
  connectedAddress: string | null;
  isConnected: boolean;
  expectedPassphrase: string;
  queryNetwork: () => Promise<NetworkDetails<TNetwork>>;
  onNetworkChecked: (network: TNetwork, passphrase: string | null) => void;
  signTransaction: (
    xdr: string,
    options: { networkPassphrase: string; address: string }
  ) => Promise<unknown>;
};

export async function signWithFreighterGuard<TNetwork extends string>({
  xdr,
  connectedAddress,
  isConnected,
  expectedPassphrase,
  queryNetwork,
  onNetworkChecked,
  signTransaction,
}: SigningGuardOptions<TNetwork>): Promise<SigningResult> {
  if (typeof xdr !== "string" || !xdr.trim()) {
    return { success: false, error: "Signature rejected: transaction XDR is missing." };
  }

  let freshNetwork: NetworkDetails<TNetwork>;
  try {
    freshNetwork = await queryNetwork();
    onNetworkChecked(freshNetwork.network, freshNetwork.passphrase);
  } catch {
    return {
      success: false,
      error: "ACCION BLOQUEADA: No se pudo verificar la red en Freighter. Estado fail-closed.",
    };
  }

  if (freshNetwork.network !== "TESTNET" || freshNetwork.passphrase !== expectedPassphrase) {
    return {
      success: false,
      error: `ACCION BLOQUEADA POR SEGURIDAD: La red (${freshNetwork.network}) no coincide con la passphrase oficial de Stellar Testnet ("${expectedPassphrase}"). Por especificación P0, toda firma fuera de Testnet está estrictamente bloqueada.`,
    };
  }

  if (!isConnected || !connectedAddress?.trim()) {
    return { success: false, error: "Conecta tu wallet Freighter para firmar en Testnet." };
  }

  try {
    const response = await signTransaction(xdr, {
      networkPassphrase: expectedPassphrase,
      address: connectedAddress,
    });

    if (!response || typeof response !== "object") {
      return { success: false, error: "Signature rejected: invalid Freighter response." };
    }

    const { signedTxXdr, signerAddress, error } = response as {
      signedTxXdr?: unknown;
      signerAddress?: unknown;
      error?: unknown;
    };

    if (error) {
      let message = "Signature rejected by Freighter.";
      if (typeof error === "string" && error) {
        message = error;
      } else if (typeof error === "object" && error !== null && "message" in error &&
        typeof error.message === "string" && error.message) {
        message = error.message;
      }
      return { success: false, error: message };
    }

    if (typeof signedTxXdr !== "string" || !signedTxXdr.trim()) {
      return { success: false, error: "Firma vacía: Freighter no devolvió signedTxXdr." };
    }

    if (typeof signerAddress !== "string" || !signerAddress) {
      return { success: false, error: "Signature rejected: Freighter did not return signerAddress." };
    }

    if (signerAddress !== connectedAddress) {
      return {
        success: false,
        error: `Firma rechazada: signerAddress (${signerAddress}) no coincide con la wallet conectada (${connectedAddress}).`,
      };
    }

    return { success: true, signedXdr: signedTxXdr };
  } catch (err: unknown) {
    return {
      success: false,
      error: err instanceof Error ? err.message : "Firma rechazada por el usuario en Freighter.",
    };
  }
}
