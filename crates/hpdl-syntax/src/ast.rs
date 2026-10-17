//! Abstract Syntax Tree (AST) node definitions.

use crate::SourceSpan;

/// A complete HPDL file.
#[derive(Debug, Clone, PartialEq)]
pub struct FileNode {
    /// Language declaration.
    pub language: LanguageDeclNode,

    /// Planet declaration.
    pub planet: PlanetNode,
}

/// Language declaration: `language hpdl "1.0";`
#[derive(Debug, Clone, PartialEq)]
pub struct LanguageDeclNode {
    /// The language version string.
    pub version: String,

    /// Source span of this node.
    pub span: SourceSpan,
}

/// Planet declaration: `planet identifier { fields }`
#[derive(Debug, Clone, PartialEq)]
pub struct PlanetNode {
    /// Planet identifier.
    pub identifier: String,

    /// List of fields.
    pub fields: Vec<FieldNode>,

    /// Source span of this node.
    pub span: SourceSpan,
}

/// A field: `name: value`
#[derive(Debug, Clone, PartialEq)]
pub struct FieldNode {
    /// Field name.
    pub name: String,

    /// Field value.
    pub value: ValueNode,

    /// Source span of this node.
    pub span: SourceSpan,
}

/// A value (literal, list, or object).
#[derive(Debug, Clone, PartialEq)]
pub enum ValueNode {
    /// String literal.
    String {
        /// The string value (without quotes).
        value: String,
        /// Source span.
        span: SourceSpan,
    },

    /// Integer literal.
    ///
    /// Stored as `i128` so planetary magnitudes (e.g. mass in kg, which exceed
    /// `u64`) survive parsing without lossy float coercion. Range checks into
    /// the narrower model types happen during conversion.
    Integer {
        /// The integer value.
        value: i128,
        /// Optional unit suffix.
        unit: Option<String>,
        /// Source span.
        span: SourceSpan,
    },

    /// Float literal.
    Float {
        /// The float value.
        value: f64,
        /// Optional unit suffix.
        unit: Option<String>,
        /// Source span.
        span: SourceSpan,
    },

    /// Boolean literal.
    Boolean {
        /// The boolean value.
        value: bool,
        /// Source span.
        span: SourceSpan,
    },

    /// List of values: `[value1, value2]`
    List {
        /// List elements.
        elements: Vec<ValueNode>,
        /// Source span.
        span: SourceSpan,
    },

    /// Object (map of fields): `{ field1: value1, field2: value2 }`
    Object {
        /// Object fields.
        fields: Vec<FieldNode>,
        /// Source span.
        span: SourceSpan,
    },
}

impl ValueNode {
    /// Get the source span of this value node.
    pub fn span(&self) -> SourceSpan {
        match self {
            Self::String { span, .. }
            | Self::Integer { span, .. }
            | Self::Float { span, .. }
            | Self::Boolean { span, .. }
            | Self::List { span, .. }
            | Self::Object { span, .. } => *span,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_value_node_span() {
        let span = SourceSpan::new(0, 5);
        let value = ValueNode::String {
            value: "test".to_string(),
            span,
        };

        assert_eq!(value.span(), span);
    }

    #[test]
    fn test_field_node() {
        let field = FieldNode {
            name: "id".to_string(),
            value: ValueNode::String {
                value: "earth".to_string(),
                span: SourceSpan::new(4, 11),
            },
            span: SourceSpan::new(0, 11),
        };

        assert_eq!(field.name, "id");
    }
}
