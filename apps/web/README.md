# CanguPay — Web Client & Presentation Foundations (P0-08)

Cliente web de **CanguPay**, plataforma de escrow condicional B2B sobre **Stellar Soroban**.

---

## 🛠️ Stack Tecnológico Elegido

- **Framework**: [Next.js 16 (App Router)](https://nextjs.org/) con React 19 y Turbopack.
- **Estilos**: [Tailwind CSS v4](https://tailwindcss.com/) con `@tailwindcss/postcss`.
- **Integración de Billetera**: `@stellar/freighter-api` (v6) para conexión segura con Freighter Wallet.
- **Red y Ledger**: Stellar Testnet explícita con Soroban RPC.
- **Fuentes**: Geist Sans y Geist Mono optimizadas con `next/font`.
- **Arquitectura**: Clean Architecture / Presentational & Container Pattern con componentes modulares desacoplados.

---

## 🛡️ Seguridad y Control de Red (Mainnet Guard)

Por especificación y seguridad en P0:
- **Detección Activa de Red**: El cliente detecta dinámicamente la red configurada en Freighter (`getNetwork` / `getNetworkDetails`).
- **Bloqueo Estricto de Mainnet**: Si se detecta `PUBLIC` (Mainnet), la aplicación activa un banner de advertencia crítico y **bloquea a nivel de código cualquier firma de transacciones (`signTransactionGuard`)**, previniendo pérdidas accidentales de fondos reales.
- **Solo Testnet**: Todas las operaciones y transacciones están autorizadas exclusivamente en `TESTNET` (`Test SDF Network ; September 2015`).

---

## 👥 Multi-party Profiles (P0-08 Simulation)

Para facilitar pruebas y auditoría de los flujos de custodia condicional, la aplicación soporta perfiles de rol independientes:
- **Comprador (Buyer)**: Depositante de fondos y autorizante del escrow (`GBUYER...`).
- **Proveedor (Supplier)**: Beneficiario de la liberación tras atestación documental (`GSUPPLIER...`).
- **Árbitro (Resolver)**: Resolutor neutral de disputas contractuales (`GRESOLVER...`).
- **Motor de Reglas (Engine)**: Verificador automatizado de evidencia documental (`GENGINE...`).
- **Freighter Wallet**: Vinculación directa con la clave pública real conectada desde la extensión.

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
| `NEXT_PUBLIC_STELLAR_NETWORK_PASSPHRASE` | Frase de red para firmas | `Test SDF Network ; September 2015` |
| `NEXT_PUBLIC_HORIZON_URL` | API Horizon de Stellar | `https://horizon-testnet.stellar.org` |
| `NEXT_PUBLIC_SOROBAN_RPC_URL` | Endpoint RPC de Soroban | `https://soroban-testnet.stellar.org` |
| `NEXT_PUBLIC_ESCROW_CONTRACT_ID` | ID de contrato Soroban P0 | Placeholder de contrato |
| `NEXT_PUBLIC_DEMO_ASSET_CODE` | Código del activo demo B2B | `CPUSD` |
