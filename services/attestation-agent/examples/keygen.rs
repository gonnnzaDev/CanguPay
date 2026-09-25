//! Utilidad de desarrollo: genera keypairs ed25519 para pruebas en testnet.
//!
//! No forma parte del binario publicado. Escribe los secretos en el archivo que se
//! le pase, fuera del repositorio.
use ed25519_dalek::SigningKey;
use std::io::Write;

fn main() {
    let out = std::env::args()
        .nth(1)
        .expect("uso: keygen <archivo> <nombre>...");
    let names: Vec<String> = std::env::args().skip(2).collect();
    let mut file = std::fs::File::create(&out).expect("no se pudo crear el archivo");
    writeln!(
        file,
        "# claves de testnet generadas para la prueba de P0-07"
    )
    .unwrap();
    for name in &names {
        let seed: [u8; 32] = rand_bytes();
        let key = SigningKey::from_bytes(&seed);
        let address = format!(
            "{}",
            stellar_strkey::ed25519::PublicKey(key.verifying_key().to_bytes())
        );
        let sk = stellar_strkey::ed25519::PrivateKey::from_payload(&seed).expect("seed no valida");
        // La biblioteca marca la clave como `Unredacted` para que no se imprima por
        // accidente; el Display sale de ahi explicitamente.
        let secret = format!("{}", stellar_strkey::Unredacted(&sk));
        writeln!(file, "{name}={address}:{secret}").unwrap();
        println!("{name}={address}");
    }
}

fn rand_bytes() -> [u8; 32] {
    use std::io::Read;
    let mut buf = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .expect("no se pudo abrir /dev/urandom")
        .read_exact(&mut buf)
        .expect("no se pudo leer entropia");
    buf
}
