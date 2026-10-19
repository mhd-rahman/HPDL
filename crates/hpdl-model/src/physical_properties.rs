/// Physical constants declared for a celestial body.
///
/// All fields are `Option` — an absent value means the property is
/// undeclared, **not** that it defaults to an Earth-like value.
///
/// No floating-point types are used. Continuous quantities are stored as
/// scaled integers (e.g. surface gravity in milli-m/s²) to preserve
/// deterministic canonical encoding.
///
/// # Example
///
/// ```
/// use hpdl_model::PhysicalProperties;
///
/// // A body with only mass declared
/// let props = PhysicalProperties {
///     mass_kg: Some(5_972_000_000_000_000_000_000_000),
///     ..PhysicalProperties::default()
/// };
/// assert!(props.radius_m.is_none());
/// ```
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Default,
    serde::Serialize,
    serde::Deserialize,
    wincode::SchemaWrite,
    wincode::SchemaRead,
)]
pub struct PhysicalProperties {
    /// Mass in kilograms. `None` means undeclared.
    pub mass_kg: Option<u128>,

    /// Equatorial radius in metres. `None` means undeclared.
    pub radius_m: Option<u128>,

    /// Surface gravitational acceleration in **milli-m/s²** (i.e. 9810 ≈ 9.81 m/s²).
    /// Stored as an integer to avoid floating-point non-determinism.
    /// `None` means undeclared.
    pub surface_gravity_mms2: Option<u64>,

    /// Orbital period in seconds. `None` means undeclared or non-orbiting body.
    pub orbital_period_s: Option<u64>,

    /// Sidereal rotation period in seconds.
    /// Negative values indicate retrograde rotation.
    /// `None` means undeclared.
    pub rotation_period_s: Option<i64>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all_none() {
        let p = PhysicalProperties::default();
        assert!(p.mass_kg.is_none());
        assert!(p.radius_m.is_none());
        assert!(p.surface_gravity_mms2.is_none());
        assert!(p.orbital_period_s.is_none());
        assert!(p.rotation_period_s.is_none());
    }

    #[test]
    fn retrograde_rotation() {
        let p = PhysicalProperties {
            rotation_period_s: Some(-20_996_800), // Venus-like retrograde
            ..PhysicalProperties::default()
        };
        assert_eq!(p.rotation_period_s, Some(-20_996_800));
    }

    #[test]
    fn partial_declaration() {
        let p = PhysicalProperties {
            mass_kg: Some(1_000_000),
            ..PhysicalProperties::default()
        };
        assert_eq!(p.mass_kg, Some(1_000_000));
        assert!(p.radius_m.is_none());
    }
}
