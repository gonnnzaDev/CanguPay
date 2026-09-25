//! Pruebas de aceptación P0-03.
//!
//! Cubren el camino feliz `CREATED → FUNDED → EVIDENCE_SUBMITTED →
//! ATTESTED_PASS → RELEASED`, el de fallo (`ATTESTED_FAIL` retiene fondos),
//! `CREATED → CANCELLED`, más las reglas de negocio: fondeo por pull atómico,
//! deadline de evidencia, exclusividad de roles con firmante equivocado,
//! doble aprobación y vencimientos `finalize()` (ghost supplier/engine).
//! Los envíos directos al contrato no forman parte del escrow y no tienen
//! vía de recuperación en P0 (limitación conocida).

use super::*;
extern crate std;

use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _, MockAuth, MockAuthInvoke};
use soroban_sdk::Event as _;
use soroban_sdk::{token::StellarAssetClient, xdr, Address, BytesN, IntoVal, MuxedAddress};

const AMOUNT: i128 = 10_000_000_000;
const STRAY: i128 = 2_000_000_000;
const SUBMISSION_PERIOD: u64 = 1000;
const ATTESTATION_PERIOD: u64 = 1000;
const OBJECTION_PERIOD: u64 = 1000;
const CORRECTION_PERIOD: u64 = 1000;
const RESOLUTION_PERIOD: u64 = 1000;
const FUNDED_AT: u64 = 0;

fn evidence_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[7u8; 32])
}

fn report_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[9u8; 32])
}

fn must_panic(closure: impl FnOnce()) {
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(closure)).is_err());
}

struct TestContext {
    env: Env,
    contract: Address,
    token: Address,
    token_admin: Address,
    buyer: Address,
    supplier: Address,
    engine: Address,
    resolver: Address,
}

fn setup(mock_all_auths: bool) -> TestContext {
    let env = Env::default();
    if mock_all_auths {
        env.mock_all_auths();
    }
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let buyer = Address::generate(&env);
    let supplier = Address::generate(&env);
    let engine = Address::generate(&env);
    let resolver = Address::generate(&env);
    let contract = env.register(ConditionalPayment, ());
    TestContext {
        env,
        contract,
        token,
        token_admin,
        buyer,
        supplier,
        engine,
        resolver,
    }
}

/// Sub-invocación mockeada de `transfer` del SAC, para autorizar con mocks
/// estrictos los pulls/pagos que el contrato hace en nombre propio.
fn transfer_sub<'a>(
    token: &'a Address,
    from: Address,
    to: Address,
    amount: i128,
    env: &Env,
) -> MockAuthInvoke<'a> {
    MockAuthInvoke {
        contract: token,
        fn_name: "transfer",
        args: (from, MuxedAddress::from(to), amount).into_val(env),
        sub_invokes: &[],
    }
}

impl TestContext {
    fn client(&self) -> ConditionalPaymentClient<'_> {
        ConditionalPaymentClient::new(&self.env, &self.contract)
    }

    fn sac(&self) -> StellarAssetClient<'_> {
        StellarAssetClient::new(&self.env, &self.token)
    }

    fn config(&self) -> EscrowConfig {
        EscrowConfig {
            buyer: self.buyer.clone(),
            supplier: self.supplier.clone(),
            engine: self.engine.clone(),
            resolver: self.resolver.clone(),
            token: self.token.clone(),
            amount: AMOUNT,
            submission_period: SUBMISSION_PERIOD,
            attestation_period: ATTESTATION_PERIOD,
            objection_period: OBJECTION_PERIOD,
            correction_period: CORRECTION_PERIOD,
            resolution_period: RESOLUTION_PERIOD,
            fallback_outcome: FallbackOutcome::Refund,
            fallback_split_bps: 0,
            max_correction_attempts: 1,
        }
    }

    fn initialize(&self) {
        self.client().initialize(&self.config());
    }

    /// Mintea al buyer y fondea con el pull atómico de `fund()`.
    fn mint_and_fund(&self) {
        self.sac().mint(&self.buyer, &AMOUNT);
        self.client().fund();
    }

    fn to_attested_pass(&self) {
        // Helper happy-path con mock_all_auths; auth por rol se verifica
        // exhaustivamente en los tests de auth estrictos (setup(false) + MockAuth).
        self.initialize();
        self.mint_and_fund();
        self.client().submit_evidence(&evidence_hash(&self.env));
        self.client()
            .attest(&AttestationOutcome::Pass, &report_hash(&self.env));
    }

    fn own_events(&self) -> std::vec::Vec<xdr::ContractEvent> {
        self.env
            .events()
            .all()
            .filter_by_contract(&self.contract)
            .events()
            .to_vec()
    }

    fn assert_single_event(&self, expected: xdr::ContractEvent) {
        let events = self.own_events();
        assert_eq!(events.len(), 1, "se esperaba exactamente un evento");
        assert_eq!(events[0], expected);
    }

    fn token_balance(&self, address: &Address) -> i128 {
        TokenClient::new(&self.env, &self.token).balance(address)
    }
}

#[test]
fn happy_path_pass_releases_funds() {
    let ctx = setup(true);

    ctx.initialize();
    ctx.assert_single_event(
        EscrowCreated {
            buyer: ctx.buyer.clone(),
            supplier: ctx.supplier.clone(),
            engine: ctx.engine.clone(),
            resolver: ctx.resolver.clone(),
            token: ctx.token.clone(),
            amount: AMOUNT,
            submission_period: SUBMISSION_PERIOD,
            attestation_period: ATTESTATION_PERIOD,
            objection_period: OBJECTION_PERIOD,
            correction_period: CORRECTION_PERIOD,
            resolution_period: RESOLUTION_PERIOD,
            fallback_outcome: FallbackOutcome::Refund,
            fallback_split_bps: 0,
            max_correction_attempts: 1,
        }
        .to_xdr(&ctx.env, &ctx.contract),
    );
    assert_eq!(ctx.client().state(), EscrowState::Created);
    assert_eq!(ctx.client().config(), ctx.config());

    ctx.sac().mint(&ctx.buyer, &AMOUNT);
    ctx.client().fund();
    ctx.assert_single_event(
        Funded {
            funded_at: FUNDED_AT,
            submission_deadline: FUNDED_AT + SUBMISSION_PERIOD,
        }
        .to_xdr(&ctx.env, &ctx.contract),
    );
    assert_eq!(ctx.client().state(), EscrowState::Funded);
    assert_eq!(
        ctx.token_balance(&ctx.contract),
        AMOUNT,
        "el pull movió amount"
    );

    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.assert_single_event(
        EvidenceSubmitted {
            attempt: 0,
            evidence_bundle_hash: evidence_hash(&ctx.env),
        }
        .to_xdr(&ctx.env, &ctx.contract),
    );
    assert_eq!(ctx.client().state(), EscrowState::EvidenceSubmitted);

    ctx.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env));
    ctx.assert_single_event(
        Attested {
            outcome: AttestationOutcome::Pass,
            report_hash: report_hash(&ctx.env),
        }
        .to_xdr(&ctx.env, &ctx.contract),
    );
    assert_eq!(ctx.client().state(), EscrowState::AttestedPass);

    ctx.client().approve();
    ctx.assert_single_event(
        Approved {
            to: ctx.supplier.clone(),
            amount: AMOUNT,
        }
        .to_xdr(&ctx.env, &ctx.contract),
    );
    assert_eq!(ctx.client().state(), EscrowState::Released);
    assert_eq!(
        ctx.token_balance(&ctx.supplier),
        AMOUNT,
        "el supplier recibe exactamente amount"
    );
    assert_eq!(
        ctx.token_balance(&ctx.contract),
        0,
        "el contrato queda a cero"
    );
}

#[test]
fn happy_path_fail_holds_funds() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.client()
        .attest(&AttestationOutcome::Fail, &report_hash(&ctx.env));

    assert_eq!(ctx.client().state(), EscrowState::AttestedFail);
    assert_eq!(ctx.token_balance(&ctx.supplier), 0, "no se mueven fondos");
    assert_eq!(
        ctx.token_balance(&ctx.contract),
        AMOUNT,
        "el contrato retiene los fondos"
    );
}

#[test]
fn cancel_before_fund() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.client().cancel();
    ctx.assert_single_event(
        Cancelled {
            by: ctx.buyer.clone(),
        }
        .to_xdr(&ctx.env, &ctx.contract),
    );
    assert_eq!(ctx.client().state(), EscrowState::Cancelled);
}

