//! Convert AST nodes to `hpdl_model` types.
//!
//! This is the native-HPDL half of the "equivalent semantics ⇒ identical model"
//! contract (ADR 0010): every block the RON frontend can express is wired here so
//! that an `.hpdl` source and its `.ron` equivalent produce the *same*
//! [`PlanetDefinition`], and therefore the same validator diagnostics and the same
//! canonical PIR bytes.
//!
//! Enum variants and newtypes are spelled exactly as in RON — `Planet`,
//! `Custom("x")`, `Present(Microbial)`, `ModuleRef(name: "a", version: "1.0")` —
//! and arrive as [`ValueNode::Variant`].
//!
//! World laws are intentionally **not** wired here (ADR 0010 / ADR 0005): the
//! model's `world_laws` field is `#[serde(skip)]`, so neither serialized frontend
//! can express them, and doing so is coupled to a future PIR format-version
//! decision.

use crate::{ast::*, Error};
use hpdl_model::*;
use std::collections::BTreeMap;

/// Convert a FileNode to a PlanetDefinition.
pub fn file_to_planet_definition(file: FileNode) -> Result<PlanetDefinition, Error> {
    let fields = fields_to_map(file.planet.fields)?;

    let id = extract_string_field(&fields, "id")?;
    let body_id = BodyId::new(id).map_err(|e| Error::parse(format!("Invalid body ID: {e}")))?;

    let physical = extract_physical_properties(&fields)?;
    let traits = extract_traits(&fields)?;
    let modules = extract_modules(&fields)?;
    let statuses = extract_statuses(&fields)?;
    let actions = extract_actions(&fields)?;
    let migrations = extract_migrations(&fields)?;

    let mut body = CelestialBody::new(
        body_id, physical, traits, modules, statuses, actions, migrations,
    )
    .map_err(|e| Error::parse(format!("Invalid celestial body: {e}")))?;

    // Optional orbital / environment blocks (absent = undeclared, never defaulted).
    if let Some(identity) = extract_identity(&fields)? {
        body = body.with_identity(identity);
    }
    if let Some(parent) = extract_parent(&fields)? {
        body = body.with_parent(parent);
    }
    if let Some(orbit) = extract_orbit(&fields)? {
        body = body.with_orbit(orbit);
    }
    if let Some(rotation) = extract_rotation(&fields)? {
        body = body.with_rotation(rotation);
    }
    if let Some(environment) = extract_environment(&fields)? {
        body = body.with_environment(environment);
    }

    PlanetDefinition::new(file.language.version, body)
        .map_err(|e| Error::parse(format!("Invalid planet definition: {e}")))
}

// ---------------------------------------------------------------------------
// Generic value helpers
// ---------------------------------------------------------------------------

/// Convert a list of fields to a map, rejecting duplicate field names.
fn fields_to_map(fields: Vec<FieldNode>) -> Result<BTreeMap<String, ValueNode>, Error> {
    let mut map = BTreeMap::new();
    for field in fields {
        if map.insert(field.name.clone(), field.value).is_some() {
            return Err(Error::parse(format!("Duplicate field: {}", field.name)));
        }
    }
    Ok(map)
}

/// Interpret a value as a string.
fn as_string(value: &ValueNode, ctx: &str) -> Result<String, Error> {
    match value {
        ValueNode::String { value, .. } => Ok(value.clone()),
        _ => Err(Error::parse(format!("{ctx} must be a string"))),
    }
}

/// Interpret a value as an `i128` integer (unit suffix is ignored; see ADR 0010).
fn as_integer(value: &ValueNode, ctx: &str) -> Result<i128, Error> {
    match value {
        ValueNode::Integer { value, .. } => Ok(*value),
        ValueNode::Float { .. } => Err(Error::parse(format!(
            "{ctx} must be an integer in canonical units (floats are not permitted on the canonical path)"
        ))),
        _ => Err(Error::parse(format!("{ctx} must be an integer"))),
    }
}

/// Interpret a value as a list of elements.
fn as_list<'a>(value: &'a ValueNode, ctx: &str) -> Result<&'a [ValueNode], Error> {
    match value {
        ValueNode::List { elements, .. } => Ok(elements),
        _ => Err(Error::parse(format!("{ctx} must be a list"))),
    }
}

/// Interpret a value as a two-element pair, spelled either as a tuple
/// `(a, b)` (RON parity) or a two-element list `[a, b]`.
fn as_pair<'a>(value: &'a ValueNode, ctx: &str) -> Result<(&'a ValueNode, &'a ValueNode), Error> {
    let elems = match value {
        ValueNode::Tuple { elements, .. } | ValueNode::List { elements, .. } => elements.as_slice(),
        _ => return Err(Error::parse(format!("{ctx} must be a (a, b) pair"))),
    };
    match elems {
        [a, b] => Ok((a, b)),
        _ => Err(Error::parse(format!(
            "{ctx} must have exactly two elements"
        ))),
    }
}

/// Interpret a value as an object/struct field map.
fn as_object(value: &ValueNode, ctx: &str) -> Result<BTreeMap<String, ValueNode>, Error> {
    match value {
        ValueNode::Object { fields, .. } => fields_to_map(fields.clone()),
        // A struct-variant body (e.g. `ModuleRef(name: ..., version: ...)`) is also
        // accepted as an object when the variant name is contextually implied.
        ValueNode::Variant {
            args, fields, name, ..
        } if args.is_empty() && !fields.is_empty() => {
            let _ = name;
            fields_to_map(fields.clone())
        }
        _ => Err(Error::parse(format!("{ctx} must be an object"))),
    }
}

/// A view of a variant/newtype: its name plus positional args and named fields.
struct VariantView<'a> {
    name: &'a str,
    args: &'a [ValueNode],
    fields: &'a [FieldNode],
}

