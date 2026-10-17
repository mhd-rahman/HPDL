//! Emit a single valid encoded PIR envelope to stdout (binary).
//!
//! Used once to seed the fuzz corpus with a well-formed input so the fuzzer
//! explores mutations of a real envelope, not just random noise. It writes the
//! bytes directly to the path given as the first argument (binary-safe; avoids
//! shell pipe mangling):
//!     cargo run -p hpdl-pir --example emit_seed -- fuzz/seeds/valid_envelope.bin

#![allow(clippy::expect_used)]

use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, PlanetDefinition, TraitSet};
use hpdl_pir::encode_planet_definition;

fn main() {
    let out_path = std::env::args()
        .nth(1)
        .expect("usage: emit_seed <output-path>");
    let body = CelestialBody::new(
        BodyId::new("earth").expect("valid id"),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .expect("valid body");
    let planet = PlanetDefinition::new("1.0", body).expect("valid definition");
    let envelope = encode_planet_definition(&planet).expect("encode");
    let bytes = bincode::serialize(&envelope).expect("serialize envelope");
    std::fs::write(&out_path, &bytes).expect("write seed bytes");
    eprintln!("wrote {} bytes to {out_path}", bytes.len());
}