#[test]
fn cancel_after_fund_is_rejected() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();

    must_panic(|| ctx.client().cancel());
    assert_eq!(ctx.client().state(), EscrowState::Funded);
}

#[test]
fn double_initialize_is_rejected() {
    let ctx = setup(true);
    ctx.initialize();
    must_panic(|| ctx.initialize());
}

#[test]
fn initialization_requires_positive_amount() {
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.amount = 0;
    must_panic(|| ctx.client().initialize(&bad));
}

#[test]
fn initialization_requires_positive_period() {
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.submission_period = 0;
    must_panic(|| ctx.client().initialize(&bad));
}

#[test]
fn fund_without_transfer_is_atomic() {
    let ctx = setup(true);
    ctx.initialize();

    must_panic(|| ctx.client().fund());
    assert_eq!(
        ctx.client().state(),
        EscrowState::Created,
        "no cambia de estado"
    );
    assert_eq!(ctx.token_balance(&ctx.contract), 0, "nada se movió");
    assert_eq!(ctx.token_balance(&ctx.buyer), 0);
    assert_eq!(ctx.own_events().len(), 0, "no emite el evento Funded");
}

#[test]
fn fund_with_incorrect_balance_is_atomic() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.sac().mint(&ctx.buyer, &(AMOUNT / 2));

    must_panic(|| ctx.client().fund());
    assert_eq!(
        ctx.client().state(),
        EscrowState::Created,
        "no cambia de estado"
    );
    assert_eq!(
        ctx.token_balance(&ctx.contract),
        0,
        "el pull fallido no deja fondeo parcial"
    );
    assert_eq!(
        ctx.token_balance(&ctx.buyer),
        AMOUNT / 2,
        "el buyer conserva su saldo"
    );
    assert_eq!(ctx.own_events().len(), 0, "no emite el evento Funded");
}

#[test]
fn fund_failure_is_recoverable() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.sac().mint(&ctx.buyer, &(AMOUNT / 2));

    must_panic(|| ctx.client().fund());
    assert_eq!(ctx.client().state(), EscrowState::Created);

    // La operación se recupera: el buyer completa el saldo y fondea.
    ctx.sac().mint(&ctx.buyer, &(AMOUNT - AMOUNT / 2));
    ctx.client().fund();
    ctx.assert_single_event(
        Funded {
            funded_at: FUNDED_AT,
            submission_deadline: FUNDED_AT + SUBMISSION_PERIOD,
        }
        .to_xdr(&ctx.env, &ctx.contract),
    );
    assert_eq!(ctx.client().state(), EscrowState::Funded);
    assert_eq!(ctx.token_balance(&ctx.contract), AMOUNT);
}

#[test]
fn fund_after_cancel_is_rejected() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.client().cancel();

    must_panic(|| ctx.client().fund());
    assert_eq!(ctx.client().state(), EscrowState::Cancelled);
}

#[test]
fn direct_sends_after_funding_are_out_of_settlement() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();

    // Envío directo post-fondeo: queda fuera del settlement (limitación conocida).
    ctx.sac().mint(&ctx.buyer, &STRAY);
    TokenClient::new(&ctx.env, &ctx.token).transfer(&ctx.buyer, &ctx.contract, &STRAY);

    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env));
    ctx.client().approve();

    assert_eq!(ctx.client().state(), EscrowState::Released);
    assert_eq!(
        ctx.token_balance(&ctx.supplier),
        AMOUNT,
        "el settlement paga exactamente amount"
    );
    assert_eq!(
        ctx.token_balance(&ctx.contract),
        STRAY,
        "el envío directo queda fuera del settlement"
    );
}

#[test]
fn submit_evidence_before_fund_is_rejected() {
    let ctx = setup(true);
    ctx.initialize();

    must_panic(|| ctx.client().submit_evidence(&evidence_hash(&ctx.env)));
    assert_eq!(ctx.client().state(), EscrowState::Created);
}

#[test]
fn submit_evidence_after_deadline_is_rejected() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();

    ctx.env
        .ledger()
        .set_timestamp(FUNDED_AT + SUBMISSION_PERIOD);

    must_panic(|| ctx.client().submit_evidence(&evidence_hash(&ctx.env)));
    assert_eq!(ctx.client().state(), EscrowState::Funded);
}

#[test]
fn attest_from_wrong_state_is_rejected() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();

    must_panic(|| {
        ctx.client()
            .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env))
    });
    assert_eq!(ctx.client().state(), EscrowState::Funded);
}

#[test]
fn approve_from_attested_fail_is_rejected_and_retains_funds() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.client()
        .attest(&AttestationOutcome::Fail, &report_hash(&ctx.env));

    must_panic(|| ctx.client().approve());
    assert_eq!(ctx.client().state(), EscrowState::AttestedFail);
    assert_eq!(ctx.token_balance(&ctx.supplier), 0);
    assert_eq!(ctx.token_balance(&ctx.contract), AMOUNT);
}

#[test]
fn double_approve_is_rejected_and_pays_once() {
    let ctx = setup(true);
    ctx.to_attested_pass();

    ctx.client().approve();
    assert_eq!(ctx.client().state(), EscrowState::Released);
    assert_eq!(ctx.token_balance(&ctx.supplier), AMOUNT);

    must_panic(|| ctx.client().approve());
    assert_eq!(ctx.client().state(), EscrowState::Released);
    assert_eq!(
        ctx.token_balance(&ctx.supplier),
        AMOUNT,
        "el pago se hace exactamente una vez"
    );
    assert_eq!(ctx.token_balance(&ctx.contract), 0);
}