/// Interpret a value as an enum variant / newtype.
fn as_variant<'a>(value: &'a ValueNode, ctx: &str) -> Result<VariantView<'a>, Error> {
    match value {
        ValueNode::Variant {
            name, args, fields, ..
        } => Ok(VariantView { name, args, fields }),
        _ => Err(Error::parse(format!(
            "{ctx} must be a variant (e.g. `Planet` or `Custom(\"x\")`)"
        ))),
    }
}

/// Narrow an `i128` to a target integer type, with a typed error on overflow.
macro_rules! narrow {
    ($v:expr, $t:ty, $ctx:expr) => {
        <$t>::try_from($v)
            .map_err(|_| Error::parse(format!("{} is out of range for {}", $ctx, stringify!($t))))
    };
}

/// Fetch a required field from a map.
fn require<'a>(
    fields: &'a BTreeMap<String, ValueNode>,
    name: &str,
) -> Result<&'a ValueNode, Error> {
    fields
        .get(name)
        .ok_or_else(|| Error::parse(format!("Missing required field: {name}")))
}

/// Fetch a named field from a variant's struct-field list.
fn variant_field<'a>(v: &VariantView<'a>, name: &str) -> Option<&'a ValueNode> {
    v.fields.iter().find(|f| f.name == name).map(|f| &f.value)
}

// ---------------------------------------------------------------------------
// Top-level block extractors
// ---------------------------------------------------------------------------

fn extract_string_field(fields: &BTreeMap<String, ValueNode>, name: &str) -> Result<String, Error> {
    as_string(require(fields, name)?, &format!("Field '{name}'"))
}

fn extract_physical_properties(
    fields: &BTreeMap<String, ValueNode>,
) -> Result<PhysicalProperties, Error> {
    let Some(physical_val) = fields.get("physical") else {
        return Ok(PhysicalProperties::default());
    };
    let phys = as_object(physical_val, "Field 'physical'")?;

    Ok(PhysicalProperties {
        mass_kg: opt_u128(&phys, "mass_kg")?,
        radius_m: opt_u128(&phys, "radius_m")?,
        surface_gravity_mms2: opt_u64(&phys, "surface_gravity_mms2")?,
        orbital_period_s: opt_u64(&phys, "orbital_period_s")?,
        rotation_period_s: opt_i64(&phys, "rotation_period_s")?,
    })
}

fn opt_u128(fields: &BTreeMap<String, ValueNode>, name: &str) -> Result<Option<u128>, Error> {
    match fields.get(name) {
        Some(v) => Ok(Some(narrow!(as_integer(v, name)?, u128, name)?)),
        None => Ok(None),
    }
}

fn opt_u64(fields: &BTreeMap<String, ValueNode>, name: &str) -> Result<Option<u64>, Error> {
    match fields.get(name) {
        Some(v) => Ok(Some(narrow!(as_integer(v, name)?, u64, name)?)),
        None => Ok(None),
    }
}

fn opt_i64(fields: &BTreeMap<String, ValueNode>, name: &str) -> Result<Option<i64>, Error> {
    match fields.get(name) {
        Some(v) => Ok(Some(narrow!(as_integer(v, name)?, i64, name)?)),
        None => Ok(None),
    }
}

fn extract_traits(fields: &BTreeMap<String, ValueNode>) -> Result<TraitSet, Error> {
    let Some(traits_val) = fields.get("traits") else {
        return Ok(TraitSet::empty());
    };
    let trait_map = as_object(traits_val, "Field 'traits'")?;

    let tags = match trait_map.get("tags") {
        Some(v) => {
            let mut out = Vec::new();
            for elem in as_list(v, "trait tags")? {
                out.push(as_string(elem, "trait tag")?);
            }
            out
        }
        None => Vec::new(),
    };

    TraitSet::new(tags).map_err(|e| Error::parse(format!("Invalid trait set: {e}")))
}

fn extract_modules(fields: &BTreeMap<String, ValueNode>) -> Result<Vec<ModuleRef>, Error> {
    let Some(v) = fields.get("modules") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for elem in as_list(v, "Field 'modules'")? {
        out.push(parse_module_ref(elem)?);
    }
    Ok(out)
}

/// Parse a `ModuleRef(name: "a", version: "1.0")` variant.
fn parse_module_ref(value: &ValueNode) -> Result<ModuleRef, Error> {
    let v = as_variant(value, "module")?;
    if v.name != "ModuleRef" {
        return Err(Error::parse(format!(
            "expected `ModuleRef(...)`, found `{}`",
            v.name
        )));
    }
    let name = as_string(
        variant_field(&v, "name").ok_or_else(|| Error::parse("ModuleRef missing 'name'"))?,
        "ModuleRef name",
    )?;
    let version = as_string(
        variant_field(&v, "version").ok_or_else(|| Error::parse("ModuleRef missing 'version'"))?,
        "ModuleRef version",
    )?;
    ModuleRef::new(name, version).map_err(|e| Error::parse(format!("Invalid module ref: {e}")))
}

fn extract_statuses(fields: &BTreeMap<String, ValueNode>) -> Result<Vec<StatusDeclaration>, Error> {
    let Some(v) = fields.get("statuses") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for elem in as_list(v, "Field 'statuses'")? {
        out.push(parse_status(elem)?);
    }
    Ok(out)
}

