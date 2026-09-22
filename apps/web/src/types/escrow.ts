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

export type FallbackOutcome = "RELEASE" | "REFUND" | "SPLIT";

export interface EscrowDetailsData {
  operationId: string;
  contractId?: string;
  status: EscrowStatus;
  amount: string; // Token amount in minimal units or formatted string, never float
  asset: string; // e.g. "CPUSD"
  parties: EscrowParties;
  activeDeadline?: EscrowDeadline;
  hashes: EscrowHashes;
  fallbackOutcome?: FallbackOutcome;
  fallbackSplitBps?: number;
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
  viewerRole?: UserRole | null;
  canFinalize?: boolean;
  onCreateEscrow?: () => void;
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
  labelKey?: string;
  descKey?: string;
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
  canFinalize: boolean = false,
  t?: (key: string, params?: Record<string, string | number>) => string
): EscrowAction[] {
  if (isTerminalStatus(status)) return [];

  const tr = (key: string, fallback: string) => {
    if (!t) return fallback;
    const res = t(key);
    return res && res !== key ? res : fallback;
  };

  const actions: EscrowAction[] = [];

  if (role === "buyer") {
    if (status === "CREATED") {
      actions.push({
        id: "create",
        labelKey: "actions.create",
        descKey: "actions.create_desc",
        label: tr("actions.create", "Crear Escrow"),
        fullName: "Crear Nuevo Escrow Comercial",
        variant: "secondary",
        expectedOutcome: "CREATED",
        description: tr("actions.create_desc", "Configura e inicializa un nuevo contrato de custodia comercial."),
      });
      actions.push({
        id: "fund",
        labelKey: "actions.fund",
        descKey: "actions.fund_desc",
        label: tr("actions.fund", "Fondear Depósito"),
        fullName: "Fondear Depósito en Custodia",
        variant: "primary",
        expectedOutcome: "FUNDED",
        description: tr("actions.fund_desc", "Transfiere los fondos de compra al contrato inteligente."),
      });
      actions.push({
        id: "cancel",
        labelKey: "actions.cancel",
        descKey: "actions.cancel_desc",
        label: tr("actions.cancel", "Cancelar Escrow"),
        fullName: "Cancelar Depósito Previo a Fondeo",
        variant: "danger",
        expectedOutcome: "CANCELLED",
        description: tr("actions.cancel_desc", "Cancela la operación antes de que los fondos sean bloqueados."),
      });
    } else if (status === "ATTESTED_PASS") {
      actions.push({
        id: "approve",
        labelKey: "actions.approve",
        descKey: "actions.approve_desc",
        label: tr("actions.approve", "Aprobar Liberación"),
        fullName: "Aprobar Liberación Definitiva",
        variant: "primary",
        expectedOutcome: "RELEASED",
        description: tr("actions.approve_desc", "Acepta el resultado satisfactorio del motor y libera los fondos al proveedor."),
      });
      actions.push({
        id: "dispute_pass",
        labelKey: "actions.dispute",
        descKey: "actions.dispute_desc",
        label: tr("actions.dispute", "Disputar Atestación"),
        fullName: "Objetar y Abrir Disputa",
        variant: "danger",
        expectedOutcome: "DISPUTED",
        description: tr("actions.dispute_desc", "Objeta la atestación y eleva la operación al árbitro (resolver)."),
      });
    }
  } else if (role === "supplier") {
    if (status === "FUNDED") {
      actions.push({
        id: "submit_evidence",
        labelKey: "actions.submit_evidence",
        descKey: "actions.submit_evidence_desc",
        label: tr("actions.submit_evidence", "Presentar Evidencia"),
        fullName: "Presentar Lote Documental (Evidence)",
        variant: "primary",
        expectedOutcome: "EVIDENCE_SUBMITTED",
        description: tr("actions.submit_evidence_desc", "Registra el hash del lote documental de entrega para evaluación del motor."),
      });
    } else if (status === "ATTESTED_FAIL") {
      actions.push({
        id: "submit_correction",
        labelKey: "actions.correct_evidence",
        descKey: "actions.correct_evidence_desc",
        label: tr("actions.correct_evidence", "Presentar Corrección"),
        fullName: "Presentar Corrección Técnica",
        variant: "primary",
        expectedOutcome: "EVIDENCE_SUBMITTED",
        description: tr("actions.correct_evidence_desc", "Envía un nuevo lote documental corregido (1 intento permitido)."),
      });
      actions.push({
        id: "dispute_fail",
        labelKey: "actions.dispute",
        descKey: "actions.dispute_desc",
        label: tr("actions.dispute", "Disputar Dictamen"),
        fullName: "Elevar Disputa al Árbitro",
        variant: "danger",
        expectedOutcome: "DISPUTED",
        description: tr("actions.dispute_desc", "Objeta la observación del motor y solicita arbitraje neutral."),
      });
    }
  } else if (role === "resolver") {
    if (status === "DISPUTED") {
      actions.push({
        id: "resolve_release",
        labelKey: "actions.release",
        descKey: "actions.release_desc",
        label: tr("actions.release", "Liberar a Proveedor"),
        fullName: "Dictamen Arbitral: Liberación (Release)",
        variant: "primary",
        expectedOutcome: "RELEASED",
        description: tr("actions.release_desc", "Falla a favor del proveedor, liberando la totalidad de los fondos."),
      });
      actions.push({
        id: "resolve_refund",
        labelKey: "actions.refund",
        descKey: "actions.refund_desc",
        label: tr("actions.refund", "Reembolsar a Comprador"),
        fullName: "Dictamen Arbitral: Reembolso (Refund)",
        variant: "danger",
        expectedOutcome: "REFUNDED",
        description: tr("actions.refund_desc", "Falla a favor del comprador, devolviendo los fondos en custodia."),
      });
      actions.push({
        id: "resolve_split",
        labelKey: "actions.split",
        descKey: "actions.split_desc",
        label: tr("actions.split", "Dividir Fondos (Split)"),
        fullName: "Dictamen Arbitral: División Equitativa (Split)",
        variant: "secondary",
        expectedOutcome: "SPLIT",
        description: tr("actions.split_desc", "Resuelve una liquidación porcentual según los acuerdos comerciales."),
      });
    }
  }

  // Permissionless finalize: available to ANY account (including observer)
  // ONLY when the contract ledger indicates the deadline passed.
  if (canFinalize) {
    actions.push({
      id: "finalize",
      labelKey: "actions.finalize",
      descKey: "actions.finalize_desc",
      label: tr("actions.finalize", "Ejecutar Finalize"),
      fullName: "Ejecución Vencimiento (Finalize)",
      variant: "secondary",
      expectedOutcome: "RELEASED",
      description: tr("actions.finalize_desc", "Acción permisionada por expiración del plazo contractual en ledger."),
    });
  }

  return actions;
}