#[test]
fn initialize_requires_buyer() {
    let ctx = setup(false);
    let init = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "initialize",
        args: (ctx.config(),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    ctx.env.mock_auths(&[MockAuth {
        address: &ctx.supplier,
        invoke: &init,
    }]);

    // El supplier autoriza, pero el buyer requerido no: falla.
    must_panic(|| ctx.client().initialize(&ctx.config()));
    must_panic(|| {
        ctx.client().state();
    });
}

#[test]
fn cancel_by_supplier_rejected_in_created() {
    let ctx = setup(false);
    let init = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "initialize",
        args: (ctx.config(),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let cancel = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "cancel",
        args: soroban_sdk::Vec::new(&ctx.env),
        sub_invokes: &[],
    };
    ctx.env.mock_auths(&[
        MockAuth {
            address: &ctx.buyer,
            invoke: &init,
        },
        MockAuth {
            address: &ctx.supplier,
            invoke: &cancel,
        },
    ]);

    ctx.initialize();
    must_panic(|| ctx.client().cancel());
    assert_eq!(ctx.client().state(), EscrowState::Created);
}

#[test]
fn fund_by_supplier_rejected_in_created() {
    let ctx = setup(false);
    let init = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "initialize",
        args: (ctx.config(),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let fund = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "fund",
        args: soroban_sdk::Vec::new(&ctx.env),
        sub_invokes: &[],
    };
    ctx.env.mock_auths(&[
        MockAuth {
            address: &ctx.buyer,
            invoke: &init,
        },
        MockAuth {
            address: &ctx.supplier,
            invoke: &fund,
        },
    ]);

    ctx.initialize();
    must_panic(|| ctx.client().fund());
    assert_eq!(ctx.client().state(), EscrowState::Created);
    assert_eq!(ctx.token_balance(&ctx.contract), 0);
}

#[test]
fn submit_by_buyer_rejected_when_funded() {
    let ctx = setup(false);
    let init = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "initialize",
        args: (ctx.config(),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let pull = transfer_sub(
        &ctx.token,
        ctx.buyer.clone(),
        ctx.contract.clone(),
        AMOUNT,
        &ctx.env,
    );
    let fund = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "fund",
        args: soroban_sdk::Vec::new(&ctx.env),
        sub_invokes: &[pull],
    };
    let submit = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "submit_evidence",
        args: (evidence_hash(&ctx.env),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let mint = MockAuthInvoke {
        contract: &ctx.token,
        fn_name: "mint",
        args: (ctx.buyer.clone(), AMOUNT).into_val(&ctx.env),
        sub_invokes: &[],
    };
    ctx.env.mock_auths(&[
        MockAuth {
            address: &ctx.token_admin,
            invoke: &mint,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &init,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &fund,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &submit,
        },
    ]);

    ctx.sac().mint(&ctx.buyer, &AMOUNT);
    ctx.initialize();
    ctx.client().fund();
    // El buyer autoriza, pero en FUNDED solo el supplier presenta evidencia.
    must_panic(|| ctx.client().submit_evidence(&evidence_hash(&ctx.env)));
    assert_eq!(ctx.client().state(), EscrowState::Funded);
}

#[test]
fn attest_by_supplier_rejected_when_evidence_submitted() {
    let ctx = setup(false);
    let init = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "initialize",
        args: (ctx.config(),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let pull = transfer_sub(
        &ctx.token,
        ctx.buyer.clone(),
        ctx.contract.clone(),
        AMOUNT,
        &ctx.env,
    );
    let fund = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "fund",
        args: soroban_sdk::Vec::new(&ctx.env),
        sub_invokes: &[pull],
    };
    let submit = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "submit_evidence",
        args: (evidence_hash(&ctx.env),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let attest = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "attest",
        args: (AttestationOutcome::Pass, report_hash(&ctx.env)).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let mint = MockAuthInvoke {
        contract: &ctx.token,
        fn_name: "mint",
        args: (ctx.buyer.clone(), AMOUNT).into_val(&ctx.env),
        sub_invokes: &[],
    };
    ctx.env.mock_auths(&[
        MockAuth {
            address: &ctx.token_admin,
            invoke: &mint,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &init,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &fund,
        },
        MockAuth {
            address: &ctx.supplier,
            invoke: &submit,
        },
        MockAuth {
            address: &ctx.supplier,
            invoke: &attest,
        },
    ]);

    ctx.sac().mint(&ctx.buyer, &AMOUNT);
    ctx.initialize();
    ctx.client().fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    // El supplier autoriza, pero atestiguar es solo del engine.
    must_panic(|| {
        ctx.client()
            .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env))
    });
    assert_eq!(ctx.client().state(), EscrowState::EvidenceSubmitted);
}

#[test]
fn approve_by_supplier_rejected_when_pass() {
    let ctx = setup(false);
    let init = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "initialize",
        args: (ctx.config(),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let pull = transfer_sub(
        &ctx.token,
        ctx.buyer.clone(),
        ctx.contract.clone(),
        AMOUNT,
        &ctx.env,
    );
    let fund = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "fund",
        args: soroban_sdk::Vec::new(&ctx.env),
        sub_invokes: &[pull],
    };
    let submit = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "submit_evidence",
        args: (evidence_hash(&ctx.env),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let attest = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "attest",
        args: (AttestationOutcome::Pass, report_hash(&ctx.env)).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let approve = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "approve",
        args: soroban_sdk::Vec::new(&ctx.env),
        sub_invokes: &[],
    };
    let mint = MockAuthInvoke {
        contract: &ctx.token,
        fn_name: "mint",
        args: (ctx.buyer.clone(), AMOUNT).into_val(&ctx.env),
        sub_invokes: &[],
    };
    ctx.env.mock_auths(&[
        MockAuth {
            address: &ctx.token_admin,
            invoke: &mint,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &init,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &fund,
        },
        MockAuth {
            address: &ctx.supplier,
            invoke: &submit,
        },
        MockAuth {
            address: &ctx.engine,
            invoke: &attest,
        },
        MockAuth {
            address: &ctx.supplier,
            invoke: &approve,
        },
    ]);

    ctx.sac().mint(&ctx.buyer, &AMOUNT);
    ctx.initialize();
    ctx.client().fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env));
    // El supplier autoriza, pero aprobar es solo del buyer: no se mueve nada.
    must_panic(|| ctx.client().approve());
    assert_eq!(ctx.client().state(), EscrowState::AttestedPass);
    assert_eq!(ctx.token_balance(&ctx.supplier), 0);
    assert_eq!(ctx.token_balance(&ctx.contract), AMOUNT);
}

// ── P0-03: vencimientos finalize() ──

#[test]
fn ghost_supplier_submission_timeout_refunds() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    assert_eq!(ctx.client().state(), EscrowState::Funded);
    let deadline = FUNDED_AT + SUBMISSION_PERIOD;

    // deadline -1: aún no finalizable.
    ctx.env.ledger().set_timestamp(deadline - 1);
    must_panic(|| {
        ctx.client().finalize();
    });
    assert_eq!(ctx.client().state(), EscrowState::Funded);
    assert_eq!(ctx.token_balance(&ctx.contract), AMOUNT);

    // deadline: ghost supplier → refund.
    ctx.env.ledger().set_timestamp(deadline);
    let ret = ctx.client().finalize();
    assert_eq!(ret, EscrowState::Refunded);
    // Verificamos que se emitió Finalized antes de consultar state (state() limpia eventos en este harness).
    let events = ctx.own_events();
    assert!(
        events.iter().any(|e| *e
            == Finalized {
                reason: FinalizeReason::SubmissionTimeout
            }
            .to_xdr(&ctx.env, &ctx.contract)),
        "debe emitir Finalized SubmissionTimeout"
    );
    assert_eq!(ctx.client().state(), EscrowState::Refunded);
    assert_eq!(ctx.token_balance(&ctx.buyer), AMOUNT, "buyer recuperado");
    assert_eq!(ctx.token_balance(&ctx.contract), 0);

    // deadline +1 e idempotencia: segunda llamada no transfiere ni emite nuevo evento.
    let events_before = ctx.own_events().len();
    ctx.env.ledger().set_timestamp(deadline + 1);
    let ret2 = ctx.client().finalize();
    assert_eq!(ret2, EscrowState::Refunded);
    // No nuevo evento: el len no debe crecer (state() limpia, así que comparamos antes de state)
    let events_after = ctx.own_events().len();
    assert_eq!(
        events_after, events_before,
        "idempotente sin segundo evento"
    );
    assert_eq!(ctx.token_balance(&ctx.buyer), AMOUNT);
}

#[test]
fn ghost_engine_attestation_timeout_refunds() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    assert_eq!(ctx.client().state(), EscrowState::EvidenceSubmitted);
    // attestation_deadline = submit_at + ATTESTATION_PERIOD; submit_at = 0.
    let deadline = FUNDED_AT + ATTESTATION_PERIOD;

    // deadline -1: aún no finalizable, attest todavía permitido.
    ctx.env.ledger().set_timestamp(deadline - 1);
    must_panic(|| {
        ctx.client().finalize();
    });
    assert_eq!(ctx.client().state(), EscrowState::EvidenceSubmitted);

    // deadline: ghost engine → refund.
    ctx.env.ledger().set_timestamp(deadline);
    // attest ya no se admite exactamente en deadline.
    must_panic(|| {
        ctx.client()
            .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env))
    });
    let ret = ctx.client().finalize();
    assert_eq!(ret, EscrowState::Refunded);
    let events = ctx.own_events();
    assert!(
        events.iter().any(|e| *e
            == Finalized {
                reason: FinalizeReason::AttestationTimeout
            }
            .to_xdr(&ctx.env, &ctx.contract)),
        "debe emitir Finalized AttestationTimeout"
    );
    assert_eq!(ctx.client().state(), EscrowState::Refunded);
    assert_eq!(ctx.token_balance(&ctx.buyer), AMOUNT);
    assert_eq!(ctx.token_balance(&ctx.contract), 0);

    // deadline +1 e idempotencia.
    let len_before = ctx.own_events().len();
    ctx.env.ledger().set_timestamp(deadline + 1);
    let ret2 = ctx.client().finalize();
    assert_eq!(ret2, EscrowState::Refunded);
    let len_after = ctx.own_events().len();
    assert_eq!(len_after, len_before, "idempotente sin segundo evento");
    assert_eq!(ctx.token_balance(&ctx.buyer), AMOUNT);
}