/// Parse a `StatusDeclaration(name:, scale_min:, scale_max:, …optional…)` variant.
fn parse_status(value: &ValueNode) -> Result<StatusDeclaration, Error> {
    let v = as_variant(value, "status")?;
    if v.name != "StatusDeclaration" {
        return Err(Error::parse(format!(
            "expected `StatusDeclaration(...)`, found `{}`",
            v.name
        )));
    }
    let name = as_string(
        variant_field(&v, "name")
            .ok_or_else(|| Error::parse("StatusDeclaration missing 'name'"))?,
        "status name",
    )?;
    let scale_min = narrow!(
        as_integer(
            variant_field(&v, "scale_min")
                .ok_or_else(|| Error::parse("StatusDeclaration missing 'scale_min'"))?,
            "scale_min",
        )?,
        i64,
        "scale_min"
    )?;
    let scale_max = narrow!(
        as_integer(
            variant_field(&v, "scale_max")
                .ok_or_else(|| Error::parse("StatusDeclaration missing 'scale_max'"))?,
            "scale_max",
        )?,
        i64,
        "scale_max"
    )?;

    let mut status = StatusDeclaration::new(name, scale_min, scale_max)
        .map_err(|e| Error::parse(format!("Invalid status: {e}")))?;

    if let Some(dim) = variant_field(&v, "dimension") {
        status = status.with_dimension(parse_opt_dimension(dim)?);
    }
    if let Some(b) = variant_field(&v, "baseline") {
        if let Some(baseline) = parse_opt(b, parse_baseline)? {
            status = status.with_baseline(baseline);
        }
    }
    if let Some(e) = variant_field(&v, "evaluator") {
        if let Some(ev) = parse_opt(e, parse_evaluator)? {
            status = status.with_evaluator(ev);
        }
    }
    if let Some(c) = variant_field(&v, "cut_points") {
        if let Some(cp) = parse_opt(c, parse_cut_points)? {
            status = status.with_cut_points(cp);
        }
    }
    if let Some(w) = variant_field(&v, "window") {
        if let Some(win) = parse_opt(w, parse_window)? {
            status = status.with_window(win);
        }
    }
    Ok(status)
}

/// Resolve an optional field spelled as either a bare value or `Some(value)` /
/// `None` (RON parity). Returns `Ok(None)` for `None`.
fn parse_opt<T>(
    value: &ValueNode,
    inner: impl Fn(&ValueNode) -> Result<T, Error>,
) -> Result<Option<T>, Error> {
    if let ValueNode::Variant { name, args, .. } = value {
        match name.as_str() {
            "None" => return Ok(None),
            "Some" => {
                let arg = args
                    .first()
                    .ok_or_else(|| Error::parse("`Some(...)` requires one argument"))?;
                return Ok(Some(inner(arg)?));
            }
            _ => {}
        }
    }
    Ok(Some(inner(value)?))
}

fn parse_opt_dimension(value: &ValueNode) -> Result<StatusDimension, Error> {
    // `dimension` is itself optional in the model; a bare variant is the dimension.
    match parse_opt(value, parse_dimension)? {
        Some(d) => Ok(d),
        None => Err(Error::parse("status dimension cannot be None here")),
    }
}

fn parse_dimension(value: &ValueNode) -> Result<StatusDimension, Error> {
    let v = as_variant(value, "status dimension")?;
    match v.name {
        "Physical" => Ok(StatusDimension::Physical),
        "NativeEcosystem" => Ok(StatusDimension::NativeEcosystem),
        "Civilization" => Ok(StatusDimension::Civilization),
        "Supernatural" => Ok(StatusDimension::Supernatural),
        "Custom" => {
            let s = as_string(first_arg(&v, "Custom")?, "Custom dimension name")?;
            StatusDimension::custom(s).map_err(|e| Error::parse(format!("Invalid dimension: {e}")))
        }
        other => Err(Error::parse(format!("Unknown status dimension: {other}"))),
    }
}

fn parse_baseline(value: &ValueNode) -> Result<BaselineRef, Error> {
    let v = as_variant(value, "baseline")?;
    if v.name != "BaselineRef" {
        return Err(Error::parse(format!(
            "expected `BaselineRef(...)`, found `{}`",
            v.name
        )));
    }
    let name = as_string(
        variant_field(&v, "name").ok_or_else(|| Error::parse("BaselineRef missing 'name'"))?,
        "baseline name",
    )?;
    let digest = parse_digest(
        variant_field(&v, "digest").ok_or_else(|| Error::parse("BaselineRef missing 'digest'"))?,
    )?;
    BaselineRef::new(name, digest).map_err(|e| Error::parse(format!("Invalid baseline: {e}")))
}

fn parse_evaluator(value: &ValueNode) -> Result<EvaluatorRef, Error> {
    let v = as_variant(value, "evaluator")?;
    if v.name != "EvaluatorRef" {
        return Err(Error::parse(format!(
            "expected `EvaluatorRef(...)`, found `{}`",
            v.name
        )));
    }
    // EvaluatorRef wraps a single ModuleRef, either positional or under `module:`.
    let module_val = v
        .args
        .first()
        .or_else(|| variant_field(&v, "module"))
        .ok_or_else(|| Error::parse("EvaluatorRef requires a ModuleRef"))?;
    Ok(EvaluatorRef::new(parse_module_ref(module_val)?))
}

