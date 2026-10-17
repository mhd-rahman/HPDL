use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{status_declaration::validate_name, Capability, Error, ReadWriteSet};

/// A declaration that a named action exists on a body, with its explicit
/// read and write field sets.
///
/// Actions declare their data dependencies up front. Writes to status fields
/// are forbidden at the declaration level — status values are always derived,
/// never directly assigned.
///
/// # Example
///
/// ```
/// use hpdl_model::ActionDeclaration;
///
/// let a = ActionDeclaration::new(
///     "TerraformStep",
///     ["state.atmosphere_co2"],
///     ["state.atmosphere_co2"],
/// ).unwrap();
/// assert_eq!(a.name(), "TerraformStep");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDeclaration {
    name: String,
    reads: BTreeSet<String>,
    writes: BTreeSet<String>,
    /// Capability tokens this action declares it requires to mutate state.
    /// Empty = the action requires no special capability.
    #[serde(default)]
    capabilities: BTreeSet<Capability>,
}

impl ActionDeclaration {
    /// Construct an `ActionDeclaration`.
    ///
    /// Returns [`Error::InvalidName`] if the action name is invalid.
    /// Returns [`Error::StatusWriteForbidden`] if any entry in `writes`
    /// starts with `"status."`.
    pub fn new(
        name: impl Into<String>,
        reads: impl IntoIterator<Item = impl Into<String>>,
        writes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, Error> {
        let name = name.into();
        validate_name(&name)?;

        let reads: BTreeSet<String> = reads.into_iter().map(Into::into).collect();
        let writes: BTreeSet<String> = writes.into_iter().map(Into::into).collect();

        for field in &writes {
            if field.starts_with("status.") {
                return Err(Error::StatusWriteForbidden {
                    action_name: name,
                    field: field.clone(),
                });
            }
        }

        Ok(Self {
            name,
            reads,
            writes,
            capabilities: BTreeSet::new(),
        })
    }

    /// Attach declared [`Capability`] requirements to this action (builder style).
    ///
    /// # Example
    ///
    /// ```
    /// use hpdl_model::{ActionDeclaration, Capability};
    ///
    /// let a = ActionDeclaration::new("Terraform", [] as [&str; 0], ["state.atmosphere.co2"])
    ///     .unwrap()
    ///     .with_capabilities([Capability::new("state.atmosphere.write").unwrap()]);
    /// assert_eq!(a.capabilities().count(), 1);
    /// ```
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: impl IntoIterator<Item = Capability>) -> Self {
        self.capabilities = capabilities.into_iter().collect();
        self
    }

    /// Return the action name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the declared read set (lexicographic order).
    pub fn reads(&self) -> impl Iterator<Item = &String> {
        self.reads.iter()
    }

    /// Return the declared write set (lexicographic order).
    pub fn writes(&self) -> impl Iterator<Item = &String> {
        self.writes.iter()
    }

    /// Return the declared capability requirements (lexicographic order).
    pub fn capabilities(&self) -> impl Iterator<Item = &Capability> {
        self.capabilities.iter()
    }

    /// Assemble a [`ReadWriteSet`] view of this action's reads/writes/capabilities.
    ///
    /// Returns [`Error::InvalidFieldPath`] if any read/write path is malformed.
    /// (Paths are not re-validated at `ActionDeclaration::new` time for backward
    /// compatibility, so this is where field-path well-formedness is checked.)
    pub fn read_write_set(&self) -> Result<ReadWriteSet, Error> {
        ReadWriteSet::new(
            self.reads.iter().cloned(),
            self.writes.iter().cloned(),
            self.capabilities.iter().cloned(),
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn valid_action() {
        let a = ActionDeclaration::new("Terraform", ["state.co2"], ["state.co2"]).unwrap();
        assert_eq!(a.name(), "Terraform");
        assert_eq!(a.reads().count(), 1);
        assert_eq!(a.writes().count(), 1);
    }

    #[test]
    fn status_write_forbidden() {
        let err = ActionDeclaration::new("Bad", [] as [&str; 0], ["status.AtmosphericStability"]);
        assert!(matches!(err, Err(Error::StatusWriteForbidden { .. })));
    }

    #[test]
    fn empty_read_write_sets_allowed() {
        assert!(ActionDeclaration::new("NoOp", [] as [&str; 0], [] as [&str; 0]).is_ok());
    }

    #[test]
    fn empty_name_invalid() {
        assert!(matches!(
            ActionDeclaration::new("", [] as [&str; 0], [] as [&str; 0]),
            Err(Error::InvalidName(_))
        ));
    }

    #[test]
    fn non_status_write_allowed() {
        // "statusfield" does not start with "status." — must be allowed
        assert!(ActionDeclaration::new("A", [] as [&str; 0], ["statusfield"]).is_ok());
    }
}