#[test]
fn finalize_permissionless_any_caller() {
    let ctx = setup(false);
    // initialize y fund requieren buyer, pero finalize no.
    let init = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "initialize",
        args: (ctx.config(),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let pull = transfer_sub(
        &ctx.token,
        ctx.buyer.clone(),
        ctx.contract.clone(),
        AMOUNT,
        &ctx.env,
    );
    let fund = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "fund",
        args: soroban_sdk::Vec::new(&ctx.env),
        sub_invokes: &[pull],
    };
    let mint = MockAuthInvoke {
        contract: &ctx.token,
        fn_name: "mint",
        args: (ctx.buyer.clone(), AMOUNT).into_val(&ctx.env),
        sub_invokes: &[],
    };
    ctx.env.mock_auths(&[
        MockAuth {
            address: &ctx.token_admin,
            invoke: &mint,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &init,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &fund,
        },
    ]);
    ctx.sac().mint(&ctx.buyer, &AMOUNT);
    ctx.initialize();
    ctx.client().fund();

    let deadline = FUNDED_AT + SUBMISSION_PERIOD;
    ctx.env.ledger().set_timestamp(deadline);

    // Cualquier cuenta (supplier) puede disparar finalize; no hay mock_auth para finalize.
    ctx.env.mock_auths(&[]);
    let ret = ctx.client().finalize();
    assert_eq!(ret, EscrowState::Refunded);
    assert_eq!(
        ctx.token_balance(&ctx.buyer),
        AMOUNT,
        "reembolsa buyer aunque llame supplier"
    );
}

#[test]
fn finalize_rejected_in_created_and_before_deadline() {
    let ctx = setup(true);
    ctx.initialize();
    // En CREATED no hay timeout de fondeo.
    must_panic(|| {
        ctx.client().finalize();
    });
    assert_eq!(ctx.client().state(), EscrowState::Created);

    ctx.mint_and_fund();
    // Funded pero antes del deadline → NotFinalizableYet.
    ctx.env
        .ledger()
        .set_timestamp(FUNDED_AT + SUBMISSION_PERIOD - 1);
    must_panic(|| {
        ctx.client().finalize();
    });
    assert_eq!(ctx.client().state(), EscrowState::Funded);
}

#[test]
fn finalize_idempotent_in_terminal_states() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.client().cancel();
    assert_eq!(ctx.client().state(), EscrowState::Cancelled);
    // finalize en terminal es no-op.
    let ret = ctx.client().finalize();
    assert_eq!(ret, EscrowState::Cancelled);
    assert_eq!(ctx.own_events().len(), 0);

    let ctx2 = setup(true);
    ctx2.to_attested_pass();
    ctx2.client().approve();
    assert_eq!(ctx2.client().state(), EscrowState::Released);
    let ret2 = ctx2.client().finalize();
    assert_eq!(ret2, EscrowState::Released);
    assert_eq!(ctx2.own_events().len(), 0);
}

#[test]
fn attest_after_deadline_is_rejected() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    let deadline = FUNDED_AT + ATTESTATION_PERIOD;
    ctx.env.ledger().set_timestamp(deadline);
    must_panic(|| {
        ctx.client()
            .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env))
    });
    assert_eq!(ctx.client().state(), EscrowState::EvidenceSubmitted);
    // finalize después sí reembolsa.
    ctx.client().finalize();
    assert_eq!(ctx.client().state(), EscrowState::Refunded);
}

#[test]
fn submit_after_attestation_deadline_still_evidence_state() {
    // Tras EVIDENCE_SUBMITTED, el siguiente deadline es de atestación; submit no aplica.
    // Solo verificamos que submit no es el camino tras evidencia.
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    must_panic(|| ctx.client().submit_evidence(&evidence_hash(&ctx.env)));
    assert_eq!(ctx.client().state(), EscrowState::EvidenceSubmitted);
}

#[test]
fn initialize_validates_new_periods_and_fallback() {
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.attestation_period = 0;
    must_panic(|| ctx.client().initialize(&bad));

    let mut bad2 = ctx.config();
    bad2.objection_period = 0;
    must_panic(|| ctx.client().initialize(&bad2));

    let mut bad3 = ctx.config();
    bad3.fallback_outcome = FallbackOutcome::Split;
    bad3.fallback_split_bps = 0;
    must_panic(|| ctx.client().initialize(&bad3));

    let mut bad4 = ctx.config();
    bad4.fallback_outcome = FallbackOutcome::Refund;
    bad4.fallback_split_bps = 5000;
    must_panic(|| ctx.client().initialize(&bad4));

    let mut bad5 = ctx.config();
    bad5.fallback_split_bps = 10_001;
    must_panic(|| ctx.client().initialize(&bad5));
}

#[test]
fn initialize_rejects_buyer_equals_supplier() {
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.supplier = bad.buyer.clone();
    must_panic(|| ctx.client().initialize(&bad));
}

#[test]
fn initialize_rejects_buyer_equals_engine() {
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.engine = bad.buyer.clone();
    must_panic(|| ctx.client().initialize(&bad));
}

#[test]
fn initialize_rejects_supplier_equals_engine() {
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.engine = bad.supplier.clone();
    must_panic(|| ctx.client().initialize(&bad));
}

#[test]
fn initialize_rejects_buyer_equals_resolver() {
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.resolver = bad.buyer.clone();
    must_panic(|| ctx.client().initialize(&bad));
}

#[test]
fn initialize_rejects_supplier_equals_resolver() {
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.resolver = bad.supplier.clone();
    must_panic(|| ctx.client().initialize(&bad));
}

#[test]
fn initialize_rejects_engine_equals_resolver() {
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.resolver = bad.engine.clone();
    must_panic(|| ctx.client().initialize(&bad));
}

#[test]
fn initialize_rejects_same_roles() {
    // las tres iguales (cubierto por los 3 anteriores, pero se mantiene por completitud)
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.supplier = bad.buyer.clone();
    bad.engine = bad.buyer.clone();
    must_panic(|| ctx.client().initialize(&bad));
}

#[test]
fn initialize_rejects_invalid_max_correction_attempts() {
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.max_correction_attempts = 0;
    must_panic(|| ctx.client().initialize(&bad));
    let mut bad2 = ctx.config();
    bad2.max_correction_attempts = 2;
    must_panic(|| ctx.client().initialize(&bad2));
}

#[test]
fn no_objection_timeout_releases_to_supplier() {
    let ctx = setup(true);
    ctx.to_attested_pass();
    let attested_at = FUNDED_AT; // attest ocurre en timestamp 0 en este helper
    let deadline = attested_at + OBJECTION_PERIOD;
    ctx.env.ledger().set_timestamp(deadline - 1);
    must_panic(|| {
        ctx.client().finalize();
    });
    ctx.env.ledger().set_timestamp(deadline);
    let ret = ctx.client().finalize();
    assert_eq!(ret, EscrowState::Released);
    let events = ctx.own_events();
    assert!(
        events.iter().any(|e| *e
            == Finalized {
                reason: FinalizeReason::NoObjection
            }
            .to_xdr(&ctx.env, &ctx.contract)),
        "debe emitir Finalized NoObjection"
    );
    // state después de verificar evento para no limpiar el buffer antes
    assert_eq!(ctx.client().state(), EscrowState::Released);
    assert_eq!(ctx.token_balance(&ctx.supplier), AMOUNT);
    // idempotente
    let len_before = ctx.own_events().len();
    assert_eq!(ctx.client().finalize(), EscrowState::Released);
    let len_after = ctx.own_events().len();
    assert_eq!(len_after, len_before);
}

#[test]
fn approve_rejects_at_and_after_objection_deadline_only_finalize_succeeds() {
    let ctx = setup(true);
    ctx.to_attested_pass();
    // to_attested_pass attest en FUNDED_AT (0), deadline = 0 + OBJECTION_PERIOD
    let deadline = FUNDED_AT + OBJECTION_PERIOD;

    // deadline-1: approve sí puede
    ctx.env.ledger().set_timestamp(deadline - 1);
    ctx.client().approve();
    assert_eq!(ctx.client().state(), EscrowState::Released);
    assert_eq!(ctx.token_balance(&ctx.supplier), AMOUNT);
    assert_eq!(ctx.token_balance(&ctx.contract), 0);

    // reset para probar deadline y deadline+1
    let ctx2 = setup(true);
    ctx2.to_attested_pass();
    assert_eq!(ctx2.client().state(), EscrowState::AttestedPass);

    // deadline: approve debe rechazar, solo finalize
    ctx2.env.ledger().set_timestamp(deadline);
    must_panic(|| ctx2.client().approve());
    assert_eq!(ctx2.client().state(), EscrowState::AttestedPass);
    assert_eq!(ctx2.token_balance(&ctx2.supplier), 0);
    assert_eq!(ctx2.token_balance(&ctx2.contract), AMOUNT);
    let ret = ctx2.client().finalize();
    assert_eq!(ret, EscrowState::Released);
    assert_eq!(ctx2.token_balance(&ctx2.supplier), AMOUNT);

    // deadline+1: idem
    let ctx3 = setup(true);
    ctx3.to_attested_pass();
    ctx3.env.ledger().set_timestamp(deadline + 1);
    must_panic(|| ctx3.client().approve());
    assert_eq!(ctx3.client().state(), EscrowState::AttestedPass);
    let ret3 = ctx3.client().finalize();
    assert_eq!(ret3, EscrowState::Released);
    assert_eq!(ctx3.token_balance(&ctx3.supplier), AMOUNT);

    // Disputed no es terminal en finalize P0-03: finalize en Disputed debe ser NotFinalizableYet
    // (simulado: seteamos estado Disputed directamente para verificar que no lo trata como terminal)
    let ctx4 = setup(true);
    ctx4.initialize();
    ctx4.env.as_contract(&ctx4.contract, || {
        ctx4.env
            .storage()
            .instance()
            .set(&DataKey::State, &EscrowState::Disputed)
    });
    assert_eq!(ctx4.client().state(), EscrowState::Disputed);
    must_panic(|| {
        ctx4.client().finalize();
    });
    assert_eq!(ctx4.client().state(), EscrowState::Disputed);
}