fn parse_cut_points(value: &ValueNode) -> Result<CutPoints, Error> {
    let v = as_variant(value, "cut_points")?;
    if v.name != "CutPoints" {
        return Err(Error::parse(format!(
            "expected `CutPoints(...)`, found `{}`",
            v.name
        )));
    }
    let scale_min = narrow!(
        as_integer(
            variant_field(&v, "scale_min")
                .ok_or_else(|| Error::parse("CutPoints missing 'scale_min'"))?,
            "scale_min",
        )?,
        i64,
        "scale_min"
    )?;
    let scale_max = narrow!(
        as_integer(
            variant_field(&v, "scale_max")
                .ok_or_else(|| Error::parse("CutPoints missing 'scale_max'"))?,
            "scale_max",
        )?,
        i64,
        "scale_max"
    )?;
    let cuts_val =
        variant_field(&v, "cuts").ok_or_else(|| Error::parse("CutPoints missing 'cuts'"))?;
    let mut cuts: Vec<(i64, String)> = Vec::new();
    for elem in as_list(cuts_val, "cut list")? {
        // Each cut is a 2-tuple `(value, "label")`.
        let (val_node, label_node) = as_pair(elem, "cut")?;
        let cut = narrow!(as_integer(val_node, "cut value")?, i64, "cut value")?;
        let label = as_string(label_node, "cut label")?;
        cuts.push((cut, label));
    }
    CutPoints::new(scale_min, scale_max, cuts)
        .map_err(|e| Error::parse(format!("Invalid cut points: {e}")))
}

fn parse_window(value: &ValueNode) -> Result<EvaluationWindow, Error> {
    let v = as_variant(value, "window")?;
    if v.name != "EvaluationWindow" {
        return Err(Error::parse(format!(
            "expected `EvaluationWindow(...)`, found `{}`",
            v.name
        )));
    }
    let span = narrow!(
        as_integer(
            variant_field(&v, "span")
                .ok_or_else(|| Error::parse("EvaluationWindow missing 'span'"))?,
            "span",
        )?,
        u64,
        "span"
    )?;
    let scope = match variant_field(&v, "scope") {
        Some(s) => parse_opt(s, |val| as_string(val, "window scope"))?,
        None => None,
    };
    EvaluationWindow::new(span, scope).map_err(|e| Error::parse(format!("Invalid window: {e}")))
}

fn extract_actions(fields: &BTreeMap<String, ValueNode>) -> Result<Vec<ActionDeclaration>, Error> {
    let Some(v) = fields.get("actions") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for elem in as_list(v, "Field 'actions'")? {
        out.push(parse_action(elem)?);
    }
    Ok(out)
}

/// Parse an `ActionDeclaration(name:, reads:, writes:, capabilities:)` variant.
fn parse_action(value: &ValueNode) -> Result<ActionDeclaration, Error> {
    let v = as_variant(value, "action")?;
    if v.name != "ActionDeclaration" {
        return Err(Error::parse(format!(
            "expected `ActionDeclaration(...)`, found `{}`",
            v.name
        )));
    }
    let name = as_string(
        variant_field(&v, "name")
            .ok_or_else(|| Error::parse("ActionDeclaration missing 'name'"))?,
        "action name",
    )?;
    let reads = string_list(variant_field(&v, "reads"), "reads")?;
    let writes = string_list(variant_field(&v, "writes"), "writes")?;

    let mut action = ActionDeclaration::new(name, reads, writes)
        .map_err(|e| Error::parse(format!("Invalid action: {e}")))?;

    if let Some(caps_val) = variant_field(&v, "capabilities") {
        let mut caps = Vec::new();
        for elem in as_list(caps_val, "capabilities")? {
            caps.push(parse_capability(elem)?);
        }
        action = action.with_capabilities(caps);
    }
    Ok(action)
}

/// Read an optional list-of-strings field (absent → empty).
fn string_list(value: Option<&ValueNode>, ctx: &str) -> Result<Vec<String>, Error> {
    let Some(v) = value else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for elem in as_list(v, ctx)? {
        out.push(as_string(elem, ctx)?);
    }
    Ok(out)
}

fn parse_capability(value: &ValueNode) -> Result<Capability, Error> {
    let v = as_variant(value, "capability")?;
    if v.name != "Capability" {
        return Err(Error::parse(format!(
            "expected `Capability(\"...\")`, found `{}`",
            v.name
        )));
    }
    let s = as_string(first_arg(&v, "Capability")?, "capability token")?;
    Capability::new(s).map_err(|e| Error::parse(format!("Invalid capability: {e}")))
}

fn extract_migrations(
    fields: &BTreeMap<String, ValueNode>,
) -> Result<Vec<MigrationDeclaration>, Error> {
    let Some(v) = fields.get("migrations") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for elem in as_list(v, "Field 'migrations'")? {
        out.push(parse_migration(elem)?);
    }
    Ok(out)
}

/// Parse a `MigrationDeclaration(name:, from_version:, to_version:)` variant.
fn parse_migration(value: &ValueNode) -> Result<MigrationDeclaration, Error> {
    let v = as_variant(value, "migration")?;
    if v.name != "MigrationDeclaration" {
        return Err(Error::parse(format!(
            "expected `MigrationDeclaration(...)`, found `{}`",
            v.name
        )));
    }
    let name = as_string(
        variant_field(&v, "name")
            .ok_or_else(|| Error::parse("MigrationDeclaration missing 'name'"))?,
        "migration name",
    )?;
    let from = narrow!(
        as_integer(
            variant_field(&v, "from_version")
                .ok_or_else(|| Error::parse("MigrationDeclaration missing 'from_version'"))?,
            "from_version",
        )?,
        u32,
        "from_version"
    )?;
    let to = narrow!(
        as_integer(
            variant_field(&v, "to_version")
                .ok_or_else(|| Error::parse("MigrationDeclaration missing 'to_version'"))?,
            "to_version",
        )?,
        u32,
        "to_version"
    )?;
    MigrationDeclaration::new(name, from, to)
        .map_err(|e| Error::parse(format!("Invalid migration: {e}")))
}

// ---------------------------------------------------------------------------
// Orbital block extractors
// ---------------------------------------------------------------------------

fn extract_identity(
    fields: &BTreeMap<String, ValueNode>,
) -> Result<Option<CelestialIdentity>, Error> {
    match fields.get("identity") {
        Some(v) => parse_opt(v, parse_identity),
        None => Ok(None),
    }
}

