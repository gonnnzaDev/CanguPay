/**
 * Traduce los errores del contrato a mensajes que una persona pueda entender.
 *
 * El RPC devuelve el error del contrato con la traza completa del evento
 * diagnostico: "Error(Contract, #17) Event log (newest first): 0: [Diagnostic
 * Event] contract:CC..., topics:[error, Error(Contract, #17)], data:...". Eso
 * no se le enseña a un usuario. Se le dice que paso y por que, y el detalle
 * crudo se deja en la consola.
 */

/** Codigos de Error en contracts/conditional-payment/src/types.rs. */
const ERRORES_CONTRATO: Record<number, string> = {
  1: "Este escrow ya fue inicializado.",
  2: "El contrato todavia no esta inicializado.",
  3: "El importe del escrow no es valido.",
  4: "El plazo de entrega no es valido.",
  5: "El escrow no esta en un estado que admita esta operacion.",
  7: "Vencio el plazo de entrega de la evidencia.",
  8: "Esta operacion ya fue aprobada.",
  10: "El escrow todavia no se puede finalizar: falta que venza un plazo.",
  11: "El plazo de atestacion no es valido.",
  12: "El plazo de objecion no es valido.",
  13: "El plazo de correccion no es valido.",
  14: "El plazo de resolucion no es valido.",
  15: "El resultado de fallback configurado no es valido.",
  16: "Vencio el plazo de atestacion del motor.",
  17: "Vencio el plazo de objecion del comprador.",
  18: "El numero maximo de correcciones no es valido.",
  19: "La operacion desbordo el rango permitido.",
};

/** Codigos cuyo remedio es finalizar por la matriz de vencimientos. */
const VENCIMIENTOS = new Set([7, 16, 17]);

/** Extrae el codigo numerico de un error del contrato, venga como venga. */
export function codigoDeErrorContrato(error: unknown): number | null {
  const texto =
    error instanceof Error ? `${error.message} ${error.name}` : String(error ?? "");
  const directo = texto.match(/Error\(Contract,\s*#(\d+)\)/i);
  if (directo) return Number(directo[1]);
  // El RPC a veces lo devuelve solo, sin la envoltura HostError.
  const suelto = texto.match(/\b#(\d+)\b/);
  if (suelto && /contract/i.test(texto)) return Number(suelto[1]);
  return null;
}

/** Descripcion de una sola linea, sin la traza del evento diagnostico. */
export function mensajeDeErrorContrato(error: unknown): string | null {
  const codigo = codigoDeErrorContrato(error);
  if (codigo === null) return null;
  return ERRORES_CONTRATO[codigo] ?? `El contrato rechazo la operacion (error #${codigo}).`;
}

/** true si el error es un plazo vencido: la unica salida es finalizar. */
export function esPlazoVencido(error: unknown): boolean {
  const codigo = codigoDeErrorContrato(error);
  return codigo !== null && VENCIMIENTOS.has(codigo);
}

/**
 * Mensaje completo para la interfaz. Si el error es del contrato se traduce; si
 * no, se devuelve el texto original. El detalle crudo se registra en consola
 * para que quede en los logs sin aparecer en pantalla.
 */
export function errorLegible(error: unknown): string {
  const original = error instanceof Error ? error.message : String(error ?? "");
  const codigo = codigoDeErrorContrato(error);
  if (codigo === null) return original || "No se pudo completar la operacion.";

  if (original) {
    // Los logs se quedan con el texto entero, la interfaz con la traduccion.
    console.error(`CanguPay: error del contrato #${codigo}`, original);
  }
  const base = ERRORES_CONTRATO[codigo] ?? `El contrato rechazo la operacion (error #${codigo}).`;
  if (VENCIMIENTOS.has(codigo)) {
    return `${base} La operacion ya no es posible: corresponde finalizar el escrow segun la matriz de vencimientos.`;
  }
  return base;
}