#[test]
fn correction_timeout_refunds_buyer() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.client()
        .attest(&AttestationOutcome::Fail, &report_hash(&ctx.env));
    let deadline = FUNDED_AT + CORRECTION_PERIOD;
    ctx.env.ledger().set_timestamp(deadline - 1);
    must_panic(|| {
        ctx.client().finalize();
    });
    ctx.env.ledger().set_timestamp(deadline);
    let ret = ctx.client().finalize();
    assert_eq!(ret, EscrowState::Refunded);
    let events = ctx.own_events();
    assert!(
        events.iter().any(|e| *e
            == Finalized {
                reason: FinalizeReason::CorrectionTimeout
            }
            .to_xdr(&ctx.env, &ctx.contract)),
        "debe emitir Finalized CorrectionTimeout"
    );
    assert_eq!(ctx.client().state(), EscrowState::Refunded);
    assert_eq!(ctx.token_balance(&ctx.buyer), AMOUNT);
}

// ── P0-04: corrección única, disputa y fallback ──

#[test]
fn correction_increments_attempt_and_restarts_deadline() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.client()
        .attest(&AttestationOutcome::Fail, &report_hash(&ctx.env));
    assert_eq!(ctx.client().state(), EscrowState::AttestedFail);
    let new_hash = BytesN::from_array(&ctx.env, &[8u8; 32]);
    ctx.client().submit_evidence(&new_hash);
    let events = ctx.own_events();
    assert!(events.iter().any(|e| *e
        == EvidenceSubmitted {
            attempt: 1,
            evidence_bundle_hash: new_hash.clone()
        }
        .to_xdr(&ctx.env, &ctx.contract)));
    assert_eq!(ctx.client().state(), EscrowState::EvidenceSubmitted);
    ctx.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env));
    assert_eq!(ctx.client().state(), EscrowState::AttestedPass);
}

#[test]
fn second_correction_is_rejected_but_dispute_after_second_fail_allowed() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.client()
        .attest(&AttestationOutcome::Fail, &report_hash(&ctx.env));
    ctx.client()
        .submit_evidence(&BytesN::from_array(&ctx.env, &[8u8; 32]));
    ctx.client()
        .attest(&AttestationOutcome::Fail, &report_hash(&ctx.env));
    assert_eq!(ctx.client().state(), EscrowState::AttestedFail);
    must_panic(|| {
        ctx.client()
            .submit_evidence(&BytesN::from_array(&ctx.env, &[9u8; 32]))
    });
    assert_eq!(ctx.client().state(), EscrowState::AttestedFail);
    let reason = BytesN::from_array(&ctx.env, &[10u8; 32]);
    let evidence = BytesN::from_array(&ctx.env, &[11u8; 32]);
    ctx.client().raise_dispute(&reason, &evidence);
    assert_eq!(ctx.client().state(), EscrowState::Disputed);
}

#[test]
fn raise_dispute_requires_hashes_and_correct_actor_within_deadline() {
    let ctx = setup(true);
    ctx.to_attested_pass();
    let reason = BytesN::from_array(&ctx.env, &[10u8; 32]);
    let evidence = BytesN::from_array(&ctx.env, &[11u8; 32]);
    ctx.client().raise_dispute(&reason, &evidence);
    let events = ctx.own_events();
    assert!(events.iter().any(|e| *e
        == DisputeRaised {
            by: ctx.buyer.clone(),
            reason_hash: reason.clone(),
            dispute_evidence_hash: evidence.clone()
        }
        .to_xdr(&ctx.env, &ctx.contract)));
    assert_eq!(ctx.client().state(), EscrowState::Disputed);

    let ctx2 = setup(false);
    let reason2 = BytesN::from_array(&ctx2.env, &[10u8; 32]);
    let evidence2 = BytesN::from_array(&ctx2.env, &[11u8; 32]);
    let init2 = MockAuthInvoke {
        contract: &ctx2.contract,
        fn_name: "initialize",
        args: (ctx2.config(),).into_val(&ctx2.env),
        sub_invokes: &[],
    };
    let pull2 = transfer_sub(
        &ctx2.token,
        ctx2.buyer.clone(),
        ctx2.contract.clone(),
        AMOUNT,
        &ctx2.env,
    );
    let fund2 = MockAuthInvoke {
        contract: &ctx2.contract,
        fn_name: "fund",
        args: soroban_sdk::Vec::new(&ctx2.env),
        sub_invokes: &[pull2],
    };
    let submit2 = MockAuthInvoke {
        contract: &ctx2.contract,
        fn_name: "submit_evidence",
        args: (evidence_hash(&ctx2.env),).into_val(&ctx2.env),
        sub_invokes: &[],
    };
    let attest2 = MockAuthInvoke {
        contract: &ctx2.contract,
        fn_name: "attest",
        args: (AttestationOutcome::Pass, report_hash(&ctx2.env)).into_val(&ctx2.env),
        sub_invokes: &[],
    };
    let dispute2 = MockAuthInvoke {
        contract: &ctx2.contract,
        fn_name: "raise_dispute",
        args: (reason2.clone(), evidence2.clone()).into_val(&ctx2.env),
        sub_invokes: &[],
    };
    let mint2 = MockAuthInvoke {
        contract: &ctx2.token,
        fn_name: "mint",
        args: (ctx2.buyer.clone(), AMOUNT).into_val(&ctx2.env),
        sub_invokes: &[],
    };
    ctx2.env.mock_auths(&[
        MockAuth {
            address: &ctx2.token_admin,
            invoke: &mint2,
        },
        MockAuth {
            address: &ctx2.buyer,
            invoke: &init2,
        },
        MockAuth {
            address: &ctx2.buyer,
            invoke: &fund2,
        },
        MockAuth {
            address: &ctx2.supplier,
            invoke: &submit2,
        },
        MockAuth {
            address: &ctx2.engine,
            invoke: &attest2,
        },
        MockAuth {
            address: &ctx2.supplier,
            invoke: &dispute2,
        },
    ]);
    ctx2.sac().mint(&ctx2.buyer, &AMOUNT);
    ctx2.initialize();
    ctx2.client().fund();
    ctx2.client().submit_evidence(&evidence_hash(&ctx2.env));
    ctx2.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx2.env));
    must_panic(|| ctx2.client().raise_dispute(&reason2, &evidence2));

    let ctx3 = setup(true);
    ctx3.initialize();
    ctx3.mint_and_fund();
    ctx3.client().submit_evidence(&evidence_hash(&ctx3.env));
    ctx3.client()
        .attest(&AttestationOutcome::Fail, &report_hash(&ctx3.env));
    let reason3 = BytesN::from_array(&ctx3.env, &[10u8; 32]);
    let evidence3 = BytesN::from_array(&ctx3.env, &[11u8; 32]);
    ctx3.client().raise_dispute(&reason3, &evidence3);
    assert_eq!(ctx3.client().state(), EscrowState::Disputed);

    let ctx4 = setup(false);
    let reason4 = BytesN::from_array(&ctx4.env, &[10u8; 32]);
    let evidence4 = BytesN::from_array(&ctx4.env, &[11u8; 32]);
    let init4 = MockAuthInvoke {
        contract: &ctx4.contract,
        fn_name: "initialize",
        args: (ctx4.config(),).into_val(&ctx4.env),
        sub_invokes: &[],
    };
    let pull4 = transfer_sub(
        &ctx4.token,
        ctx4.buyer.clone(),
        ctx4.contract.clone(),
        AMOUNT,
        &ctx4.env,
    );
    let fund4 = MockAuthInvoke {
        contract: &ctx4.contract,
        fn_name: "fund",
        args: soroban_sdk::Vec::new(&ctx4.env),
        sub_invokes: &[pull4],
    };
    let submit4 = MockAuthInvoke {
        contract: &ctx4.contract,
        fn_name: "submit_evidence",
        args: (evidence_hash(&ctx4.env),).into_val(&ctx4.env),
        sub_invokes: &[],
    };
    let attest4 = MockAuthInvoke {
        contract: &ctx4.contract,
        fn_name: "attest",
        args: (AttestationOutcome::Fail, report_hash(&ctx4.env)).into_val(&ctx4.env),
        sub_invokes: &[],
    };
    let dispute4 = MockAuthInvoke {
        contract: &ctx4.contract,
        fn_name: "raise_dispute",
        args: (reason4.clone(), evidence4.clone()).into_val(&ctx4.env),
        sub_invokes: &[],
    };
    let mint4 = MockAuthInvoke {
        contract: &ctx4.token,
        fn_name: "mint",
        args: (ctx4.buyer.clone(), AMOUNT).into_val(&ctx4.env),
        sub_invokes: &[],
    };
    ctx4.env.mock_auths(&[
        MockAuth {
            address: &ctx4.token_admin,
            invoke: &mint4,
        },
        MockAuth {
            address: &ctx4.buyer,
            invoke: &init4,
        },
        MockAuth {
            address: &ctx4.buyer,
            invoke: &fund4,
        },
        MockAuth {
            address: &ctx4.supplier,
            invoke: &submit4,
        },
        MockAuth {
            address: &ctx4.engine,
            invoke: &attest4,
        },
        MockAuth {
            address: &ctx4.buyer,
            invoke: &dispute4,
        },
    ]);
    ctx4.sac().mint(&ctx4.buyer, &AMOUNT);
    ctx4.initialize();
    ctx4.client().fund();
    ctx4.client().submit_evidence(&evidence_hash(&ctx4.env));
    ctx4.client()
        .attest(&AttestationOutcome::Fail, &report_hash(&ctx4.env));
    must_panic(|| ctx4.client().raise_dispute(&reason4, &evidence4));

    let ctx5 = setup(true);
    ctx5.to_attested_pass();
    ctx5.env
        .ledger()
        .set_timestamp(FUNDED_AT + OBJECTION_PERIOD);
    let reason5 = BytesN::from_array(&ctx5.env, &[10u8; 32]);
    let evidence5 = BytesN::from_array(&ctx5.env, &[11u8; 32]);
    must_panic(|| ctx5.client().raise_dispute(&reason5, &evidence5));

    let ctx6 = setup(true);
    ctx6.to_attested_pass();
    let reason6 = BytesN::from_array(&ctx6.env, &[10u8; 32]);
    let evidence6 = BytesN::from_array(&ctx6.env, &[11u8; 32]);
    ctx6.client().raise_dispute(&reason6, &evidence6);
    must_panic(|| ctx6.client().raise_dispute(&reason6, &evidence6));
}