fn parse_identity(value: &ValueNode) -> Result<CelestialIdentity, Error> {
    let v = as_variant(value, "identity")?;
    match v.name {
        "Star" => Ok(CelestialIdentity::Star),
        "Planet" => Ok(CelestialIdentity::Planet),
        "DwarfPlanet" => Ok(CelestialIdentity::DwarfPlanet),
        "Moon" => Ok(CelestialIdentity::Moon),
        "Asteroid" => Ok(CelestialIdentity::Asteroid),
        "Comet" => Ok(CelestialIdentity::Comet),
        "Barycenter" => Ok(CelestialIdentity::Barycenter),
        "Other" => {
            let s = as_string(first_arg(&v, "Other")?, "identity name")?;
            CelestialIdentity::other(s).map_err(|e| Error::parse(format!("Invalid identity: {e}")))
        }
        other => Err(Error::parse(format!("Unknown celestial identity: {other}"))),
    }
}

fn extract_parent(fields: &BTreeMap<String, ValueNode>) -> Result<Option<ParentRef>, Error> {
    match fields.get("parent") {
        Some(v) => parse_opt(v, parse_parent),
        None => Ok(None),
    }
}

fn parse_parent(value: &ValueNode) -> Result<ParentRef, Error> {
    let v = as_variant(value, "parent")?;
    if v.name != "ParentRef" {
        return Err(Error::parse(format!(
            "expected `ParentRef(...)`, found `{}`",
            v.name
        )));
    }
    let target_val =
        variant_field(&v, "target").ok_or_else(|| Error::parse("ParentRef missing 'target'"))?;
    let target = parse_body_id(target_val)?;
    let frame = parse_frame(
        variant_field(&v, "frame").ok_or_else(|| Error::parse("ParentRef missing 'frame'"))?,
    )?;
    Ok(ParentRef::new(target, frame))
}

/// Parse a `BodyId("x")` newtype into a `BodyId`.
fn parse_body_id(value: &ValueNode) -> Result<BodyId, Error> {
    let v = as_variant(value, "BodyId")?;
    if v.name != "BodyId" {
        return Err(Error::parse(format!(
            "expected `BodyId(\"...\")`, found `{}`",
            v.name
        )));
    }
    let s = as_string(first_arg(&v, "BodyId")?, "body id")?;
    BodyId::new(s).map_err(|e| Error::parse(format!("Invalid body id: {e}")))
}

fn parse_frame(value: &ValueNode) -> Result<ReferenceFrame, Error> {
    let v = as_variant(value, "reference frame")?;
    match v.name {
        "EquatorialJ2000" => Ok(ReferenceFrame::EquatorialJ2000),
        "EclipticJ2000" => Ok(ReferenceFrame::EclipticJ2000),
        "BodyFixed" => Ok(ReferenceFrame::BodyFixed),
        "ParentCentered" => Ok(ReferenceFrame::ParentCentered),
        "Custom" => {
            let s = as_string(first_arg(&v, "Custom")?, "custom frame name")?;
            ReferenceFrame::custom(s).map_err(|e| Error::parse(format!("Invalid frame: {e}")))
        }
        other => Err(Error::parse(format!("Unknown reference frame: {other}"))),
    }
}

fn extract_orbit(fields: &BTreeMap<String, ValueNode>) -> Result<Option<KeplerianElements>, Error> {
    match fields.get("orbit") {
        Some(v) => parse_opt(v, parse_orbit),
        None => Ok(None),
    }
}

fn parse_orbit(value: &ValueNode) -> Result<KeplerianElements, Error> {
    let v = as_variant(value, "orbit")?;
    if v.name != "KeplerianElements" {
        return Err(Error::parse(format!(
            "expected `KeplerianElements(...)`, found `{}`",
            v.name
        )));
    }
    let sma = narrow!(
        as_integer(req_vf(&v, "semi_major_axis_m")?, "semi_major_axis_m")?,
        u128,
        "semi_major_axis_m"
    )?;
    let ecc = narrow!(
        as_integer(req_vf(&v, "eccentricity_micro")?, "eccentricity_micro")?,
        u32,
        "eccentricity_micro"
    )?;
    let inc = narrow!(
        as_integer(req_vf(&v, "inclination_mdeg")?, "inclination_mdeg")?,
        i32,
        "inclination_mdeg"
    )?;
    let node = narrow!(
        as_integer(
            req_vf(&v, "lon_ascending_node_mdeg")?,
            "lon_ascending_node_mdeg"
        )?,
        i32,
        "lon_ascending_node_mdeg"
    )?;
    let peri = narrow!(
        as_integer(req_vf(&v, "arg_periapsis_mdeg")?, "arg_periapsis_mdeg")?,
        i32,
        "arg_periapsis_mdeg"
    )?;
    let anomaly = narrow!(
        as_integer(
            req_vf(&v, "mean_anomaly_epoch_mdeg")?,
            "mean_anomaly_epoch_mdeg"
        )?,
        i32,
        "mean_anomaly_epoch_mdeg"
    )?;
    let frame = parse_frame(req_vf(&v, "frame")?)?;
    KeplerianElements::new(sma, ecc, inc, node, peri, anomaly, frame)
        .map_err(|e| Error::parse(format!("Invalid orbit: {e}")))
}

fn extract_rotation(fields: &BTreeMap<String, ValueNode>) -> Result<Option<RotationModel>, Error> {
    match fields.get("rotation") {
        Some(v) => parse_opt(v, parse_rotation),
        None => Ok(None),
    }
}

