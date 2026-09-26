// Runner de flujo completo, con la wallet duplicada por el protocolo postMessage
// que usa de verdad @stellar/freighter-api.
//
// Comprueba: lectura on-chain, derivado de rol por cuenta, el estado del
// banner, la red equivocada, la ausencia de extension y, sobre todo, que al
// pulsar una accion la web pida una firma de verdad a la wallet y que se
// comporte bien cuando el usuario la rechaza.
import { spawn } from "node:child_process";
import { setTimeout as sleep } from "node:timers/promises";
import { chromium } from "playwright";
import * as S from "@stellar/stellar-sdk";

const CONTRATO =
  process.env.ESCROW_CONTRACT ??
  "CCGZQCVPZPJLTTD4NSCL7MZWFH6PKFAAZCSGXCPZH7ZEB7T56L2WJ2OF";
const RPC = "https://soroban-testnet.stellar.org";
const PASSPHRASE = "Test SDF Network ; September 2015";
const PUERTO = Number(process.env.PUERTO ?? 3497);
const BASE = `http://127.0.0.1:${PUERTO}`;

const ROL_ES = {
  buyer: "Comprador",
  supplier: "Proveedor",
  resolver: "Árbitro",
  observer: "Observador",
};

const hallazgos = [];
let escenas = 0;
let correctas = 0;

function registrar(escena, ok, detalle) {
  escenas++;
  if (ok) correctas++;
  else hallazgos.push(`${escena}: ${detalle}`);
}

// El doble de la extension. `firmar` decide si firma, rechaza o falla.
async function instalarWallet(page, { direccion, red = "TESTNET", firmar = "firmar" }) {
  await page.exposeFunction("__cgFirmar", async (xdr) => {
    if (firmar === "rechazar") throw new Error("El usuario rechazo la firma");
    const tx = S.TransactionBuilder.fromXDR(xdr, S.Networks.TESTNET);
    return S.Keypair.random().signTransaction(tx).toXDR();
  });

  await page.addInitScript(
    ({ direccion, red, passphrase, firmar }) => {
      window.__cg = { peticiones: [], firmas: [], errores: [] };
      window.freighter = { isConnected: true };

      window.addEventListener("message", (ev) => {
        if (ev.source !== window) return;
        const d = ev.data;
        if (!d || d.source !== "FREIGHTER_EXTERNAL_MSG_REQUEST") return;
        window.__cg.peticiones.push(d.type);

        const esTestnet = red === "TESTNET";
        const net = {
          network: red,
          networkPassphrase: esTestnet ? passphrase : "Another Future ; September 2015",
          networkUrl: esTestnet
            ? "https://soroban-testnet.stellar.org"
            : "https://rpc-futurenet.stellar.org:443",
          sorobanRpcUrl: esTestnet
            ? "https://soroban-testnet.stellar.org"
            : "https://rpc-futurenet.stellar.org:443",
        };
        const r = {
          source: "FREIGHTER_EXTERNAL_MSG_RESPONSE",
          messagedId: d.messageId,
          networkDetails: net,
          apiError: "",
          ...net,
        };
        switch (d.type) {
          case "REQUEST_PUBLIC_KEY":
            r.isConnected = true;
            r.publicKey = direccion;
            break;
          case "REQUEST_ACCESS":
            r.publicKey = direccion;
            break;
          case "SUBMIT_TRANSACTION": {
            window.__cg.firmas.push(d.transactionXdr ?? null);
            if (firmar === "rechazar") {
              r.error = { code: -1, message: "El usuario rechazo la firma" };
            } else {
              window.__cangupayFirmar(d.transactionXdr).then(
                (xdr) => {
                  r.signedTransaction = xdr;
                  r.signerAddress = direccion;
                  window.postMessage(r, window.location.origin);
                },
                (e) => {
                  window.__cg.errores.push(String(e.message ?? e));
                  r.error = { code: -1, message: String(e.message ?? e) };
                  window.postMessage(r, window.location.origin);
                },
              );
              return;
            }
            break;
          }
          default:
            break;
        }
        window.postMessage(r, window.location.origin);
      });
      localStorage.setItem("cangupay_freighter_connected", "true");
    },
    { direccion, red, passphrase: PASSPHRASE, firmar },
  );
}

async function participantes() {
  const srv = new S.rpc.Server(RPC);
  const { result: c } = await srv.queryContract(CONTRATO, "config");
  return { buyer: c.buyer, supplier: c.supplier, resolver: c.resolver };
}

