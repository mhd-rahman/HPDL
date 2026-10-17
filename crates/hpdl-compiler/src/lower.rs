//! Fixed-point lowering / representability guard.
//!
//! The domain model already stores canonical **scaled integers** (e.g.
//! `mass_kg: u128`, `surface_gravity_mms2` in milli-m/s²), and the current PIR
//! format encodes those integers directly. So this stage is a
//! *representability guard*, not a re-scaling pass: it verifies every numeric
//! field fits its documented canonical storage width using checked arithmetic,
//! and reserves the seam for the per-field quantum / overflow contracts of
//! Amendment A2 (open decision OD-06.2).
//!
//! No floating-point arithmetic occurs here (enforced crate-wide via
//! `#![deny(clippy::float_arithmetic)]`). The function is a pure function of the
//! validated model and introduces no ambient nondeterminism.

use hpdl_model::PlanetDefinition;

use crate::Error;

/// Canonical storage width for `mass_kg` (model: `u128`).
const MASS_KG_MAX: u128 = u128::MAX;
/// Canonical storage width for `radius_m` (model: `u128`).
const RADIUS_M_MAX: u128 = u128::MAX;

/// Verify that every numeric field of the definition is representable in its
/// canonical PIR storage width.
///
/// # Errors
///
/// Returns [`Error::Lowering`] if any field cannot be represented. With the
/// current model widths this cannot trigger for well-typed inputs; the checks
/// exist so that any future tightening of a field's quantum or storage width
/// fails loudly rather than silently truncating.
pub(crate) fn check_representable(definition: &PlanetDefinition) -> Result<(), Error> {
    let props = definition.body().properties();

    if let Some(mass_kg) = props.mass_kg {
        check_u128("mass_kg", mass_kg, MASS_KG_MAX)?;
    }
    if let Some(radius_m) = props.radius_m {
        check_u128("radius_m", radius_m, RADIUS_M_MAX)?;
    }
    if let Some(gravity) = props.surface_gravity_mms2 {
        check_u64("surface_gravity_mms2", gravity)?;
    }
    if let Some(period) = props.orbital_period_s {
        check_u64("orbital_period_s", period)?;
    }
    if let Some(rotation) = props.rotation_period_s {
        check_i64("rotation_period_s", rotation)?;
    }

    Ok(())
}

/// Verify an unsigned 128-bit field does not exceed its canonical maximum.
fn check_u128(field: &str, value: u128, max: u128) -> Result<(), Error> {
    if value > max {
        return Err(Error::Lowering {
            field: field.to_string(),
            reason: format!("value {value} exceeds canonical maximum {max}"),
        });
    }
    Ok(())
}

/// Verify an unsigned 64-bit field round-trips through its storage width.
fn check_u64(field: &str, value: u64) -> Result<(), Error> {
    // The field is already `u64`; confirm it round-trips through the widened
    // intermediate used for mixed-quantum arithmetic (Amendment A2) without
    // loss. Any future rescale would reuse this checked path.
    let widened = u128::from(value);
    match u64::try_from(widened) {
        Ok(back) if back == value => Ok(()),
        _ => Err(Error::Lowering {
            field: field.to_string(),
            reason: format!("value {value} is not representable in canonical u64 storage"),
        }),
    }
}

/// Verify a signed 64-bit field round-trips through its storage width.
fn check_i64(field: &str, value: i64) -> Result<(), Error> {
    let widened = i128::from(value);
    match i64::try_from(widened) {
        Ok(back) if back == value => Ok(()),
        _ => Err(Error::Lowering {
            field: field.to_string(),
            reason: format!("value {value} is not representable in canonical i64 storage"),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, TraitSet};

    fn definition_with_props(props: PhysicalProperties) -> PlanetDefinition {
        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            props,
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();
        PlanetDefinition::new("1.0", body).unwrap()
    }

    #[test]
    fn representable_values_pass() {
        let props = PhysicalProperties {
            mass_kg: Some(5_972_000_000_000_000_000_000_000), // ~Earth mass
            radius_m: Some(6_371_000),
            surface_gravity_mms2: Some(9_810),
            orbital_period_s: Some(31_557_600),
            rotation_period_s: Some(-20_996_800), // retrograde
        };
        assert!(check_representable(&definition_with_props(props)).is_ok());
    }

    #[test]
    fn all_absent_passes() {
        assert!(check_representable(&definition_with_props(PhysicalProperties::default())).is_ok());
    }

    #[test]
    fn u128_over_tightened_max_fails() {
        // Directly exercise the guard with a tightened maximum to prove it
        // reports a typed error (the model's own u128 field cannot exceed
        // u128::MAX, so we test the guard primitive).
        let err = check_u128("mass_kg", 1_000, 999).unwrap_err();
        match err {
            Error::Lowering { field, .. } => assert_eq!(field, "mass_kg"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn extreme_values_round_trip() {
        let props = PhysicalProperties {
            mass_kg: Some(u128::MAX),
            radius_m: Some(u128::MAX),
            surface_gravity_mms2: Some(u64::MAX),
            orbital_period_s: Some(u64::MAX),
            rotation_period_s: Some(i64::MIN),
        };
        assert!(check_representable(&definition_with_props(props)).is_ok());
    }
}
