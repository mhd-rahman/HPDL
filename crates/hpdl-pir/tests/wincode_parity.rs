//! SPIKE: wincode 0.6.3 vs bincode 1.3.3 byte-parity probe.
//!
//! Tracked by `docs/rfcs/0015-migrate-bincode-to-wincode.md` (Proposed).
//!
//! Goal (Option A): before swapping the serializer under `hpdl-pir`'s frozen,
//! hashed canonical-byte contract (PIR v2.0, ADR 0014), prove that wincode's
//! default output is **byte-identical** to bincode's default config for the
//! value shapes the PIR payload actually uses. The real acceptance gate is the
//! frozen golden-vector digests holding over the actual `hpdl-model` types once
//! those derive wincode's schema traits (ADR 0015); this probe only clears the
//! leaf-shape parity question up front.
//!
//! wincode is NOT a drop-in `serde` replacement: it works through its own
//! `SchemaWrite`/`SchemaRead` derive macros, and its parity claim is explicitly
//! conditional ("produces the same bytes as bincode ... provided your schemas
//! and containers match the layout implied by your serde types"). So this probe
//! derives BOTH serde (`Serialize`) and wincode (`SchemaWrite`) on local mirror
//! types that reproduce the byte shapes `hpdl-model` relies on, and asserts the
//! two serializers emit the same bytes. These local types stand in for the real
//! model types so we learn the parity answer WITHOUT first deriving wincode on
//! the entire `hpdl-model` graph (~35 types / 16 files).
//!
//! The shapes covered mirror what PIR encodes:
//! - fixed-width integers including u128 / i128 (orbital `KeplerianElements`),
//! - `String` and `Vec<u8>` (ids, payload),
//! - `Option<T>` (every optional field — bincode writes a presence byte),
//! - `BTreeMap` / `BTreeSet` (`Composition`, `TraitSet`, `MagicBridge`),
//! - a fieldless enum (bincode default u32 discriminant).
//!
//! If any assertion fails, wincode's defaults do NOT match bincode for that
//! shape, and a swap would change canonical bytes => PIR v3.0 territory, not a
//! maintenance swap. If all pass, the swap is byte-safe for these shapes
//! (still requiring the full-graph derive work + the frozen golden-vector
//! digests to hold as the real acceptance gate).

#![allow(clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use wincode::config::DefaultConfig;
use wincode::{SchemaRead, SchemaWrite};

/// Assert wincode and bincode emit identical bytes for a value that derives
/// both frameworks' write traits.
///
/// The `SchemaWrite<DefaultConfig, Src = T>` bound matches what the free
/// `wincode::serialize` requires — i.e. wincode's default (bincode-compatible)
/// configuration, which is exactly the config whose parity we are probing.
fn assert_parity<T>(label: &str, value: &T)
where
    T: Serialize + SchemaWrite<DefaultConfig, Src = T>,
{
    let bin = bincode::serialize(value).unwrap();
    let win = wincode::serialize(value).unwrap();
    assert_eq!(
        win, bin,
        "wincode/bincode byte MISMATCH for `{label}`:\n  bincode = {bin:02x?}\n  wincode = {win:02x?}"
    );
}

#[derive(Serialize, SchemaWrite, SchemaRead)]
struct Ints {
    a_u8: u8,
    b_u32: u32,
    c_u64: u64,
    d_u128: u128,
    e_i64: i64,
    f_i128: i128,
}

#[derive(Serialize, SchemaWrite, SchemaRead)]
struct Strings {
    id: String,
    payload: Vec<u8>,
}

#[derive(Serialize, SchemaWrite, SchemaRead)]
struct Options {
    present: Option<u64>,
    absent: Option<u64>,
    nested: Option<String>,
}

#[derive(Serialize, SchemaWrite, SchemaRead)]
struct Collections {
    map: BTreeMap<String, u32>,
    set: BTreeSet<String>,
}

#[derive(Serialize, SchemaWrite, SchemaRead)]
enum Dimension {
    Physical,
    NativeEcosystem,
    Civilization,
}

#[test]
fn parity_fixed_width_integers() {
    assert_parity(
        "Ints",
        &Ints {
            a_u8: 0xAB,
            b_u32: 0xDEAD_BEEF,
            c_u64: 6_371_000,
            // Large magnitude like a semi-major axis in metres (u128 in PIR).
            d_u128: 149_597_870_700_000,
            e_i64: -88_642,
            f_i128: -170_141_183_460_469_231,
        },
    );
}

#[test]
fn parity_string_and_bytes() {
    assert_parity(
        "Strings",
        &Strings {
            id: "proxima-b".to_string(),
            payload: vec![0, 1, 2, 250, 255],
        },
    );
    // Empty cases matter: zero-length length prefixes must match too.
    assert_parity(
        "Strings(empty)",
        &Strings {
            id: String::new(),
            payload: Vec::new(),
        },
    );
}

#[test]
fn parity_options() {
    assert_parity(
        "Options",
        &Options {
            present: Some(42),
            absent: None,
            nested: Some("x".to_string()),
        },
    );
}

#[test]
fn parity_btree_collections() {
    let mut map = BTreeMap::new();
    map.insert("co2".to_string(), 209_500u32);
    map.insert("n2".to_string(), 780_800u32);
    let set: BTreeSet<String> = ["geology.rocky", "atmosphere.thin"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_parity("Collections", &Collections { map, set });
    // Empty collections: the length prefix for an empty seq/map must match.
    assert_parity(
        "Collections(empty)",
        &Collections {
            map: BTreeMap::new(),
            set: BTreeSet::new(),
        },
    );
}

#[test]
fn parity_enum_discriminant() {
    // bincode encodes a fieldless enum variant as a u32 index by default.
    assert_parity("Dimension::Physical", &Dimension::Physical);
    assert_parity("Dimension::NativeEcosystem", &Dimension::NativeEcosystem);
    assert_parity("Dimension::Civilization", &Dimension::Civilization);
}