#[test]
fn resolve_before_deadline_with_split_validation() {
    let ctx = setup(true);
    ctx.to_attested_pass();
    let reason = BytesN::from_array(&ctx.env, &[10u8; 32]);
    let evidence = BytesN::from_array(&ctx.env, &[11u8; 32]);
    ctx.client().raise_dispute(&reason, &evidence);
    ctx.client().resolve(&FallbackOutcome::Split, &5000);
    let events = ctx.own_events();
    assert!(events.iter().any(|e| *e
        == Resolved {
            by: ctx.resolver.clone(),
            outcome: FallbackOutcome::Split,
            split_bps: 5000
        }
        .to_xdr(&ctx.env, &ctx.contract)));
    assert_eq!(ctx.client().state(), EscrowState::Split);
    assert_eq!(ctx.token_balance(&ctx.supplier), AMOUNT * 5000 / 10_000);
    assert_eq!(
        ctx.token_balance(&ctx.buyer),
        AMOUNT - AMOUNT * 5000 / 10_000
    );

    let ctx2 = setup(true);
    ctx2.to_attested_pass();
    let reason2 = BytesN::from_array(&ctx2.env, &[10u8; 32]);
    let evidence2 = BytesN::from_array(&ctx2.env, &[11u8; 32]);
    ctx2.client().raise_dispute(&reason2, &evidence2);
    must_panic(|| ctx2.client().resolve(&FallbackOutcome::Split, &0));
    must_panic(|| ctx2.client().resolve(&FallbackOutcome::Split, &10000));
    must_panic(|| ctx2.client().resolve(&FallbackOutcome::Split, &10001));
    must_panic(|| ctx2.client().resolve(&FallbackOutcome::Release, &5000));
    assert_eq!(ctx2.client().state(), EscrowState::Disputed);
    assert_eq!(ctx2.token_balance(&ctx2.contract), AMOUNT);

    let ctx3 = setup(true);
    ctx3.to_attested_pass();
    let reason3 = BytesN::from_array(&ctx3.env, &[10u8; 32]);
    let evidence3 = BytesN::from_array(&ctx3.env, &[11u8; 32]);
    ctx3.client().raise_dispute(&reason3, &evidence3);
    ctx3.env
        .ledger()
        .set_timestamp(FUNDED_AT + RESOLUTION_PERIOD);
    must_panic(|| ctx3.client().resolve(&FallbackOutcome::Refund, &0));
}

#[test]
fn resolve_by_non_resolver_rejected() {
    // Auth estricto: solo resolver puede resolver, buyer/supplier/engine no.
    let ctx = setup(false);
    let reason = BytesN::from_array(&ctx.env, &[10u8; 32]);
    let evidence = BytesN::from_array(&ctx.env, &[11u8; 32]);
    let init = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "initialize",
        args: (ctx.config(),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let pull = transfer_sub(
        &ctx.token,
        ctx.buyer.clone(),
        ctx.contract.clone(),
        AMOUNT,
        &ctx.env,
    );
    let fund = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "fund",
        args: soroban_sdk::Vec::new(&ctx.env),
        sub_invokes: &[pull],
    };
    let submit = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "submit_evidence",
        args: (evidence_hash(&ctx.env),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let attest = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "attest",
        args: (AttestationOutcome::Pass, report_hash(&ctx.env)).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let dispute = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "raise_dispute",
        args: (reason.clone(), evidence.clone()).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let resolve_buyer = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "resolve",
        args: (FallbackOutcome::Refund, 0u32).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let resolve_supplier = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "resolve",
        args: (FallbackOutcome::Release, 0u32).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let resolve_engine = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "resolve",
        args: (FallbackOutcome::Split, 5000u32).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let mint = MockAuthInvoke {
        contract: &ctx.token,
        fn_name: "mint",
        args: (ctx.buyer.clone(), AMOUNT).into_val(&ctx.env),
        sub_invokes: &[],
    };
    ctx.env.mock_auths(&[
        MockAuth {
            address: &ctx.token_admin,
            invoke: &mint,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &init,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &fund,
        },
        MockAuth {
            address: &ctx.supplier,
            invoke: &submit,
        },
        MockAuth {
            address: &ctx.engine,
            invoke: &attest,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &dispute,
        },
        MockAuth {
            address: &ctx.buyer,
            invoke: &resolve_buyer,
        },
        MockAuth {
            address: &ctx.supplier,
            invoke: &resolve_supplier,
        },
        MockAuth {
            address: &ctx.engine,
            invoke: &resolve_engine,
        },
    ]);
    ctx.sac().mint(&ctx.buyer, &AMOUNT);
    ctx.initialize();
    ctx.client().fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env));
    ctx.client().raise_dispute(&reason, &evidence);
    assert_eq!(ctx.client().state(), EscrowState::Disputed);
    must_panic(|| ctx.client().resolve(&FallbackOutcome::Refund, &0));
    must_panic(|| ctx.client().resolve(&FallbackOutcome::Release, &0));
    must_panic(|| ctx.client().resolve(&FallbackOutcome::Split, &5000));
    assert_eq!(ctx.client().state(), EscrowState::Disputed);
    assert_eq!(ctx.token_balance(&ctx.contract), AMOUNT);
}