// Esperar a que la lectura termine: el esqueleto desaparece y la cabecera se
// estabiliza. Sin esto se muestrea a medio cargar.
async function estabilizar(page) {
  // El banner "Contrato sincronizado" solo se pinta cuando el snapshot ya llego.
  // Antes de eso la web no sabe si el plazo vencio y, por diseno, no ofrece
  // ninguna accion: esperar por el esqueleto solo no alcanza.
  const listo = () =>
    page
      .evaluate(() => {
        if (document.querySelector(".animate-pulse")) return false;
        const b = [...document.querySelectorAll("[role=status]")].map((e) => e.innerText);
        return b.some((t) => /sincronizado/i.test(t));
      })
      .catch(() => false);

  for (let i = 0; i < 40; i++) {
    if (await listo()) {
      await sleep(500);
      if (await listo()) return true;
    }
    await sleep(500);
  }
  return false;
}

async function estado(page) {
  return page
    .evaluate(() => {
      const cab = (document.querySelector("header")?.textContent ?? "").replace(/\s+/g, " ");
      const m = cab.match(/Rol:\s*([^|]*?)(?:\s*XLM\b|\s*(?:No conectado|Conectado|Conectando|Conectar Wallet|Copiar|Desconectar|ES|EN)|$)/);
      const titulos = [...document.querySelectorAll("header [title]")].map((e) =>
        e.getAttribute("title"),
      );
      const banners = [...document.querySelectorAll("[role=status]")]
        .map((e) => e.innerText.replace(/\s+/g, " ").trim())
        .filter((t) => /contrato|lectura|sincronizado/i.test(t));
      const cuerpo = document.body.innerText.replace(/\s+/g, " ");
      return {
        rol: m ? m[1].trim() : null,
        conectado: titulos.some((t) => t && /Conectado|Connected/i.test(t)),
        bloqueada: titulos.some((t) => t && /bloqueada|blocked/i.test(t)),
        red: titulos.find((t) => t && /^(Red|Network|Bloqueado|Blocked):/.test(t)) ?? null,
        banners,
        tienePendiente: cuerpo.includes("Pendiente on-chain"),
        cargando: !!document.querySelector(".animate-pulse"),
      };
    })
    .catch(() => null);
}

async function botonesAccion(page) {
  return (await page
    .locator("main button")
    .allTextContents()
    .catch(() => []))
    .map((b) => b.trim())
    .filter(Boolean);
}

async function abrir(page, opts) {
  const ruido = [];
  page.on("pageerror", (e) => ruido.push(`pageerror: ${String(e).slice(0, 140)}`));
  page.on("console", (m) => {
    // El 404 de horizon de una cuenta que no existe es esperado.
    if (m.type() === "error" && !/404|Failed to load resource/.test(m.text()))
      ruido.push(`consola: ${m.text().slice(0, 140)}`);
  });
  if (opts.direccion) await instalarWallet(page, opts);
  else await page.addInitScript(() => localStorage.setItem("cangupay_freighter_connected", "false"));
  await page.goto(BASE, { waitUntil: "domcontentloaded" });
  if (!(await estabilizar(page))) ruido.push("el snapshot no llego: la web no llego al estado sincronizado");
  return ruido;
}

