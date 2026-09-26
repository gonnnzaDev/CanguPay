# syntax=docker/dockerfile:1.7
# Frontend de CanguPay.
#
# Next.js 16 con pnpm. La imagen final solo lleva el bundle standalone, el
# estatico y los ficheros publicos: no node_modules completo ni el codigo fuente.
#
# OJO: las variables NEXT_PUBLIC_* se INLINEAN al compilar, no se leen al
# arrancar. Cambiar una en el servidor sin recompilar no hace nada. Por eso
# van como ARG de build y no solo como ENV.

# ---------- deps: instalar dependencias con el lockfile ----------
FROM node:24-bookworm-slim AS deps
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@11.9.0 --activate

# node-linker=hoisted: pnpm symlinka por defecto y el trazado de dependencias
# de Next standalone no resuelve bien esa estructura. Con hoisted la imagen
# final trae un node_modules plano.
ENV NPM_CONFIG_NODE_LINKER=hoisted

COPY apps/web/package.json apps/web/pnpm-lock.yaml apps/web/pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile

# ---------- builder: compilar ----------
FROM node:24-bookworm-slim AS builder
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@11.9.0 --activate
ENV NPM_CONFIG_NODE_LINKER=hoisted
ENV NEXT_TELEMETRY_DISABLED=1

COPY --from=deps /app/node_modules ./node_modules
COPY apps/web/ ./

# Valores por defecto pensados para testnet. Sin contrato configurado la web cae
# en modo vista previa, que es lo correcto para una demo sin despliegue.
ARG NEXT_PUBLIC_SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
ARG NEXT_PUBLIC_ESCROW_CONTRACT_ID=""
ENV NEXT_PUBLIC_SOROBAN_RPC_URL=$NEXT_PUBLIC_SOROBAN_RPC_URL
ENV NEXT_PUBLIC_ESCROW_CONTRACT_ID=$NEXT_PUBLIC_ESCROW_CONTRACT_ID

# El typecheck lo corre el propio next build; si falla, la imagen no se construye.
RUN pnpm build

# ---------- runner: imagen minima ----------
FROM node:24-bookworm-slim AS runner
WORKDIR /app

ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV PORT=3000
ENV HOSTNAME=0.0.0.0

# Usuario sin privilegios. next/image y el runtime no necesitan root.
RUN groupadd --system --gid 1001 nodejs \
 && useradd  --system --uid 1001 --gid nodejs nextjs

# El standalone incluye un server.js con el runtime de Node.
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static
COPY --from=builder --chown=nextjs:nodejs /app/public ./public

USER nextjs
EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD node -e "fetch('http://127.0.0.1:3000/').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"

CMD ["node", "server.js"]
