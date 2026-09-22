/**
 * CanguPay — Escrow Domain Types (Frontend UI Model)
 *
 * Represents the documented P0 escrow state machine (docs/state-machine.md).
 * Decoupled from chain bindings and RPC layers.
 */

import { UserRole } from "./wallet";

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

/**
 * Pure domain function: derives the human role from the connected wallet
 * and the escrow participants configuration.
 *
 * wallet == config.parties.buyer    -> buyer
 * wallet == config.parties.supplier -> supplier
 * wallet == config.parties.resolver -> resolver
 * any other address                 -> observer
 * no wallet connected               -> null
 */
export function deriveWalletRole(
  walletAddress: string | null | undefined,
  parties?: EscrowParties | null
): UserRole | null {
  if (!walletAddress) return null;
  if (!parties) return "observer";

  const addr = walletAddress.trim();
  if (parties.buyer && addr === parties.buyer.trim()) return "buyer";
  if (parties.supplier && addr === parties.supplier.trim()) return "supplier";
  if (parties.resolver && addr === parties.resolver.trim()) return "resolver";

  return "observer";
}

export interface EscrowAction {
  id: string;
  label: string;
  fullName: string;
  variant: "primary" | "secondary" | "danger";
  expectedOutcome: EscrowStatus;
  description: string;
}

/**
 * Pure domain function: defines the exact action matrix matching the P0 architecture.
 *
 * CREATED:
 *  - buyer: fund, cancel
 * FUNDED:
 *  - supplier: submit evidence (NO buyer release from FUNDED)
 * EVIDENCE_SUBMITTED:
 *  - engine: attests autonomously (no human action)
 * ATTESTED_PASS:
 *  - buyer: approve (RELEASED), dispute (DISPUTED)
 * ATTESTED_FAIL:
 *  - supplier: correct (EVIDENCE_SUBMITTED), dispute (DISPUTED)
 * DISPUTED:
 *  - resolver: release (RELEASED), refund (REFUNDED), split (SPLIT)
 * Finalize:
 *  - any account when contract deadline reached (canFinalize === true)
 */
export function getAvailableActions(
  role: UserRole | null,
  status: EscrowStatus,
  canFinalize: boolean = false
): EscrowAction[] {
  if (isTerminalStatus(status)) return [];

  const actions: EscrowAction[] = [];

  if (role === "buyer") {
    if (status === "CREATED") {
      actions.push({
        id: "fund",
        label: "Fondear Depósito",
        fullName: "Fondear Depósito en Custodia",
        variant: "primary",
        expectedOutcome: "FUNDED",
        description: "Transfiere los fondos de compra al contrato inteligente.",
      });
      actions.push({
        id: "cancel",
        label: "Cancelar Escrow",
        fullName: "Cancelar Depósito Previo a Fondeo",
        variant: "danger",
        expectedOutcome: "CANCELLED",
        description: "Cancela la operación antes de que los fondos sean bloqueados.",
      });
    } else if (status === "ATTESTED_PASS") {
      actions.push({
        id: "approve",
        label: "Aprobar Liberación",
        fullName: "Aprobar Liberación Definitiva",
        variant: "primary",
        expectedOutcome: "RELEASED",
        description: "Acepta el resultado satisfactorio del motor y libera los fondos al proveedor.",
      });
      actions.push({
        id: "dispute_pass",
        label: "Disputar Atestación",
        fullName: "Objetar y Abrir Disputa",
        variant: "danger",
        expectedOutcome: "DISPUTED",
        description: "Objeta la atestación y eleva la operación al árbitro (resolver).",
      });
    }
  } else if (role === "supplier") {
    if (status === "FUNDED") {
      actions.push({
        id: "submit_evidence",
        label: "Presentar Evidencia",
        fullName: "Presentar Lote Documental (Evidence)",
        variant: "primary",
        expectedOutcome: "EVIDENCE_SUBMITTED",
        description: "Registra el hash del lote documental de entrega para evaluación del motor.",
      });
    } else if (status === "ATTESTED_FAIL") {
      actions.push({
        id: "submit_correction",
        label: "Presentar Corrección",
        fullName: "Presentar Corrección Técnica",
        variant: "primary",
        expectedOutcome: "EVIDENCE_SUBMITTED",
        description: "Envía un nuevo lote documental corregido (1 intento permitido).",
      });
      actions.push({
        id: "dispute_fail",
        label: "Disputar Dictamen",
        fullName: "Elevar Disputa al Árbitro",
        variant: "danger",
        expectedOutcome: "DISPUTED",
        description: "Objeta la observación del motor y solicita arbitraje neutral.",
      });
    }
  } else if (role === "resolver") {
    if (status === "DISPUTED") {
      actions.push({
        id: "resolve_release",
        label: "Liberar a Proveedor",
        fullName: "Dictamen Arbitral: Liberación (Release)",
        variant: "primary",
        expectedOutcome: "RELEASED",
        description: "Falla a favor del proveedor, liberando la totalidad de los fondos.",
      });
      actions.push({
        id: "resolve_refund",
        label: "Reembolsar a Comprador",
        fullName: "Dictamen Arbitral: Reembolso (Refund)",
        variant: "danger",
        expectedOutcome: "REFUNDED",
        description: "Falla a favor del comprador, devolviendo los fondos en custodia.",
      });
      actions.push({
        id: "resolve_split",
        label: "Dividir Fondos (Split)",
        fullName: "Dictamen Arbitral: División Equitativa (Split)",
        variant: "secondary",
        expectedOutcome: "SPLIT",
        description: "Resuelve una liquidación porcentual según los acuerdos comerciales.",
      });
    }
  }

  // Permissionless finalize: available to ANY account (including observer)
  // ONLY when the contract ledger indicates the deadline passed.
  if (canFinalize) {
    actions.push({
      id: "finalize",
      label: "Ejecutar Finalize",
      fullName: "Ejecución Vencimiento (Finalize)",
      variant: "secondary",
      expectedOutcome: "RELEASED",
      description: "Acción permisionada por expiración del plazo contractual en ledger.",
    });
  }

  return actions;
}