fn parse_rotation(value: &ValueNode) -> Result<RotationModel, Error> {
    let v = as_variant(value, "rotation")?;
    if v.name != "RotationModel" {
        return Err(Error::parse(format!(
            "expected `RotationModel(...)`, found `{}`",
            v.name
        )));
    }
    let period = narrow!(
        as_integer(req_vf(&v, "sidereal_period_s")?, "sidereal_period_s")?,
        i64,
        "sidereal_period_s"
    )?;
    let tilt = narrow!(
        as_integer(req_vf(&v, "axial_tilt_mdeg")?, "axial_tilt_mdeg")?,
        i32,
        "axial_tilt_mdeg"
    )?;
    let frame = parse_frame(req_vf(&v, "frame")?)?;
    RotationModel::new(period, tilt, frame)
        .map_err(|e| Error::parse(format!("Invalid rotation: {e}")))
}

// ---------------------------------------------------------------------------
// Environment block extractors
// ---------------------------------------------------------------------------

fn extract_environment(
    fields: &BTreeMap<String, ValueNode>,
) -> Result<Option<EnvironmentDefinition>, Error> {
    match fields.get("environment") {
        Some(v) => parse_opt(v, parse_environment),
        None => Ok(None),
    }
}

fn parse_environment(value: &ValueNode) -> Result<EnvironmentDefinition, Error> {
    let v = as_variant(value, "environment")?;
    if v.name != "EnvironmentDefinition" {
        return Err(Error::parse(format!(
            "expected `EnvironmentDefinition(...)`, found `{}`",
            v.name
        )));
    }
    let biosphere = parse_biosphere(req_vf(&v, "biosphere")?)?;
    let mut env = EnvironmentDefinition::new(biosphere);

    if let Some(a) = variant_field(&v, "atmosphere") {
        if let Some(atm) = parse_opt(a, parse_atmosphere)? {
            env = env.with_atmosphere(atm);
        }
    }
    if let Some(t) = variant_field(&v, "thermal") {
        if let Some(th) = parse_opt(t, parse_thermal)? {
            env = env.with_thermal(th);
        }
    }
    if let Some(h) = variant_field(&v, "hydrosphere") {
        if let Some(hy) = parse_opt(h, parse_hydrosphere)? {
            env = env.with_hydrosphere(hy);
        }
    }
    if let Some(c) = variant_field(&v, "cryosphere") {
        if let Some(cr) = parse_opt(c, parse_cryosphere)? {
            env = env.with_cryosphere(cr);
        }
    }
    if let Some(assets_val) = variant_field(&v, "assets") {
        for elem in as_list(assets_val, "assets")? {
            env = env.with_asset(parse_asset(elem)?);
        }
    }
    Ok(env)
}

fn parse_biosphere(value: &ValueNode) -> Result<Biosphere, Error> {
    let v = as_variant(value, "biosphere")?;
    match v.name {
        "Absent" => Ok(Biosphere::Absent),
        "Unknown" => Ok(Biosphere::Unknown),
        "Present" => Ok(Biosphere::Present(parse_complexity(first_arg(
            &v, "Present",
        )?)?)),
        other => Err(Error::parse(format!("Unknown biosphere state: {other}"))),
    }
}

fn parse_complexity(value: &ValueNode) -> Result<ComplexityTier, Error> {
    let v = as_variant(value, "complexity tier")?;
    match v.name {
        "Microbial" => Ok(ComplexityTier::Microbial),
        "Simple" => Ok(ComplexityTier::Simple),
        "Complex" => Ok(ComplexityTier::Complex),
        "Intelligent" => Ok(ComplexityTier::Intelligent),
        "Other" => {
            let s = as_string(first_arg(&v, "Other")?, "complexity tier name")?;
            ComplexityTier::other(s).map_err(|e| Error::parse(format!("Invalid tier: {e}")))
        }
        other => Err(Error::parse(format!("Unknown complexity tier: {other}"))),
    }
}

fn parse_atmosphere(value: &ValueNode) -> Result<AtmosphereProfile, Error> {
    let v = as_variant(value, "atmosphere")?;
    if v.name != "AtmosphereProfile" {
        return Err(Error::parse(format!(
            "expected `AtmosphereProfile(...)`, found `{}`",
            v.name
        )));
    }
    let pressure = narrow!(
        as_integer(req_vf(&v, "surface_pressure_pa")?, "surface_pressure_pa")?,
        u64,
        "surface_pressure_pa"
    )?;
    let composition = parse_composition(req_vf(&v, "composition")?)?;
    let climate_model = match variant_field(&v, "climate_model") {
        Some(c) => parse_opt(c, parse_module_ref)?,
        None => None,
    };
    AtmosphereProfile::new(pressure, composition, climate_model)
        .map_err(|e| Error::parse(format!("Invalid atmosphere: {e}")))
}

fn parse_composition(value: &ValueNode) -> Result<Composition, Error> {
    let v = as_variant(value, "composition")?;
    if v.name != "Composition" {
        return Err(Error::parse(format!(
            "expected `Composition(...)`, found `{}`",
            v.name
        )));
    }
    let components_val = req_vf(&v, "components")?;
    let mut components: Vec<(GasId, u32)> = Vec::new();
    for elem in as_list(components_val, "composition components")? {
        let (gas_node, ppm_node) = as_pair(elem, "component")?;
        let gas = parse_gas_id(gas_node)?;
        let ppm = narrow!(as_integer(ppm_node, "component ppm")?, u32, "component ppm")?;
        components.push((gas, ppm));
    }
    let tolerance = narrow!(
        as_integer(req_vf(&v, "tolerance_ppm")?, "tolerance_ppm")?,
        u32,
        "tolerance_ppm"
    )?;
    Composition::new(components, tolerance)
        .map_err(|e| Error::parse(format!("Invalid composition: {e}")))
}

