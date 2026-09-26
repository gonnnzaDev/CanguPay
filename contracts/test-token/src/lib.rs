#![no_std]

//! Token de prueba con la interfaz de Stellar Asset Contract.
//!
//! **No es CPUSD ni un token de produccion.** Existe unicamente para poder ejercitar
//! `conditional-payment` en testnet, donde no hay un SAC desplegado para una divisa de
//! prueba. Implementa el minimo que el escrow usa (`transfer` y `balance`) con la misma
//! semantica de autorizacion que el SAC: quien mueve fondos firma, y las transferencias
//! internas del propio contrato funcionan porque una direccion de contrato se
//! autoautoriza.
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    /// Saldo de una cuenta.
    Balance(Address),
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenError {
    /// Saldo insuficiente.
    InsufficientBalance = 1,
    /// Importe no positivo.
    InvalidAmount = 2,
    /// Solo el administrador puede acuñar.
    NotAdmin = 3,
    /// El token ya fue inicializado.
    AlreadyInitialized = 4,
}

#[contract]
pub struct TestToken;

#[contractimpl]
impl TestToken {
    /// Crea el token con `admin` como unica cuenta que puede acuñar.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            return;
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Acuña `amount` a `to`. Requiere firma del administrador y de `to`.
    pub fn mint(env: Env, to: Address, amount: i128) {
        to.require_auth();
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        let current = Self::balance(env.clone(), to.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &(current + amount));
    }

    /// Mueve `amount` de `from` a `to`.
    ///
    /// Firma `from`. Cuando `from` es el propio escrow, la autorizacion es automatica
    /// porque una direccion de contrato se autoautoriza: eso es lo que permite pagar al
    /// supplier o reembolsar al buyer desde `finalize()` y `approve()`.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        Self::move_funds(env, from, to, amount);
    }

    /// Saldo de `address`.
    pub fn balance(env: Env, address: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(address))
            .unwrap_or(0)
    }

    /// Administrador del token.
    pub fn admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    /// Logica de la transferencia sin autorizacion, para reutilizarla.
    pub fn move_funds(env: Env, from: Address, to: Address, amount: i128) {
        if amount <= 0 {
            env.panic_with_error(TokenError::InvalidAmount);
        }
        let from_balance = Self::balance(env.clone(), from.clone());
        if from_balance < amount {
            env.panic_with_error(TokenError::InsufficientBalance);
        }
        let to_balance = Self::balance(env.clone(), to.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &(from_balance - amount));
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &(to_balance + amount));
    }
}
