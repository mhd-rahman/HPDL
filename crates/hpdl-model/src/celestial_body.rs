use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

use crate::{
    ActionDeclaration, BodyId, CelestialIdentity, EnvironmentDefinition, Error, KeplerianElements,
    MigrationDeclaration, ModuleRef, ParentRef, PhysicalProperties, RotationModel,
    StatusDeclaration, TraitSet, WorldLaw,
};

/// A complete, immutable declaration of a single celestial body.
///
/// A `CelestialBody` is a pure *declaration* — it describes what a body is,
/// what modules it references, and what statuses and actions it exposes.
/// It holds no mutable runtime state.
///
/// # Uniqueness constraints
///
/// - Action names must be unique within a single body.
/// - Status names must be unique within a single body.
///
/// # Example
///
/// ```
/// use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, TraitSet};
///
/// let body = CelestialBody::new(
///     BodyId::new("proxima-b").unwrap(),
///     PhysicalProperties::default(),
///     TraitSet::empty(),
///     vec![],
///     vec![],
///     vec![],
///     vec![],
/// ).unwrap();
///
/// assert_eq!(body.id().as_str(), "proxima-b");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct CelestialBody {
    id: BodyId,
    properties: PhysicalProperties,
    traits: TraitSet,
    modules: Vec<ModuleRef>,
    statuses: Vec<StatusDeclaration>,
    actions: Vec<ActionDeclaration>,
    migrations: Vec<MigrationDeclaration>,
    /// Declared kind of the body. `None` means undeclared (not a default kind).
    #[serde(default)]
    identity: Option<CelestialIdentity>,
    /// Declared gravitational parent + orbit frame. `None` means undeclared.
    #[serde(default)]
    parent: Option<ParentRef>,
    /// Declared Keplerian orbit. `None` means undeclared (not a default orbit).
    #[serde(default)]
    orbit: Option<KeplerianElements>,
    /// Declared rotation model. `None` means undeclared (not a default spin).
    #[serde(default)]
    rotation: Option<RotationModel>,
    /// Declared environment (atmosphere, thermal, hydro/cryosphere, biosphere,
    /// assets). `None` means undeclared.
    #[serde(default)]
    environment: Option<EnvironmentDefinition>,
    /// Declared world laws (alternative physics, magic, custom reality). Empty
    /// means the body declares no custom law.
    ///
    /// This field is `#[serde(skip)]` / `#[wincode(skip)]`: it is deliberately
    /// **not** part of the serialized definition yet. Canonical PIR therefore
    /// sees no new field — existing PIR bytes, hashes, and golden vectors are
    /// unchanged (see
    /// spec 11, OD-11.2: world laws are not lowered into PIR this feature). A
    /// serialized frontend surface for world laws is deferred to the RON / native
    /// parsers (features 04 / 05). The laws are carried in-memory for the single
    /// semantic validator, which runs on the live model, not on decoded bytes.
    ///
    /// `#[wincode(skip)]` mirrors `#[serde(skip)]`: wincode also omits the field
    /// from the byte stream and initializes it to `Default` on read, so the
    /// canonical PIR bytes are identical under either serializer.
    #[serde(skip)]
    #[wincode(skip)]
    world_laws: Vec<WorldLaw>,
}

