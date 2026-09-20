//! Pruebas de aceptación P0-02.
//!
//! Cubren el camino feliz `CREATED → FUNDED → EVIDENCE_SUBMITTED →
//! ATTESTED_PASS → RELEASED`, el de fallo (`ATTESTED_FAIL` retiene fondos) y
//! `CREATED → CANCELLED`, más las reglas de negocio: fondeo por pull atómico,
//! recuperación tras fallo, `recover()` de saldos enviados por error,
//! deadline de evidencia, exclusividad de roles con firmante equivocado y
//! doble aprobación.

use super::*;
extern crate std;

use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _, MockAuth, MockAuthInvoke};
use soroban_sdk::Event as _;
use soroban_sdk::{token::StellarAssetClient, xdr, Address, BytesN, IntoVal, MuxedAddress};

const AMOUNT: i128 = 10_000_000_000;
const STRAY: i128 = 2_000_000_000;
const SUBMISSION_PERIOD: u64 = 1000;
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
    let token = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
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
    ctx.assert_single_event(EscrowCreated {
        buyer: ctx.buyer.clone(),
        supplier: ctx.supplier.clone(),
        engine: ctx.engine.clone(),
        token: ctx.token.clone(),
        amount: AMOUNT,
        submission_period: SUBMISSION_PERIOD,
    }
    .to_xdr(&ctx.env, &ctx.contract));
    assert_eq!(ctx.client().state(), EscrowState::Created);
    assert_eq!(ctx.client().config(), ctx.config());

    ctx.sac().mint(&ctx.buyer, &AMOUNT);
    ctx.client().fund();
    ctx.assert_single_event(Funded {
        funded_at: FUNDED_AT,
        submission_deadline: FUNDED_AT + SUBMISSION_PERIOD,
    }
    .to_xdr(&ctx.env, &ctx.contract));
    assert_eq!(ctx.client().state(), EscrowState::Funded);
    assert_eq!(ctx.token_balance(&ctx.contract), AMOUNT, "el pull movió amount");

    ctx.client().submit_evidence(&evidence_hash(&ctx.env));
    ctx.assert_single_event(EvidenceSubmitted {
        attempt: 0,
        evidence_bundle_hash: evidence_hash(&ctx.env),
    }
    .to_xdr(&ctx.env, &ctx.contract));
    assert_eq!(ctx.client().state(), EscrowState::EvidenceSubmitted);

    ctx.client()
        .attest(&AttestationOutcome::Pass, &report_hash(&ctx.env));
    ctx.assert_single_event(Attested {
        outcome: AttestationOutcome::Pass,
        report_hash: report_hash(&ctx.env),
    }
    .to_xdr(&ctx.env, &ctx.contract));
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
    assert_eq!(ctx.token_balance(&ctx.contract), 0, "el contrato queda a cero");
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
    assert_eq!(ctx.client().state(), EscrowState::Created, "no cambia de estado");
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
    assert_eq!(ctx.client().state(), EscrowState::Created, "no cambia de estado");
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
    ctx.assert_single_event(Funded {
        funded_at: FUNDED_AT,
        submission_deadline: FUNDED_AT + SUBMISSION_PERIOD,
    }
    .to_xdr(&ctx.env, &ctx.contract));
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
fn buyer_recovers_stray_balance_in_created() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.sac().mint(&ctx.buyer, &STRAY);
    TokenClient::new(&ctx.env, &ctx.token).transfer(&ctx.buyer, &ctx.contract, &STRAY);
    assert_eq!(ctx.token_balance(&ctx.contract), STRAY);

    ctx.client().recover();
    ctx.assert_single_event(
        Recovered {
            to: ctx.buyer.clone(),
            amount: STRAY,
        }
        .to_xdr(&ctx.env, &ctx.contract),
    );
    assert_eq!(ctx.token_balance(&ctx.buyer), STRAY, "el buyer recupera todo");
    assert_eq!(ctx.token_balance(&ctx.contract), 0);
    assert_eq!(ctx.client().state(), EscrowState::Created);

    // Y el fondeo normal sigue funcionando después.
    ctx.sac().mint(&ctx.buyer, &AMOUNT);
    ctx.client().fund();
    assert_eq!(ctx.client().state(), EscrowState::Funded);
}

#[test]
fn recover_only_in_created() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();

    must_panic(|| ctx.client().recover());
    assert_eq!(ctx.client().state(), EscrowState::Funded);
    assert_eq!(ctx.token_balance(&ctx.contract), AMOUNT);
}

#[test]
fn recover_nothing_to_recover_fails() {
    let ctx = setup(true);
    ctx.initialize();

    must_panic(|| ctx.client().recover());
    assert_eq!(ctx.client().state(), EscrowState::Created);
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

    ctx.env.ledger().set_timestamp(FUNDED_AT + SUBMISSION_PERIOD);

    must_panic(|| ctx.client().submit_evidence(&evidence_hash(&ctx.env)));
    assert_eq!(ctx.client().state(), EscrowState::Funded);
}

#[test]
fn attest_from_wrong_state_is_rejected() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.mint_and_fund();

    must_panic(|| ctx.client().attest(&AttestationOutcome::Pass, &report_hash(&ctx.env)));
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
        args: (
            AttestationOutcome::Pass,
            report_hash(&ctx.env),
        )
            .into_val(&ctx.env),
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
        args: (
            AttestationOutcome::Pass,
            report_hash(&ctx.env),
        )
            .into_val(&ctx.env),
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

#[test]
fn recover_by_supplier_rejected_in_created() {
    let ctx = setup(false);
    let init = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "initialize",
        args: (ctx.config(),).into_val(&ctx.env),
        sub_invokes: &[],
    };
    let stray_in = transfer_sub(
        &ctx.token,
        ctx.buyer.clone(),
        ctx.contract.clone(),
        STRAY,
        &ctx.env,
    );
    let recover = MockAuthInvoke {
        contract: &ctx.contract,
        fn_name: "recover",
        args: soroban_sdk::Vec::new(&ctx.env),
        sub_invokes: &[],
    };
    let mint = MockAuthInvoke {
        contract: &ctx.token,
        fn_name: "mint",
        args: (ctx.buyer.clone(), STRAY).into_val(&ctx.env),
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
            invoke: &stray_in,
        },
        MockAuth {
            address: &ctx.supplier,
            invoke: &recover,
        },
    ]);

    ctx.initialize();
    ctx.sac().mint(&ctx.buyer, &STRAY);
    TokenClient::new(&ctx.env, &ctx.token).transfer(&ctx.buyer, &ctx.contract, &STRAY);
    // El supplier autoriza, pero recuperar es solo del buyer: el saldo queda.
    must_panic(|| ctx.client().recover());
    assert_eq!(ctx.client().state(), EscrowState::Created);
    assert_eq!(ctx.token_balance(&ctx.contract), STRAY);
}