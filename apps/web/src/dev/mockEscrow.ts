/**
 * ============================================================================
 * CANGUPAY — DEVELOPMENT MOCK DATA (VISUAL PREVIEW ONLY)
 * ============================================================================
 *
 * NOTE: These mock fixtures are strictly for local development and UI testing
 * of the P0 state machine stages. They do NOT represent real on-chain data
 * and must not be used as authoritative contract state.
 *
 * Production builds will read state directly from Stellar RPC once PR #18
 * and Soroban RPC getters/bindings are stabilized.
 * ============================================================================
 */

import { EscrowDetailsData, EscrowStatus } from "@/types/escrow";

// Deterministic reference timestamp to prevent hydration discrepancies in SSR
const BASE_LEDGER_TIME = 1758412800;

export const mockEscrows: Record<EscrowStatus, EscrowDetailsData> = {
  CREATED: {
    operationId: "CANGU-OP-2026-000",
    contractId: undefined, // Contract ID placeholder until deployed
    status: "CREATED",
    amount: "10,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z",
      supplier: "GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L",
      engine: "GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M",
      resolver: "GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R",
    },
    activeDeadline: {
      type: "action",
      label: "Fondeo Requerido por el Comprador",
      timestamp: BASE_LEDGER_TIME + 86400,
    },
    hashes: {},
  },
  FUNDED: {
    operationId: "CANGU-OP-2026-001",
    contractId: undefined,
    status: "FUNDED",
    amount: "15,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z",
      supplier: "GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L",
      engine: "GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M",
      resolver: "GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R",
    },
    activeDeadline: {
      type: "submission",
      label: "Envío de Evidencia Documental (Proveedor)",
      timestamp: BASE_LEDGER_TIME + 14400,
    },
    hashes: {},
  },
  EVIDENCE_SUBMITTED: {
    operationId: "CANGU-OP-2026-002",
    contractId: undefined,
    status: "EVIDENCE_SUBMITTED",
    amount: "15,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z",
      supplier: "GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L",
      engine: "GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M",
      resolver: "GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R",
    },
    activeDeadline: {
      type: "attestation",
      label: "Atestación del Motor (Engine)",
      timestamp: BASE_LEDGER_TIME + 7200,
    },
    hashes: {
      evidenceBundleHash: "0x7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b",
    },
  },
  ATTESTED_PASS: {
    operationId: "CANGU-OP-2026-003",
    contractId: undefined,
    status: "ATTESTED_PASS",
    amount: "25,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z",
      supplier: "GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L",
      engine: "GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M",
      resolver: "GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R",
    },
    activeDeadline: {
      type: "action",
      label: "Ventana de Objeción del Comprador",
      timestamp: BASE_LEDGER_TIME + 7200,
    },
    hashes: {
      evidenceBundleHash: "0x3f2a1b0c9d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4e3f2a",
      reportHash: "0x9876543210abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    },
  },
  ATTESTED_FAIL: {
    operationId: "CANGU-OP-2026-004",
    contractId: undefined,
    status: "ATTESTED_FAIL",
    amount: "18,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z",
      supplier: "GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L",
      engine: "GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M",
      resolver: "GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R",
    },
    activeDeadline: {
      type: "action",
      label: "Ventana de Corrección del Proveedor (1 intento)",
      timestamp: BASE_LEDGER_TIME + 7200,
    },
    hashes: {
      evidenceBundleHash: "0x5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b",
      reportHash: "0x1234567890abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    },
  },
  DISPUTED: {
    operationId: "CANGU-OP-2026-005",
    contractId: undefined,
    status: "DISPUTED",
    amount: "8,500.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z",
      supplier: "GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L",
      engine: "GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M",
      resolver: "GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R",
    },
    activeDeadline: {
      type: "resolution",
      label: "Resolución del Árbitro Neutral (Resolver)",
      timestamp: BASE_LEDGER_TIME + 28800,
    },
    hashes: {
      evidenceBundleHash: "0x11223344556677889900aabbccddeeff0011223344556677889900aabbccddee",
      reportHash: "0xaabbccddeeff0011223344556677889900aabbccddeeff001122334455667788",
      reasonHash: "0xccddeeff0011223344556677889900aabbccddeeff0011223344556677889900",
      disputeEvidenceHash: "0xeeff0011223344556677889900aabbccddeeff0011223344556677889900aabb",
    },
  },
  RELEASED: {
    operationId: "CANGU-OP-2026-006",
    contractId: undefined,
    status: "RELEASED",
    amount: "12,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z",
      supplier: "GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L",
      engine: "GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M",
      resolver: "GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R",
    },
    hashes: {
      evidenceBundleHash: "0x44556677889900aabbccddeeff0011223344556677889900aabbccddeeff0011",
      reportHash: "0x223344556677889900aabbccddeeff0011223344556677889900aabbccddeeff",
    },
  },
  REFUNDED: {
    operationId: "CANGU-OP-2026-007",
    contractId: undefined,
    status: "REFUNDED",
    amount: "10,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z",
      supplier: "GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L",
      engine: "GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M",
      resolver: "GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R",
    },
    hashes: {},
  },
  SPLIT: {
    operationId: "CANGU-OP-2026-008",
    contractId: undefined,
    status: "SPLIT",
    amount: "20,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z",
      supplier: "GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L",
      engine: "GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M",
      resolver: "GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R",
    },
    hashes: {},
  },
  CANCELLED: {
    operationId: "CANGU-OP-2026-009",
    contractId: undefined,
    status: "CANCELLED",
    amount: "10,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z",
      supplier: "GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L",
      engine: "GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M",
      resolver: "GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R",
    },
    hashes: {},
  },
};