fn parse_gas_id(value: &ValueNode) -> Result<GasId, Error> {
    let v = as_variant(value, "GasId")?;
    if v.name != "GasId" {
        return Err(Error::parse(format!(
            "expected `GasId(\"...\")`, found `{}`",
            v.name
        )));
    }
    let s = as_string(first_arg(&v, "GasId")?, "gas id")?;
    GasId::new(s).map_err(|e| Error::parse(format!("Invalid gas id: {e}")))
}

fn parse_thermal(value: &ValueNode) -> Result<ThermalProfile, Error> {
    let v = as_variant(value, "thermal")?;
    if v.name != "ThermalProfile" {
        return Err(Error::parse(format!(
            "expected `ThermalProfile(...)`, found `{}`",
            v.name
        )));
    }
    let mean = narrow!(
        as_integer(req_vf(&v, "mean_mk")?, "mean_mk")?,
        u64,
        "mean_mk"
    )?;
    let min = narrow!(as_integer(req_vf(&v, "min_mk")?, "min_mk")?, u64, "min_mk")?;
    let max = narrow!(as_integer(req_vf(&v, "max_mk")?, "max_mk")?, u64, "max_mk")?;
    ThermalProfile::new(mean, min, max).map_err(|e| Error::parse(format!("Invalid thermal: {e}")))
}

fn parse_hydrosphere(value: &ValueNode) -> Result<Hydrosphere, Error> {
    let v = as_variant(value, "hydrosphere")?;
    if v.name != "Hydrosphere" {
        return Err(Error::parse(format!(
            "expected `Hydrosphere(...)`, found `{}`",
            v.name
        )));
    }
    let coverage = narrow!(
        as_integer(req_vf(&v, "surface_coverage_ppm")?, "surface_coverage_ppm")?,
        u32,
        "surface_coverage_ppm"
    )?;
    let dominant_liquid = match variant_field(&v, "dominant_liquid") {
        Some(d) => parse_opt(d, parse_gas_id)?,
        None => None,
    };
    Hydrosphere::new(coverage, dominant_liquid)
        .map_err(|e| Error::parse(format!("Invalid hydrosphere: {e}")))
}

fn parse_cryosphere(value: &ValueNode) -> Result<Cryosphere, Error> {
    let v = as_variant(value, "cryosphere")?;
    if v.name != "Cryosphere" {
        return Err(Error::parse(format!(
            "expected `Cryosphere(...)`, found `{}`",
            v.name
        )));
    }
    let coverage = narrow!(
        as_integer(req_vf(&v, "surface_coverage_ppm")?, "surface_coverage_ppm")?,
        u32,
        "surface_coverage_ppm"
    )?;
    Cryosphere::new(coverage).map_err(|e| Error::parse(format!("Invalid cryosphere: {e}")))
}

fn parse_asset(value: &ValueNode) -> Result<AssetRef, Error> {
    let v = as_variant(value, "asset")?;
    if v.name != "AssetRef" {
        return Err(Error::parse(format!(
            "expected `AssetRef(...)`, found `{}`",
            v.name
        )));
    }
    let name = as_string(req_vf(&v, "name")?, "asset name")?;
    let kind = parse_asset_kind(req_vf(&v, "kind")?)?;
    let digest = parse_digest(req_vf(&v, "digest")?)?;
    AssetRef::new(name, kind, digest).map_err(|e| Error::parse(format!("Invalid asset: {e}")))
}

fn parse_asset_kind(value: &ValueNode) -> Result<AssetKind, Error> {
    let v = as_variant(value, "asset kind")?;
    match v.name {
        "Texture" => Ok(AssetKind::Texture),
        "Mesh" => Ok(AssetKind::Mesh),
        "Heightmap" => Ok(AssetKind::Heightmap),
        "Other" => {
            let s = as_string(first_arg(&v, "Other")?, "asset kind name")?;
            AssetKind::other(s).map_err(|e| Error::parse(format!("Invalid asset kind: {e}")))
        }
        other => Err(Error::parse(format!("Unknown asset kind: {other}"))),
    }
}

fn parse_digest(value: &ValueNode) -> Result<Digest, Error> {
    // Accept both `Digest("...")` and a bare string, matching how RON may spell it.
    match value {
        ValueNode::Variant { name, args, .. } if name == "Digest" => {
            let arg = args
                .first()
                .ok_or_else(|| Error::parse("`Digest(...)` requires one argument"))?;
            let s = as_string(arg, "digest")?;
            Digest::new(s).map_err(|e| Error::parse(format!("Invalid digest: {e}")))
        }
        ValueNode::String { value, .. } => {
            Digest::new(value.clone()).map_err(|e| Error::parse(format!("Invalid digest: {e}")))
        }
        _ => Err(Error::parse("digest must be a string or `Digest(\"...\")`")),
    }
}

// ---------------------------------------------------------------------------
// Variant-argument helpers
// ---------------------------------------------------------------------------

/// The single positional argument of a newtype variant, e.g. the `"x"` in
/// `Custom("x")`.
fn first_arg<'a>(v: &VariantView<'a>, name: &str) -> Result<&'a ValueNode, Error> {
    v.args
        .first()
        .ok_or_else(|| Error::parse(format!("`{name}(...)` requires one argument")))
}

