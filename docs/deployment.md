# Despliegue con Docker

Todo lo que hay que levantar en local y en un servidor, y lo que hay que saber
antes de intentarlo.

## Resumen

| Que | Imagen | Fichero |
| --- | --- | --- |
| Frontend | `cangupay/web` | `deploy/docker/web.Dockerfile` |
| Agente (motor y keeper) | `cangupay/agent` | `deploy/docker/agent.Dockerfile` |
| Contratos Soroban | `cangupay/contracts` | `deploy/docker/contracts.Dockerfile` |
| Bateria de pruebas | `cangupay/tests` | `deploy/docker/tests.Dockerfile` |

| Pila | Fichero |
| --- | --- |
| Local, con perfiles | `deploy/compose.local.yml` |
| Servidor | `deploy/compose.server.yml` |

| Script | Para que |
| --- | --- |
| `scripts/docker/build.sh` | Compilar imagenes |
| `scripts/docker/test.sh` | Correr la bateria completa |
| `scripts/docker/up.sh` | Levantar web, nodo local, keeper o servidor |
| `scripts/docker/smoke.sh` | Comprobar que lo levantado funciona de verdad |

## Antes de nada

```bash
cp deploy/.env.example .env
```

El `.env` esta en `.gitignore`. Las claves van ahi o en el gestor de secretos
del servidor, nunca en el repo.

## Local

### La web

```bash
scripts/docker/up.sh web
```

Se publica en `http://localhost:3000`. Sin `NEXT_PUBLIC_ESCROW_CONTRACT_ID`
compilado, la web cae en **modo vista previa** con datos de ejemplo, que es lo
correcto para una demo sin despliegue.

> **Las variables `NEXT_PUBLIC_*` se compilan dentro de la imagen.** Next.js las
> inlinea durante el build, no las lee al arrancar. Cambiar el contrato en el
> `.env` y reiniciar no hace nada: hay que reconstruir. Este es el tropiezo mas
> probable de todo el despliegue, y por eso `smoke.sh` comprueba si la web ha
> caido en vista previa sin querer.

### Las pruebas

```bash
scripts/docker/test.sh              # rust, python y web
scripts/docker/test.sh rust         # contrato y agente
scripts/docker/test.sh python       # reglas del motor y fixtures
scripts/docker/test.sh web          # typecheck, build y tests del frontend
```

`test.sh` devuelve codigo distinto de cero si algo falla, para poder engancharlo
a CI tal cual.

### Los contratos

```bash
scripts/docker/up.sh contracts
```

Compila los dos WASM y deja los hashes en pantalla. Salen en
`target/wasm32v1-none/release/`.

La primera vez instala `stellar-cli` desde cero y **tarda bastante**. Se paga
una sola vez porque va en una etapa de Docker propia; despues queda cacheado.
Para builds seguidos, la via rapida es `scripts/docker/build.sh contracts` con
la cache ya caliente.

El CLI se fija con `STELLAR_CLI_VERSION` (25.2.0 por defecto, la que documenta
el repo). Si cambias el SDK del contrato, comprueba que el CLI genera protocolo
28; un WASM que compila con una version antigua no lo sube la red.

### Nodo Stellar local

```bash
scripts/docker/up.sh localnet
```

Levanta rpc en `:8000` y horizon en `:8080`, con la passphrase
`Local Sandbox Stellar Network ; September 2022`. Para que el resto de la pila
lo use:

```bash
CANGUPA_RPC_URL=http://localhost:8000
CANGUPA_NETWORK="Local Sandbox Stellar Network ; September 2022"
```

Es **solo para desarrollo**. Para lo de verdad, la red de pruebas.

### El keeper

```bash
scripts/docker/up.sh agent
```

Exige en el `.env` el contrato, el bundle, el importe, el token y la clave de
la engine. `up.sh` comprueba que esten antes de arrancar y dice cual falta.

### Comprobar

```bash
scripts/docker/smoke.sh
```

No se queda en comprobar que el puerto responde. Contrasta que la web **no** ha
caido en vista previa, que el contrato responde de verdad a la red y que el
keeper no reinicia en bucle. Un 200 con la web en modo ejemplo no significa
nada, y por eso lo comprueba.

## Servidor

```bash
cp deploy/.env.example .env      # y completalo
scripts/docker/up.sh server
```

Levanta solo la web y el keeper. Los contratos no se levantan: se despliegan
una vez con el CLI y a partir de ahi viven en la cadena. El nodo tampoco: se usa
la red publica.

Decisiones que ya estan tomadas en `compose.server.yml`, para que no haya que
repetirlas:

- **El frontend no lleva la clave del motor.** Solo el keeper la tiene. Si el
  web se expone a internet, el keeper sigue cerrado. Se puede separar en dos
  maquinas sin tocar nada.
- **Solo lectura en el sistema de ficheros**, con `/tmp` en tmpfs. Si alguien
  consigue ejecución en un contenedor, no puede escribir la imagen.
- **Reinicio automatico** y rotacion de logs (10 MB, 3 ficheros).
- **Healthchecks** en ambos. Con `--wait` de compose, el arranque espera a que
  esten sanos.

### Secretos

La clave de la engine llega de dos maneras, y las dos estan en el compose:

```bash
CANGUPA_ENGINE_SECRET=S...                      # en el entorno
CANGUPA_ENGINE_KEYFILE=/run/secrets/engine      # fichero montado
```

Si usas Docker secrets, montalos en el servicio `keeper` y apunta la segunda.

### Redireccion HTTPS

El frontend no termina TLS: pon detras un proxy que lo haga.

```yaml
services:
  caddy:
    image: caddy:2
    ports: ["80:80", "443:443"]
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
    depends_on:
      web:
        condition: service_healthy
```

El contrato Soroban no es web: no necesita proxy, y no debe exponerse.

## Lo que falta y conviene saber

1. **Las imagenes no se han construido nunca.** No hay docker en el entorno donde
   se escribieron. Los Dockerfiles y los compose estan escritos y con la
   sintaxis validada, pero sin construir. El primer `build.sh` es la prueba de
   fuego.

2. **Los flags de `stellar/quickstart` estan sin verificar.** El servicio
   `stellar` del compose local no se ha ejecutado. Si no arranca:
   `docker run --rm stellar/quickstart --help` y ajustar el `command`.

3. **Del `.env.example` del frontend, solo se usan dos variables.** El codigo
   lee unicamente `NEXT_PUBLIC_ESCROW_CONTRACT_ID` y `NEXT_PUBLIC_SOROBAN_RPC_URL`.
   Las otras cinco (`NEXT_PUBLIC_STELLAR_NETWORK`, `..._NETWORK_PASSPHRASE`,
   `..._HORIZON_URL`, `..._CPUSD_CONTRACT_ID`, `..._DEMO_ASSET_CODE`) no las lee
   nadie: son configuracion muerta que invita a cambiar cosas que no hacen nada.

4. **El frontend no puede actuar sobre el contrato.** Las acciones se pintan
   marcadas como `Pendiente on-chain` y estan deshabilitadas. La imagen web
   sirve y lee bien, pero la escritura no existe todavia.

5. **El token de la red de pruebas no es CPUSD**, es un contrato de prueba.
   `CANGUPA_EXPECTED_CURRENCY` lleva la direccion del token (`C...`), no un
   codigo de divisa. Poner `CPUSD` ahi hace que el keeper se rechace a si mismo.

6. **La imagen de tests lleva toolchain completo** a proposito (rust, python,
   node). Es del tamano de un sistema operativo y no debe ir a un servidor.