#[test]
fn finalize_disputed_fallback_and_double_settlement_rejected() {
    let ctx = setup(true);
    let mut cfg = ctx.config();
    cfg.fallback_outcome = FallbackOutcome::Split;
    cfg.fallback_split_bps = 7000;
    ctx.client().initialize(&cfg);
    ctx.sac().mint(&cfg.buyer, &AMOUNT);
    ctx.client().fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env));
    ctx.client().raise_dispute(
        &BytesN::from_array(&ctx.env, &[10u8; 32]),
        &BytesN::from_array(&ctx.env, &[11u8; 32]),
    );
    ctx.env
        .ledger()
        .set_timestamp(FUNDED_AT + RESOLUTION_PERIOD);
    let ret = ctx.client().finalize();
    assert_eq!(ret, EscrowState::Split);
    let events = ctx.own_events();
    assert!(events.iter().any(|e| *e
        == Finalized {
            reason: FinalizeReason::ResolutionTimeout
        }
        .to_xdr(&ctx.env, &ctx.contract)));
    assert_eq!(ctx.token_balance(&ctx.supplier), AMOUNT * 7000 / 10_000);
    assert_eq!(
        ctx.token_balance(&ctx.buyer),
        AMOUNT - AMOUNT * 7000 / 10_000
    );

    must_panic(|| ctx.client().resolve(&FallbackOutcome::Refund, &0));
    must_panic(|| {
        ctx.client().raise_dispute(
            &BytesN::from_array(&ctx.env, &[10u8; 32]),
            &BytesN::from_array(&ctx.env, &[11u8; 32]),
        )
    });
    must_panic(|| ctx.client().approve());
    must_panic(|| ctx.client().submit_evidence(&evidence_hash(&ctx.env)));
}

#[test]
fn fallback_release_refund_split_and_rounding_large_amount_and_deadline_edges() {
    // Fallback Release
    let ctx = setup(true);
    let mut cfg = ctx.config();
    cfg.fallback_outcome = FallbackOutcome::Release;
    cfg.fallback_split_bps = 0;
    ctx.client().initialize(&cfg);
    ctx.sac().mint(&cfg.buyer, &AMOUNT);
    ctx.client().fund();
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env));
    ctx.client().raise_dispute(
        &BytesN::from_array(&ctx.env, &[10u8; 32]),
        &BytesN::from_array(&ctx.env, &[11u8; 32]),
    );
    // deadline-1 no finalizable
    ctx.env
        .ledger()
        .set_timestamp(FUNDED_AT + RESOLUTION_PERIOD - 1);
    must_panic(|| {
        ctx.client().finalize();
    });
    // deadline sí
    ctx.env
        .ledger()
        .set_timestamp(FUNDED_AT + RESOLUTION_PERIOD);
    let ret = ctx.client().finalize();
    assert_eq!(ret, EscrowState::Released);
    assert_eq!(ctx.token_balance(&ctx.supplier), AMOUNT);
    assert_eq!(ctx.token_balance(&ctx.buyer), 0);
    // deadline+1 idempotente
    assert_eq!(ctx.client().finalize(), EscrowState::Released);

    // Fallback Refund
    let ctx2 = setup(true);
    let mut cfg2 = ctx2.config();
    cfg2.fallback_outcome = FallbackOutcome::Refund;
    cfg2.fallback_split_bps = 0;
    ctx2.client().initialize(&cfg2);
    ctx2.sac().mint(&cfg2.buyer, &AMOUNT);
    ctx2.client().fund();
    ctx2.client().submit_evidence(&evidence_hash(&ctx2.env));
    ctx2.client()
        .attest(&AttestationOutcome::Fail, &report_hash(&ctx2.env));
    ctx2.client().raise_dispute(
        &BytesN::from_array(&ctx2.env, &[10u8; 32]),
        &BytesN::from_array(&ctx2.env, &[11u8; 32]),
    );
    ctx2.env
        .ledger()
        .set_timestamp(FUNDED_AT + RESOLUTION_PERIOD);
    let ret2 = ctx2.client().finalize();
    assert_eq!(ret2, EscrowState::Refunded);
    assert_eq!(ctx2.token_balance(&ctx2.buyer), AMOUNT);

    // Fallback Split redondeo: amount no divisible por 10000
    let ctx3 = setup(true);
    let mut cfg3 = ctx3.config();
    cfg3.amount = 10_001; // no divisible
    cfg3.fallback_outcome = FallbackOutcome::Split;
    cfg3.fallback_split_bps = 3333; // 33.33%
    ctx3.client().initialize(&cfg3);
    ctx3.sac().mint(&cfg3.buyer, &10_001);
    ctx3.client().fund();
    ctx3.client().submit_evidence(&evidence_hash(&ctx3.env));
    ctx3.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx3.env));
    ctx3.client().raise_dispute(
        &BytesN::from_array(&ctx3.env, &[10u8; 32]),
        &BytesN::from_array(&ctx3.env, &[11u8; 32]),
    );
    ctx3.env
        .ledger()
        .set_timestamp(FUNDED_AT + RESOLUTION_PERIOD);
    let ret3 = ctx3.client().finalize();
    assert_eq!(ret3, EscrowState::Split);
    let (s, b) = crate::calc_split_amounts(&ctx3.env, 10_001, 3333);
    assert_eq!(s + b, 10_001, "conserva monto");
    assert_eq!(ctx3.token_balance(&ctx3.supplier), s);
    assert_eq!(ctx3.token_balance(&ctx3.buyer), b);

    // Monto grande sin overflow: i128::MAX / 4
    let large: i128 = i128::MAX / 4;
    let ctx4 = setup(true);
    let mut cfg4 = ctx4.config();
    cfg4.amount = large;
    cfg4.fallback_outcome = FallbackOutcome::Split;
    cfg4.fallback_split_bps = 5000;
    ctx4.client().initialize(&cfg4);
    ctx4.sac().mint(&cfg4.buyer, &large);
    ctx4.client().fund();
    ctx4.client().submit_evidence(&evidence_hash(&ctx4.env));
    ctx4.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx4.env));
    ctx4.client().raise_dispute(
        &BytesN::from_array(&ctx4.env, &[10u8; 32]),
        &BytesN::from_array(&ctx4.env, &[11u8; 32]),
    );
    ctx4.env
        .ledger()
        .set_timestamp(FUNDED_AT + RESOLUTION_PERIOD);
    let ret4 = ctx4.client().finalize();
    assert_eq!(ret4, EscrowState::Split);
    let (s4, b4) = crate::calc_split_amounts(&ctx4.env, large, 5000);
    assert_eq!(s4 + b4, large);
    assert_eq!(ctx4.token_balance(&ctx4.supplier), s4);
    assert_eq!(ctx4.token_balance(&ctx4.buyer), b4);

    // resolve() con Split también usa función segura y conserva
    let ctx5 = setup(true);
    ctx5.to_attested_pass();
    let r = BytesN::from_array(&ctx5.env, &[10u8; 32]);
    let e = BytesN::from_array(&ctx5.env, &[11u8; 32]);
    ctx5.client().raise_dispute(&r, &e);
    ctx5.client().resolve(&FallbackOutcome::Split, &1);
    assert_eq!(ctx5.token_balance(&ctx5.supplier), AMOUNT / 10_000); // 1 bps
    assert_eq!(
        ctx5.token_balance(&ctx5.buyer) + ctx5.token_balance(&ctx5.supplier),
        AMOUNT
    );
}