/// Require a named field inside a struct variant.
fn req_vf<'a>(v: &VariantView<'a>, name: &str) -> Result<&'a ValueNode, Error> {
    variant_field(v, name).ok_or_else(|| Error::parse(format!("missing field '{name}'")))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::SourceFile;

    fn parse(src: &str) -> PlanetDefinition {
        let source = SourceFile::from_string("t.hpdl", src.to_string());
        crate::parse_file(&source).expect("should parse")
    }

    #[test]
    fn test_fields_to_map() {
        let fields = vec![FieldNode {
            name: "id".to_string(),
            value: ValueNode::String {
                value: "test".to_string(),
                span: crate::SourceSpan::new(0, 4),
            },
            span: crate::SourceSpan::new(0, 4),
        }];
        let map = fields_to_map(fields).unwrap();
        assert!(map.contains_key("id"));
    }

    #[test]
    fn test_extract_string_field() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "id".to_string(),
            ValueNode::String {
                value: "test".to_string(),
                span: crate::SourceSpan::new(0, 4),
            },
        );
        assert_eq!(extract_string_field(&fields, "id").unwrap(), "test");
    }

    #[test]
    fn modules_are_extracted() {
        let def = parse(
            r#"
            language hpdl "1.0";
            planet p {
                id: "p",
                modules: [ ModuleRef(name: "hpdl/stdlib/atmo", version: "1.0") ],
            }
        "#,
        );
        assert_eq!(def.body().modules().len(), 1);
    }

    #[test]
    fn statuses_actions_migrations_extracted() {
        let def = parse(
            r#"
            language hpdl "1.0";
            planet p {
                id: "p",
                statuses: [ StatusDeclaration(name: "Vitality", scale_min: 0, scale_max: 100) ],
                actions: [ ActionDeclaration(
                    name: "Terraform",
                    reads: ["state.atmosphere.co2"],
                    writes: ["state.atmosphere.co2"],
                    capabilities: [ Capability("state.atmosphere.write") ],
                ) ],
                migrations: [ MigrationDeclaration(name: "m1", from_version: 1, to_version: 2) ],
            }
        "#,
        );
        assert_eq!(def.body().statuses().len(), 1);
        assert_eq!(def.body().actions().len(), 1);
        assert_eq!(def.body().migrations().len(), 1);
    }

    #[test]
    fn orbital_block_extracted() {
        let def = parse(
            r#"
            language hpdl "1.0";
            planet p {
                id: "p",
                identity: Planet,
                parent: ParentRef(target: BodyId("sol"), frame: EclipticJ2000),
                orbit: KeplerianElements(
                    semi_major_axis_m: 149598000000,
                    eccentricity_micro: 16700,
                    inclination_mdeg: 0,
                    lon_ascending_node_mdeg: 174000,
                    arg_periapsis_mdeg: 288000,
                    mean_anomaly_epoch_mdeg: 0,
                    frame: EclipticJ2000,
                ),
                rotation: RotationModel(
                    sidereal_period_s: 86164,
                    axial_tilt_mdeg: 23440,
                    frame: EclipticJ2000,
                ),
            }
        "#,
        );
        assert_eq!(def.body().identity(), Some(&CelestialIdentity::Planet));
        assert!(def.body().parent().is_some());
        assert!(def.body().orbit().is_some());
        assert!(def.body().rotation().is_some());
    }

    #[test]
    fn environment_block_extracted() {
        let def = parse(
            r#"
            language hpdl "1.0";
            planet p {
                id: "p",
                environment: EnvironmentDefinition(
                    biosphere: Present(Microbial),
                    atmosphere: AtmosphereProfile(
                        surface_pressure_pa: 101325,
                        composition: Composition(
                            components: [ (GasId("n2"), 1000000) ],
                            tolerance_ppm: 0,
                        ),
                    ),
                    thermal: ThermalProfile(mean_mk: 288000, min_mk: 184000, max_mk: 331000),
                    hydrosphere: Hydrosphere(surface_coverage_ppm: 708000),
                    cryosphere: Cryosphere(surface_coverage_ppm: 100000),
                ),
            }
        "#,
        );
        let env = def.body().environment().expect("env present");
        assert_eq!(
            env.biosphere(),
            &Biosphere::Present(ComplexityTier::Microbial)
        );
        assert!(env.atmosphere().is_some());
        assert!(env.thermal().is_some());
        assert!(env.hydrosphere().is_some());
        assert!(env.cryosphere().is_some());
    }

    #[test]
    fn unknown_enum_variant_is_rejected() {
        // `Jupiterian` is not a CelestialIdentity variant.
        let source = SourceFile::from_string(
            "bad.hpdl",
            r#"
            language hpdl "1.0";
            planet p { id: "p", identity: Jupiterian }
        "#
            .to_string(),
        );
        assert!(crate::parse_file(&source).is_err());
    }

    #[test]
    fn float_in_integer_field_is_rejected() {
        // Floats are not permitted on the canonical path (ADR 0010).
        let source = SourceFile::from_string(
            "bad.hpdl",
            r#"
            language hpdl "1.0";
            planet p { id: "p", physical: { mass_kg: 3.14 } }
        "#
            .to_string(),
        );
        assert!(crate::parse_file(&source).is_err());
    }

    #[test]
    fn some_and_none_optional_sugar_round_trips() {
        // `None` → absent; `Some(Planet)` → present. Both are accepted (RON parity).
        let none = parse(
            r#"
            language hpdl "1.0";
            planet p { id: "p", identity: None }
        "#,
        );
        assert_eq!(none.body().identity(), None);

        let some = parse(
            r#"
            language hpdl "1.0";
            planet p { id: "p", identity: Some(Planet) }
        "#,
        );
        assert_eq!(some.body().identity(), Some(&CelestialIdentity::Planet));
    }

    #[test]
    fn status_write_is_rejected_in_native_hpdl() {
        let source = SourceFile::from_string(
            "bad.hpdl",
            r#"
            language hpdl "1.0";
            planet p {
                id: "p",
                actions: [ ActionDeclaration(
                    name: "Bad",
                    reads: [],
                    writes: ["status.AtmosphericStability"],
                ) ],
            }
        "#
            .to_string(),
        );
        assert!(crate::parse_file(&source).is_err());
    }
}
