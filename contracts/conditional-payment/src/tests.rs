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
    let contract = env.register(ConditionalPayment, ());
    TestContext {
        env,
        contract,
        token,
        token_admin,
        buyer,
        supplier,
        engine,
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
            token: self.token.clone(),
            amount: AMOUNT,
            submission_period: SUBMISSION_PERIOD,
            attestation_period: ATTESTATION_PERIOD,
            objection_period: OBJECTION_PERIOD,
            correction_period: CORRECTION_PERIOD,
            resolution_period: RESOLUTION_PERIOD,
            fallback_outcome: FallbackOutcome::Refund,
            fallback_split_bps: 0,
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
            token: ctx.token.clone(),
            amount: AMOUNT,
            submission_period: SUBMISSION_PERIOD,
            attestation_period: ATTESTATION_PERIOD,
            objection_period: OBJECTION_PERIOD,
            correction_period: CORRECTION_PERIOD,
            resolution_period: RESOLUTION_PERIOD,
            fallback_outcome: FallbackOutcome::Refund,
            fallback_split_bps: 0,
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
fn initialize_rejects_same_roles() {
    // las tres iguales (cubierto por los 3 anteriores, pero se mantiene por completitud)
    let ctx = setup(true);
    let mut bad = ctx.config();
    bad.supplier = bad.buyer.clone();
    bad.engine = bad.buyer.clone();
    must_panic(|| ctx.client().initialize(&bad));
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