impl CelestialBody {
    /// Construct a `CelestialBody`, enforcing uniqueness constraints.
    ///
    /// Returns [`Error::DuplicateActionName`] if two actions share a name.
    /// Returns [`Error::DuplicateStatusName`] if two statuses share a name.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: BodyId,
        properties: PhysicalProperties,
        traits: TraitSet,
        modules: Vec<ModuleRef>,
        statuses: Vec<StatusDeclaration>,
        actions: Vec<ActionDeclaration>,
        migrations: Vec<MigrationDeclaration>,
    ) -> Result<Self, Error> {
        // Check action name uniqueness
        let mut seen_actions: BTreeSet<&str> = BTreeSet::new();
        for action in &actions {
            if !seen_actions.insert(action.name()) {
                return Err(Error::DuplicateActionName(action.name().to_owned()));
            }
        }

        // Check status name uniqueness
        let mut seen_statuses: BTreeSet<&str> = BTreeSet::new();
        for status in &statuses {
            if !seen_statuses.insert(status.name()) {
                return Err(Error::DuplicateStatusName(status.name().to_owned()));
            }
        }

        Ok(Self {
            id,
            properties,
            traits,
            modules,
            statuses,
            actions,
            migrations,
            identity: None,
            parent: None,
            orbit: None,
            rotation: None,
            environment: None,
            world_laws: Vec::new(),
        })
    }

    /// Attach a [`CelestialIdentity`] to this body (builder style).
    ///
    /// # Example
    ///
    /// ```
    /// use hpdl_model::{BodyId, CelestialBody, CelestialIdentity, PhysicalProperties, TraitSet};
    ///
    /// let body = CelestialBody::new(
    ///     BodyId::new("mars").unwrap(),
    ///     PhysicalProperties::default(),
    ///     TraitSet::empty(),
    ///     vec![], vec![], vec![], vec![],
    /// ).unwrap().with_identity(CelestialIdentity::Planet);
    ///
    /// assert_eq!(body.identity(), Some(&CelestialIdentity::Planet));
    /// ```
    #[must_use]
    pub fn with_identity(mut self, identity: CelestialIdentity) -> Self {
        self.identity = Some(identity);
        self
    }

    /// Attach a [`ParentRef`] to this body (builder style).
    #[must_use]
    pub fn with_parent(mut self, parent: ParentRef) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Attach [`KeplerianElements`] to this body (builder style).
    #[must_use]
    pub fn with_orbit(mut self, orbit: KeplerianElements) -> Self {
        self.orbit = Some(orbit);
        self
    }

    /// Attach a [`RotationModel`] to this body (builder style).
    #[must_use]
    pub fn with_rotation(mut self, rotation: RotationModel) -> Self {
        self.rotation = Some(rotation);
        self
    }

    /// Attach an [`EnvironmentDefinition`] to this body (builder style).
    #[must_use]
    pub fn with_environment(mut self, environment: EnvironmentDefinition) -> Self {
        self.environment = Some(environment);
        self
    }

    /// Append a single [`WorldLaw`] declaration (builder style).
    #[must_use]
    pub fn with_world_law(mut self, law: WorldLaw) -> Self {
        self.world_laws.push(law);
        self
    }

    /// Append several [`WorldLaw`] declarations (builder style).
    #[must_use]
    pub fn with_world_laws(mut self, laws: impl IntoIterator<Item = WorldLaw>) -> Self {
        self.world_laws.extend(laws);
        self
    }

    /// Return the body's identifier.
    pub fn id(&self) -> &BodyId {
        &self.id
    }

    /// Return the body's physical properties.
    pub fn properties(&self) -> &PhysicalProperties {
        &self.properties
    }

    /// Return the body's trait set.
    pub fn traits(&self) -> &TraitSet {
        &self.traits
    }

    /// Return the list of module references.
    pub fn modules(&self) -> &[ModuleRef] {
        &self.modules
    }

    /// Return the list of status declarations.
    pub fn statuses(&self) -> &[StatusDeclaration] {
        &self.statuses
    }

    /// Return the list of action declarations.
    pub fn actions(&self) -> &[ActionDeclaration] {
        &self.actions
    }

    /// Return the list of migration declarations.
    pub fn migrations(&self) -> &[MigrationDeclaration] {
        &self.migrations
    }

    /// Return the declared celestial identity, if any. `None` = undeclared.
    pub fn identity(&self) -> Option<&CelestialIdentity> {
        self.identity.as_ref()
    }

    /// Return the declared gravitational parent, if any. `None` = undeclared.
    pub fn parent(&self) -> Option<&ParentRef> {
        self.parent.as_ref()
    }

    /// Return the declared Keplerian orbit, if any. `None` = undeclared.
    pub fn orbit(&self) -> Option<&KeplerianElements> {
        self.orbit.as_ref()
    }

    /// Return the declared rotation model, if any. `None` = undeclared.
    pub fn rotation(&self) -> Option<&RotationModel> {
        self.rotation.as_ref()
    }

    /// Return the declared environment, if any. `None` = undeclared.
    pub fn environment(&self) -> Option<&EnvironmentDefinition> {
        self.environment.as_ref()
    }

    /// Return the declared world laws. Empty = no custom law declared.
    pub fn world_laws(&self) -> &[WorldLaw] {
        &self.world_laws
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn minimal_body(id: &str) -> CelestialBody {
        CelestialBody::new(
            BodyId::new(id).unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap()
    }

    #[test]
    fn valid_empty_body() {
        let body = minimal_body("test-planet");
        assert_eq!(body.id().as_str(), "test-planet");
        assert!(body.actions().is_empty());
        assert!(body.statuses().is_empty());
    }

    #[test]
    fn duplicate_action_name_rejected() {
        let a1 = ActionDeclaration::new("DoThing", [] as [&str; 0], [] as [&str; 0]).unwrap();
        let a2 = ActionDeclaration::new("DoThing", [] as [&str; 0], [] as [&str; 0]).unwrap();
        let err = CelestialBody::new(
            BodyId::new("planet").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![a1, a2],
            vec![],
        );
        assert!(matches!(err, Err(Error::DuplicateActionName(_))));
    }

    #[test]
    fn duplicate_status_name_rejected() {
        let s1 = StatusDeclaration::new("Vitality", 0, 100).unwrap();
        let s2 = StatusDeclaration::new("Vitality", 0, 50).unwrap();
        let err = CelestialBody::new(
            BodyId::new("planet").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![s1, s2],
            vec![],
            vec![],
        );
        assert!(matches!(err, Err(Error::DuplicateStatusName(_))));
    }

    #[test]
    fn distinct_action_and_status_names_ok() {
        let action = ActionDeclaration::new("DoThing", [] as [&str; 0], [] as [&str; 0]).unwrap();
        let status = StatusDeclaration::new("Vitality", 0, 100).unwrap();
        assert!(CelestialBody::new(
            BodyId::new("planet").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![status],
            vec![action],
            vec![],
        )
        .is_ok());
    }
}