#[test]
fn split_rounding_pins_actual_shares_with_independent_expectations() {
    // El test anterior de redondeo comparaba el resultado de `calc_split_amounts`
    // consigo mismo, asi que solo comprobaba que se conservara el monto: cualquier
    // reparto que sumara el total pasaba. Aqui los valores esperados estan escritos a
    // mano, de modo que un cambio en el reparto rompe el test y tiene que ser una
    // decision consciente.
    //
    // Regla: supplier = (amount / 10_000) * bps + ((amount % 10_000) * bps) / 10_000,
    // con division entera en los dos cortes; el resto se queda el buyer.
    let casos: &[(i128, u32, i128, i128)] = &[
        // amount divisible: el corte es limpio.
        (10_000, 3_333, 3_333, 6_667),
        // amount no divisible y resto que no alcanza para un punto entero.
        (10_001, 3_333, 3_333, 6_668),
        // amount menor que 10_000: la parte entera es 0 y decide solo el resto.
        (999, 3_333, 332, 667),
        // 1 bps sobre un monto pequeno: trunca a cero, el buyer se queda todo.
        (12_345, 1, 1, 12_344),
        // 1 bps sobre 10_000_000_000.
        (10_000_000_000, 1, 1_000_000, 9_999_000_000),
        // bps maximo sobre el monto minimo: el supplier no puede inventarse nada.
        (1, 9_999, 0, 1),
        // 0 bps y 10_000 bps: extremos exactos.
        (10_000, 0, 0, 10_000),
        (10_000, 10_000, 10_000, 0),
    ];
    for (amount, bps, want_supplier, want_buyer) in casos {
        let (s, b) = crate::calc_split_amounts(&Env::default(), *amount, *bps);
        assert_eq!(
            (s, b),
            (*want_supplier, *want_buyer),
            "reparto inesperado para amount={amount} bps={bps}: supplier={s} buyer={b}"
        );
        assert_eq!(s + b, *amount, "no se crea ni se destruye monto");
    }
}

#[test]
fn split_of_a_very_large_amount_is_exact_and_conserves() {
    // i128::MAX/4 multiplicado por 5000 desbordaria i128 si se calculara como
    // amount * bps. Con el corte en dos, un split al 50% es exactamente la mitad.
    let large: i128 = i128::MAX / 4;
    let (s, b) = crate::calc_split_amounts(&Env::default(), large, 5_000);
    assert_eq!(
        s,
        large / 2,
        "al 50% el supplier se lleva la mitad truncada"
    );
    assert_eq!(b, large - s);
    assert_eq!(s + b, large, "el monto se conserva integro");
    assert!(s > 0 && b > 0, "ambas partes reciben algo");
}

#[test]
fn split_never_moves_more_than_the_amount() {
    // Invariante de seguridad: por como se componga la division, ninguna parte puede
    // exceder el monto ni quedar negativa, para ningun bps valido.
    let amounts: &[i128] = &[1, 7, 9_999, 10_000, 10_001, 1_000_000, i128::MAX / 8];
    for amount in amounts {
        for bps in [0u32, 1, 3_333, 5_000, 9_999, 10_000] {
            let (s, b) = crate::calc_split_amounts(&Env::default(), *amount, bps);
            assert!(s >= 0, "supplier negativo: amount={amount} bps={bps}");
            assert!(b >= 0, "buyer negativo: amount={amount} bps={bps}");
            assert!(
                s <= *amount,
                "supplier excede el monto: amount={amount} bps={bps}"
            );
            assert!(
                b <= *amount,
                "buyer excede el monto: amount={amount} bps={bps}"
            );
            assert_eq!(
                s + b,
                *amount,
                "monto no conservado: amount={amount} bps={bps}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// snapshot(): la lectura que consume el agente de atestación.
// ---------------------------------------------------------------------------

#[test]
fn snapshot_exposes_config_sin_posiciones() {
    let ctx = setup(true);
    ctx.initialize();
    let snap = ctx.client().snapshot();

    // El agente necesita monto, token y ventana de atestación; antes tenía que
    // adivinar el orden de los campos dentro del ScVec de Config.
    assert_eq!(snap.state, EscrowState::Created);
    assert_eq!(snap.config.amount, AMOUNT);
    assert_eq!(snap.config.token, ctx.token);
    assert_eq!(snap.config.engine, ctx.engine);
    assert_eq!(snap.config.attestation_period, ATTESTATION_PERIOD);
    assert_eq!(snap.ledger_timestamp, ctx.env.ledger().timestamp());
}

#[test]
fn snapshot_en_created_no_inventa_plazos_ni_hashes() {
    let ctx = setup(true);
    ctx.initialize();
    let snap = ctx.client().snapshot();

    // Antes de fondear no existe deadline de nada: afirmar que existe
    // invitaba a un motor a calcular plazos sobre un plazo inventado.
    assert_eq!(snap.evidence_bundle_hash, None);
    assert_eq!(snap.report_hash, None);
    assert_eq!(snap.submission_deadline, None);
    assert_eq!(snap.attestation_deadline, None);
    assert_eq!(snap.correction_attempts, 0);
}

#[test]
fn snapshot_refleja_evidence_submitted() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.env.ledger().set_timestamp(FUNDED_AT + 10);
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));

    let snap = ctx.client().snapshot();
    assert_eq!(snap.state, EscrowState::EvidenceSubmitted);
    assert_eq!(snap.evidence_bundle_hash, Some(evidence_hash(&ctx.env)));
    // El plazo de atestación cuelga del momento real de la entrega, no del fondeo.
    assert_eq!(
        snap.attestation_deadline,
        Some(FUNDED_AT + 10 + ATTESTATION_PERIOD)
    );
    assert_eq!(
        snap.submission_deadline,
        Some(FUNDED_AT + SUBMISSION_PERIOD)
    );
    assert_eq!(snap.funded_at, Some(FUNDED_AT));
}

#[test]
fn snapshot_deriva_objection_y_correction_de_attested_at() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.env.ledger().set_timestamp(FUNDED_AT + 10);
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.env.ledger().set_timestamp(FUNDED_AT + 20);
    ctx.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env));

    let snap = ctx.client().snapshot();
    assert_eq!(snap.state, EscrowState::AttestedPass);
    assert_eq!(snap.report_hash, Some(report_hash(&ctx.env)));
    assert_eq!(snap.attested_at, Some(FUNDED_AT + 20));
    assert_eq!(
        snap.objection_deadline,
        Some(FUNDED_AT + 20 + OBJECTION_PERIOD)
    );
    assert_eq!(
        snap.correction_deadline,
        Some(FUNDED_AT + 20 + CORRECTION_PERIOD)
    );
}

#[test]
fn snapshot_registra_correction_attempts_tras_corregir() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.env.ledger().set_timestamp(FUNDED_AT + 10);
    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.env.ledger().set_timestamp(FUNDED_AT + 20);
    ctx.client()
        .attest(&AttestationOutcome::Fail, &report_hash(&ctx.env));
    assert_eq!(ctx.client().snapshot().correction_attempts, 0);

    let corrected = BytesN::from_array(&ctx.env, &[21u8; 32]);
    ctx.env.ledger().set_timestamp(FUNDED_AT + 30);
    ctx.client().submit_evidence(&corrected);

    // Tras la corrección el bundle nuevo es el que el motor debe evaluar, y el
    // contador deja claro que ya se consumió el único intento.
    let snap = ctx.client().snapshot();
    assert_eq!(snap.state, EscrowState::EvidenceSubmitted);
    assert_eq!(snap.evidence_bundle_hash, Some(corrected));
    assert_eq!(snap.correction_attempts, 1);
}

#[test]
fn snapshot_expone_disputa_sin_perder_el_report_hash() {
    let ctx = setup(true);
    ctx.to_attested_pass();
    let reason = BytesN::from_array(&ctx.env, &[10u8; 32]);
    let evid = BytesN::from_array(&ctx.env, &[11u8; 32]);
    ctx.client().raise_dispute(&reason, &evid);

    let snap = ctx.client().snapshot();
    assert_eq!(snap.state, EscrowState::Disputed);
    assert_eq!(snap.dispute_reason_hash, Some(reason));
    assert_eq!(snap.dispute_evidence_hash, Some(evid));
    // El reporte sigue siendo la traza de lo que el engine dijo, aunque ya no
    // sea el estado vigente: es evidencia de auditoría, no estado actual.
    assert_eq!(snap.report_hash, Some(report_hash(&ctx.env)));
}

#[test]
fn snapshot_es_estable_tras_finalizar() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();
    ctx.env
        .ledger()
        .set_timestamp(FUNDED_AT + SUBMISSION_PERIOD);
    ctx.client().finalize();

    let snap = ctx.client().snapshot();
    assert_eq!(snap.state, EscrowState::Refunded);
    // Un escrow cerrado ya no es atestestable y no debe seguir anunciando plazo.
    assert_eq!(snap.attestation_deadline, None);
    assert_eq!(snap.evidence_bundle_hash, None);
}
