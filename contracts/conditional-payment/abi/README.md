# ABI del contrato

`conditional_payment.json` es la interface del contrato tal y como la emite el
propio WASM, no una copia a mano. Es lo que deben consumir los bindings y lo que
usa el CLI para validar los argumentos antes de firmar.

## Como se regenera

```bash
# Requiere stellar CLI 25.2+; el SDK 28 ya no compila el WASM con cargo pelado.
stellar contract build --package conditional-payment

stellar contract info interface \
    --wasm target/wasm32v1-none/release/conditional_payment.wasm \
    --output json > contracts/conditional-payment/abi/conditional_payment.json
```

## Como se regenera desde un contrato ya desplegado

Sin recompilar, contra el estado real de la cadena:

```bash
stellar contract info interface \
    --contract-id C... \
    --network testnet \
    --output json > contracts/conditional-payment/abi/conditional_payment.json
```

## Que contiene

28 entradas: las 12 funciones publicas, los tipos `EscrowConfig`, `EscrowSnapshot`,
`EscrowState`, `AttestationOutcome`, `FallbackOutcome` y `FinalizeReason`, y los
eventos.

Convenciones de Soroban que hacen falta para invocarlo bien, y que no son
intuitivas:

- Los `struct` se pasan y devuelven como `Map` con claves por nombre, no como un
  `Vec` posicional. Al invocar, el orden de las claves no importa pero los nombres
  si, y todos los campos son obligatorios.
- Los `enum` viajan como `Vec([Symbol("Nombre")])`, no como el indice numerico.
  Pasar el indice hace que el contrato haga trap.
- `snapshot()` no pide `auth` y devuelve de una vez estado, configuracion, hashes y
  todos los plazos, incluido el timestamp del ledger. Es la lectura que debe usar
  un cliente para decidir.
