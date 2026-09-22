/**
 * CanguPay — Escrow Domain Types (Frontend UI Model)
 *
 * Represents the documented P0 escrow state machine (docs/state-machine.md).
 * Decoupled from chain bindings and RPC layers.
 */

export type EscrowStatus =
  | "CREATED"
  | "FUNDED"
  | "EVIDENCE_SUBMITTED"
  | "ATTESTED_PASS"
  | "ATTESTED_FAIL"
  | "DISPUTED"
  | "CANCELLED"
  | "RELEASED"
  | "REFUNDED"
  | "SPLIT";

export type EscrowActor = "buyer" | "supplier" | "engine" | "resolver" | "none";

export interface EscrowParties {
  buyer: string;
  supplier: string;
  resolver: string;
  engine: string;
}

export interface EscrowDeadline {
  type: "submission" | "attestation" | "action" | "resolution";
  label: string;
  timestamp: number; // Unix timestamp in seconds (ledger timestamp)
}

export interface EscrowHashes {
  evidenceBundleHash?: string;
  reportHash?: string;
  reasonHash?: string;
  disputeEvidenceHash?: string;
}

export interface EscrowDetailsData {
  operationId: string;
  contractId?: string;
  status: EscrowStatus;
  amount: string; // Token amount in minimal units or formatted string, never float
  asset: string; // e.g. "CPUSD"
  parties: EscrowParties;
  activeDeadline?: EscrowDeadline;
  hashes: EscrowHashes;
  transactionHash?: string;
  explorerBaseUrl?: string; // Default: https://stellar.expert/explorer/testnet
  updatedAtLedger?: number;
}

export interface EscrowDetailsProps {
  data?: EscrowDetailsData | null;
  isLoading?: boolean;
  error?: string | null;
  onRefresh?: () => void;
  actionSlot?: React.ReactNode;
  bannerSlot?: React.ReactNode;
}

/**
 * Derives the active actor who must take the next step based on contract status.
 * Visual hint only for UX — Soroban contract remains the ultimate authority.
 */
export function getActiveActor(status: EscrowStatus): EscrowActor {
  switch (status) {
    case "CREATED":
      return "buyer";
    case "FUNDED":
      return "supplier";
    case "EVIDENCE_SUBMITTED":
      return "engine";
    case "ATTESTED_PASS":
      return "buyer";
    case "ATTESTED_FAIL":
      return "supplier";
    case "DISPUTED":
      return "resolver";
    case "CANCELLED":
    case "RELEASED":
    case "REFUNDED":
    case "SPLIT":
    default:
      return "none";
  }
}

export function isTerminalStatus(status: EscrowStatus): boolean {
  return (
    status === "CANCELLED" ||
    status === "RELEASED" ||
    status === "REFUNDED" ||
    status === "SPLIT"
  );
}
