//! Pruebas de aceptación P0-02.
//!
//! Cubren el camino feliz `CREATED → FUNDED → EVIDENCE_SUBMITTED →
//! ATTESTED_PASS → RELEASED`, el de fallo (`ATTESTED_FAIL` retiene fondos) y
//! `CREATED → CANCELLED`, más las reglas de negocio: atomicidad del fondeo,
//! deadline de evidencia, exclusividad de roles (auth) y doble aprobación.

use super::*;
extern crate std;

use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::Event as _;
use soroban_sdk::{token::StellarAssetClient, xdr, Address, BytesN};

const AMOUNT: i128 = 10_000_000_000;
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
    let token = env.register_stellar_asset_contract_v2(token_admin).address();
    let buyer = Address::generate(&env);
    let supplier = Address::generate(&env);
    let engine = Address::generate(&env);
    let contract = env.register(ConditionalPayment, ());
    TestContext {
        env,
        contract,
        token,
        buyer,
        supplier,
        engine,
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

    fn fund_contract(&self) {
        self.sac().mint(&self.buyer, &AMOUNT);
        TokenClient::new(&self.env, &self.token).transfer(&self.buyer, &self.contract, &AMOUNT);
    }

    fn to_attested_pass(&self) {
        self.initialize();
        self.fund_contract();
        self.client().fund();
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

    ctx.fund_contract();
    ctx.client().fund();
    ctx.assert_single_event(Funded {
        funded_at: FUNDED_AT,
        submission_deadline: FUNDED_AT + SUBMISSION_PERIOD,
    }
    .to_xdr(&ctx.env, &ctx.contract));
    assert_eq!(ctx.client().state(), EscrowState::Funded);

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
    ctx.fund_contract();
    ctx.client().fund();
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
    ctx.fund_contract();
    ctx.client().fund();

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
    assert_eq!(ctx.own_events().len(), 0, "no emite el evento Funded");
}

#[test]
fn fund_with_incorrect_balance_is_atomic() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.sac().mint(&ctx.buyer, &(AMOUNT / 2));
    TokenClient::new(&ctx.env, &ctx.token).transfer(&ctx.buyer, &ctx.contract, &(AMOUNT / 2));

    must_panic(|| ctx.client().fund());
    assert_eq!(ctx.client().state(), EscrowState::Created, "no cambia de estado");
    assert_eq!(ctx.own_events().len(), 0, "no emite el evento Funded");
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
    ctx.fund_contract();
    ctx.client().fund();

    ctx.env.ledger().set_timestamp(FUNDED_AT + SUBMISSION_PERIOD);

    must_panic(|| ctx.client().submit_evidence(&evidence_hash(&ctx.env)));
    assert_eq!(ctx.client().state(), EscrowState::Funded);
}

#[test]
fn attest_from_wrong_state_is_rejected() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.fund_contract();
    ctx.client().fund();

    must_panic(|| ctx.client().attest(&AttestationOutcome::Pass, &report_hash(&ctx.env)));
    assert_eq!(ctx.client().state(), EscrowState::Funded);
}

#[test]
fn approve_from_attested_fail_is_rejected_and_retains_funds() {
    let ctx = setup(true);
    ctx.initialize();
    ctx.fund_contract();
    ctx.client().fund();
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
fn actions_require_the_correct_party_auth() {
    {
        let ctx = setup(false);
        ctx.initialize();
        must_panic(|| ctx.client().cancel());
        assert_eq!(ctx.client().state(), EscrowState::Created);
    }
    {
        let ctx = setup(false);
        ctx.initialize();
        must_panic(|| ctx.client().fund());
        assert_eq!(ctx.client().state(), EscrowState::Created);
    }
    {
        let ctx = setup(false);
        ctx.initialize();
        must_panic(|| ctx.client().submit_evidence(&evidence_hash(&ctx.env)));
        assert_eq!(ctx.client().state(), EscrowState::Created);
    }
    {
        let ctx = setup(false);
        ctx.initialize();
        must_panic(|| ctx.client().attest(&AttestationOutcome::Pass, &report_hash(&ctx.env)));
        assert_eq!(ctx.client().state(), EscrowState::Created);
    }
    {
        let ctx = setup(false);
        ctx.initialize();
        must_panic(|| ctx.client().approve());
        assert_eq!(ctx.client().state(), EscrowState::Created);
    }
}