async function main() {
  const vueltas = Number(process.env.VUELTAS ?? 20);
  const p = await participantes();
  const cuentas = [
    { nombre: "comprador", direccion: p.buyer, esperado: "buyer" },
    { nombre: "proveedor", direccion: p.supplier, esperado: "supplier" },
    { nombre: "arbitro", direccion: p.resolver, esperado: "resolver" },
    { nombre: "ajeno", direccion: S.Keypair.random().publicKey(), esperado: "observer" },
  ];

  console.log(`  contrato: ${CONTRATO}`);
  console.log(`  vueltas: ${vueltas}  cuentas: ${cuentas.length}`);

  const srv = spawn("npx", ["next", "start", "-p", String(PUERTO)], {
    env: { ...process.env, NEXT_PUBLIC_ESCROW_CONTRACT_ID: CONTRATO },
    stdio: "ignore",
  });
  for (let i = 0; i < 120; i++) {
    try {
      const r = await fetch(BASE, { signal: AbortSignal.timeout(1500) });
      if (r.ok || r.status === 404) break;
    } catch {}
    await sleep(500);
  }

  const navegador = await chromium.launch();
  try {
    // ---------- una pasada por cuenta, para ver la foto ----------
    for (const c of cuentas) {
      const ctx = await navegador.newContext();
      const page = await ctx.newPage();
      try {
        const ruido = await abrir(page, { direccion: c.direccion });
        const e = await estado(page);
        const acciones = await botonesAccion(page);
        console.log(
          `    ${c.nombre.padEnd(10)} rol=${String(e?.rol).padEnd(10)} conectado=${e?.conectado} red=${(e?.red ?? "").slice(0, 22)}`,
        );
        console.log(`      banner: ${JSON.stringify(e?.banners?.[0]?.slice(0, 90))}`);
        console.log(`      acciones: ${acciones.filter((a) => !/Actualizar|Copiar/.test(a)).length} ${JSON.stringify(acciones.filter((a) => !/Actualizar|Copiar/.test(a)).slice(0, 3))}`);
        console.log(`      "Pendiente on-chain" en pantalla: ${e?.tienePendiente}`);
        if (ruido.length) console.log(`      ruido: ${ruido[0]}`);
      } catch (e) {
        console.log(`    ${c.nombre}: EXCEPCION ${e.message.slice(0, 100)}`);
      } finally {
        await ctx.close();
      }
    }

    // ---------- el comprador pulsa una accion de verdad ----------
    console.log("\n  -- accion real: finalizar (operacion valida) --");
    {
      const ctx = await navegador.newContext();
      const page = await ctx.newPage();
      try {
        await abrir(page, { direccion: p.buyer, firmar: "firmar" });
        // Con el plazo vencido, Aprobar y Disputar ya no deben existir: solo
        // queda Finalizar, que si es una operacion valida ahora mismo.
        for (const etiqueta of ["Aprobar", "Disputar"]) {
          const prohibido = page.getByRole("button", { name: new RegExp(etiqueta, "i") }).first();
          const aparece = await prohibido.count();
          registrar(
            `no ofrece "${etiqueta}" con el plazo vencido`,
            aparece === 0,
            `sigue ofreciendo "${etiqueta}" aunque el contrato la rechaza`,
          );
        }

        const btn = page.getByRole("button", { name: /Finalizar/i }).first();
        const hay = await btn.count();
        if (!hay) {
          registrar("accion del comprador", false, "no aparece el boton Finalizar");
        } else {
          const deshab = await btn.isDisabled();
          registrar(
            "boton Finalizar habilitado",
            !deshab,
            `el boton Finalizar sigue deshabilitado (deshabilitado=${deshab})`,
          );
          const antes = await page.evaluate(() => window.__cg.firmas.length);
          await btn.click({ timeout: 8000 }).catch((e) => console.log(`    click: ${String(e).slice(0, 70)}`));
          await sleep(9000);
          const cg = await page.evaluate(() => window.__cg);
          const pidio = cg.firmas.length > antes;
          registrar(
            "la accion pide firma a la wallet",
            pidio,
            pidio ? "" : `la wallet no recibio ninguna peticion de firma (peticiones: ${JSON.stringify(cg.peticiones)})`,
          );
          if (pidio) {
            const xdr = cg.firmas[cg.firmas.length - 1];
            try {
              const tx = S.TransactionBuilder.fromXDR(xdr, S.Networks.TESTNET);
              const ops = tx.operations ?? [];
              const texto = JSON.stringify(tx.toXDR());
              registrar(
                "XDR de la accion bien formado",
                ops.length > 0,
                `la transaccion no trae operaciones: ${xdr.slice(0, 60)}`,
              );
              console.log(`      XDR: ${xdr.length} chars, ${ops.length} operacion(es), ${texto.length} bytes`);
              console.log(`      errores del firmante: ${JSON.stringify(cg.errores.slice(0, 2))}`);
            } catch (e) {
              registrar("XDR de la accion bien formado", false, `XDR ilegible: ${e.message.slice(0, 90)}`);
            }
          }
          const cuerpo = await page.evaluate(() => document.body.innerText.replace(/\s+/g, " "));
          const colgado = await page.evaluate(() => !!document.querySelector(".animate-pulse"));
          registrar(
            "la interfaz se recupera tras la accion",
            !colgado,
            "se queda en el esqueleto de carga tras pulsar la accion",
          );
          // Si el unico error que se ve es de plazo, la simulacion fallo antes de
          // firmar: eso seria un defecto. Con la firma de mentira lo esperable es
          // un rechazo de autenticacion.
          const textoError = cuerpo.slice(cuerpo.search(/Error|error|rechaz/i));
          const esPlazo = /plazo de (objecion|atestacion)|#17|#16|#7/.test(cuerpo);
          registrar(
            "el error no es de plazo sino de firma",
            !esPlazo,
            `la accion se detuvo por plazo vencido en vez de llegar a firmar: ${textoError.slice(0, 120)}`,
          );
          registrar(
            "el error no filtra la traza del contrato",
            !/Event log|Diagnostic Event|fail_with_error/.test(cuerpo),
            "vuelca la traza del evento diagnostico en pantalla",
          );
          console.log(`      cuerpo tras la accion: ${JSON.stringify(cuerpo.slice(0, 260))}`);
        }
      } finally {
        await ctx.close();
      }
    }

    // ---------- el usuario rechaza la firma ----------
    console.log("\n  -- el usuario rechaza la firma --");
    {
      const ctx = await navegador.newContext();
      const page = await ctx.newPage();
      try {
        await abrir(page, { direccion: p.buyer, firmar: "rechazar" });
        const btn = page.getByRole("button", { name: /Finalizar/i }).first();
        if (await btn.count()) {
          await btn.click({ timeout: 8000 }).catch(() => {});
          await sleep(6000);
          const colgado = await page.evaluate(() => !!document.querySelector(".animate-pulse"));
          const cuerpo = await page.evaluate(() => document.body.innerText.replace(/\s+/g, " "));
          const cg = await page.evaluate(() => window.__cg);
          registrar("rechazar no cuelga la interfaz", !colgado, "queda en el esqueleto tras el rechazo");
          console.log(`      firmas pedidas: ${cg.firmas.length}  colgado: ${colgado}`);
          const avisa = /rechaz|error|no pudo|fall/i.test(cuerpo);
          console.log(`      avisa del rechazo al usuario: ${avisa}`);
          console.log(`      cuerpo: ${JSON.stringify(cuerpo.slice(0, 200))}`);
        }
      } finally {
        await ctx.close();
      }
    }

    // ---------- red equivocada ----------
    console.log("\n  -- red equivocada --");
    {
      const ctx = await navegador.newContext();
      const page = await ctx.newPage();
      try {
        await abrir(page, { direccion: p.buyer, red: "FUTURENET" });
        const e = await estado(page);
        const bien = /FUTURENET/.test(e?.red ?? "") && e?.bloqueada;
        registrar("red FUTURENET bloquea la firma", bien, `red=${e?.red} bloqueada=${e?.bloqueada}`);
        console.log(`      red=${e?.red}  bloqueada=${e?.bloqueada}`);
      } finally {
        await ctx.close();
      }
    }

    // ---------- sin extension ----------
    {
      const ctx = await navegador.newContext();
      const page = await ctx.newPage();
      try {
        await abrir(page, {});
        const btn = page.getByRole("button", { name: /Conectar Wallet|Connect Wallet/i }).first();
        if (await btn.count()) {
          await btn.click({ timeout: 5000 }).catch(() => {});
          await sleep(6000);
        }
        const cab = await page.evaluate(() =>
          (document.querySelector("header")?.textContent ?? "").replace(/\s+/g, " "),
        );
        const colgado = /Conectando/.test(cab);
        registrar("sin extension no se queda colgado", !colgado, `cabecera: ${cab.slice(0, 80)}`);
        console.log(`      ${colgado ? "COLGADO" : "se recupera"}: ${JSON.stringify(cab.slice(0, 80))}`);
      } finally {
        await ctx.close();
      }
    }

    // ---------- repeticiones ----------
    console.log(`\n  -- ${vueltas} vueltas de estabilidad --`);
    for (let v = 1; v <= vueltas; v++) {
      for (const c of cuentas) {
        const ctx = await navegador.newContext();
        const page = await ctx.newPage();
        try {
          const ruido = await abrir(page, { direccion: c.direccion });
          const e = await estado(page);
          const attendu = ROL_ES[c.esperado];
          if (e?.rol !== attendu)
            registrar(`${c.nombre} v${v}`, false, `rol ${JSON.stringify(e?.rol)} != ${attendu}`);
          else if (!e?.conectado)
            registrar(`${c.nombre} v${v}`, false, "no quedo conectado");
          else if (ruido.length)
            registrar(`${c.nombre} v${v}`, false, `ruido: ${ruido[0].slice(0, 90)}`);
          else registrar(`${c.nombre} v${v}`, true);
        } finally {
          await ctx.close();
        }
      }
      if (v % 5 === 0) console.log(`    ${v}/${vueltas} vueltas`);
    }
  } finally {
    await navegador.close();
    srv.kill("SIGTERM");
  }

  console.log(`\n  escenas: ${escenas}  correctas: ${correctas}  con problemas: ${hallazgos.length}`);
  const unicos = [...new Set(hallazgos.map((h) => h.replace(/ v\d+$/, "")))];
  for (const h of unicos.slice(0, 20)) console.log(`    - ${h}`);
  if (unicos.length) process.exitCode = 1;
}

main().catch((e) => {
  console.error("  el runner fallo:", e.message);
  process.exit(1);
});
