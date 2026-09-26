# CanguPay — Web Client & Presentation Foundations (P0-08)

Cliente web de **CanguPay**, plataforma de escrow condicional B2B sobre **Stellar Soroban**.

---

## 🛠️ Stack Tecnológico

- **Framework**: [Next.js 16 (App Router)](https://nextjs.org/) con React 19 y Turbopack.
- **Estilos**: [Tailwind CSS v4](https://tailwindcss.com/) con `@tailwindcss/postcss`.
- **Integración de Billetera**: `@stellar/freighter-api` (v6) para conexión segura con Freighter Wallet.
- **Red y Ledger**: Stellar Testnet explícita con Soroban RPC.
- **Fuentes**: Geist Sans y Geist Mono optimizadas con `next/font`.
- **Arquitectura**: Clean Architecture / Presentational & Container Pattern con componentes modulares desacoplados.

---

## 🛡️ Seguridad y Control de Red (Fail-Closed Network Guard)

Por especificación y política estricta de seguridad en P0:
- **Estado Inicial `UNKNOWN`**: La aplicación no asume ni recurre a Testnet en bloques `catch`. Si Freighter no está disponible o falla la lectura, la red queda como `UNKNOWN` con passphrase `null`.
- **Validación de Passphrase Exacta**: Únicamente se permite la firma cuando la passphrase detectada coincide con exactitud matemática con:
  ```
  Test SDF Network ; September 2015
  ```
- **Fail-Closed en Firma (`signTransactionGuard`)**: Inmediatamente antes de cada intento de firma, el guard vuelve a consultar la red y passphrase directamente desde Freighter. Si la red es `PUBLIC` (Mainnet), `FUTURENET`, `STANDALONE`, `UNKNOWN`, o si la passphrase no coincide exactamente, la firma queda terminantemente bloqueada.
- **Alertas Visuales Contextuales**: La barra de advertencia superior alerta de inmediato si se detecta cualquier red que no sea la Testnet autorizada.

---

## 👥 Determinación Dinámica de Roles (Wallet-Derived Identity)

La dApp es una **única pantalla adaptativa** que ajusta su interfaz y acciones según la dirección pública de la wallet Freighter conectada y los participantes del contrato:

- **Derivación de Rol Pura (`deriveWalletRole`)**:
  - `wallet == config.parties.buyer` $\rightarrow$ **Buyer**
  - `wallet == config.parties.supplier` $\rightarrow$ **Supplier**
  - `wallet == config.parties.resolver` $\rightarrow$ **Resolver**
  - Ninguna coincidencia $\rightarrow$ **Observer** (Modo solo lectura)
  - Sin wallet conectada $\rightarrow$ **Wallet no conectada** (Rol: `—`)
- **Sin Selectores Manuales**: Se eliminó cualquier mecanismo para simular o cambiar perfiles manualmente en la UI. Para pruebas multi-rol en demo se utilizan perfiles de navegador independientes con cuentas Freighter distintas.
- **Separación de Dominios**:
  - `UserRole`: Exclusivo para la identidad del usuario humano (`buyer | supplier | resolver | observer`).
  - `EscrowActor`: Entidades de la máquina de estados Soroban (`buyer | supplier | engine | resolver | none`). Engine no es un rol humano seleccionable.

---

## ⚡ Matriz de Acciones de la Máquina de Estados (P0)

La lógica de interacción respeta el flujo canónico del contrato:
1. **`CREATED`**:
   - Buyer: Fondear depósito (`FUNDED`) o cancelar previo al fondeo (`CANCELLED`).
2. **`FUNDED`**:
   - Supplier: Presentar evidencia documental (`EVIDENCE_SUBMITTED`).
   - Buyer: **No** tiene acción de liberación directa desde este estado.
3. **`EVIDENCE_SUBMITTED`**:
   - Engine: Atestación determinista autónoma (`ATTESTED_PASS` o `ATTESTED_FAIL`).
   - UI: Muestra indicador informativo de espera de atestación del motor.
4. **`ATTESTED_PASS`**:
   - Buyer: Aprobar liberación definitiva (`RELEASED`) u objetar/disputar (`DISPUTED`).
5. **`ATTESTED_FAIL`**:
   - Supplier: Presentar corrección técnica (`EVIDENCE_SUBMITTED`) u objetar/disputar (`DISPUTED`).
6. **`DISPUTED`**:
   - Resolver: Dispone de tres dictámenes posibles:
     - `RELEASE` $\rightarrow$ Liberar a proveedor
     - `REFUND` $\rightarrow$ Reembolsar a comprador
     - `SPLIT` $\rightarrow$ Liquidación dividida
7. **Permissionless `finalize()`**:
   - Disponible para cualquier cuenta cuando el vencimiento contractual sea alcanzado en el ledger. El timestamp local es solo informativo; la autoridad pertenece a `env.ledger().timestamp()`.

---

## 🧪 Datos Mock de Desarrollo (`src/dev/mockEscrow.ts`)

Los datos utilizados para desarrollo visual están completamente aislados en `src/dev/mockEscrow.ts` y etiquetados como **DEVELOPMENT MOCK DATA**. La UI exhibe un banner explícito:
```
DATOS MOCK · SIN LECTURA RPC
```
No se emplean direcciones ni IDs simulados como si fuesen definitivos on-chain.

---

## 🚀 Inicio Rápido

1. **Instalar dependencias**:
   ```bash
   pnpm install
   ```

2. **Configurar variables de entorno**:
   ```bash
   cp .env.example .env.local
   ```

3. **Iniciar servidor de desarrollo**:
   ```bash
   pnpm dev
   ```
   Abre [http://localhost:3000](http://localhost:3000) en tu navegador.

4. **Validar Linter y Compilación**:
   ```bash
   pnpm lint
   pnpm build
   ```

---

## 📋 Variables de Entorno Públicas (`.env.example`)

| Variable | Descripción | Valor por Defecto |
| :--- | :--- | :--- |
| `NEXT_PUBLIC_STELLAR_NETWORK` | Red destino de Stellar | `TESTNET` |
| `NEXT_PUBLIC_STELLAR_NETWORK_PASSPHRASE` | Passphrase oficial requerida | `Test SDF Network ; September 2015` |
| `NEXT_PUBLIC_HORIZON_URL` | API Horizon de Stellar | `https://horizon-testnet.stellar.org` |
| `NEXT_PUBLIC_SOROBAN_RPC_URL` | Endpoint RPC de Soroban Testnet | `https://soroban-testnet.stellar.org` |
| `NEXT_PUBLIC_ESCROW_CONTRACT_ID` | ID de contrato Soroban P0 | *(Placeholder vacío)* |
| `NEXT_PUBLIC_CPUSD_CONTRACT_ID` | ID del contrato SAC CPUSD | *(Placeholder vacío)* |
| `NEXT_PUBLIC_DEMO_ASSET_CODE` | Código del token de liquidación B2B | `CPUSD` |
