use serde::Serialize;
/// type registry for storing and managing C type information extracted from DWARF
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use log;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct TypeId(pub u64);

impl Hash for TypeId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub id: TypeId,
    pub kind: BaseTypeKind,
    pub pointer_depth: usize,
    pub is_const: bool,
    pub is_volatile: bool,
    pub dwarf_offset: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BaseTypeKind {
    /// int, float, uint8_t, size_t, etc.
    Primitive {
        name: String,
        size: usize,
        alignment: usize,
    },

    Struct {
        name: String,
        fields: Vec<StructField>,
        size: usize,
        alignment: usize,
        is_opaque: bool, // true if forward declaration only
    },

    Union {
        name: String,
        variants: Vec<UnionField>,
        size: usize,
        alignment: usize,
    },

    Enum {
        name: String,
        backing_id: TypeId,
        variants: Vec<EnumVariant>,
        size: usize,
    },

    /// fixed size array e.g. int[10]
    Array {
        element_type_id: TypeId,
        count: usize,
        size: usize,
    },

    Typedef {
        name: String,
        aliased_type_id: TypeId,
    },

    /// function pointer
    Function {
        return_type_id: Option<TypeId>,
        parameter_type_ids: Vec<TypeId>,
        is_variadic: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub type_id: TypeId,
    pub offset: usize, // offset in bytes from struct start
    pub size: usize,   // size in bytes
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnionField {
    pub name: String,
    pub type_id: TypeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub value: i64,
}

#[derive(Serialize)]
enum CanonicalTypeKind {
    Primitive(CanonicalPrimitive),
    Struct(CanonicalStruct),
    Union(CanonicalUnion),
    Enum(CanonicalEnum),
    Array(CanonicalArray),
    Typedef(CanonicalTypedef),
    Function(CanonicalFunction),
}

#[derive(Serialize)]
struct CanonicalPrimitive {
    name: String,
    size: usize,
    alignment: usize,
}

#[derive(Serialize)]
struct CanonicalStruct {
    name: String,
    fields: Vec<CanonicalField>,
    size: usize,
    alignment: usize,
    is_opaque: bool,
}

#[derive(Serialize)]
struct CanonicalField {
    name: String,
    type_id: TypeId,
    offset: usize,
    size: usize,
}

#[derive(Serialize)]
struct CanonicalUnion {
    name: String,
    variants: Vec<CanonicalUnionVariant>,
    size: usize,
    alignment: usize,
}

#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
struct CanonicalUnionVariant {
    name: String,
    type_id: TypeId,
}

#[derive(Serialize)]
struct CanonicalEnum {
    name: String,
    backing_id: TypeId,
    variants: Vec<CanonicalEnumVariant>,
    size: usize,
}

#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
struct CanonicalEnumVariant {
    name: String,
    value: i64,
}

#[derive(Serialize)]
struct CanonicalArray {
    element_type_id: TypeId,
    count: usize,
    size: usize,
}

#[derive(Serialize)]
struct CanonicalTypedef {
    name: String,
    aliased_type_id: TypeId,
}

#[derive(Serialize)]
struct CanonicalFunction {
    return_type_id: Option<TypeId>,
    parameter_type_ids: Vec<TypeId>, // order matters (calling convention)
    is_variadic: bool,
}

impl BaseTypeKind {
    /// convert to canonical form for hashing
    /// sorts enum/union variants by name
    fn to_canonical(&self) -> CanonicalTypeKind {
        match self {
            BaseTypeKind::Primitive {
                name,
                size,
                alignment,
            } => CanonicalTypeKind::Primitive(CanonicalPrimitive {
                name: name.clone(),
                size: *size,
                alignment: *alignment,
            }),

            BaseTypeKind::Struct {
                name,
                fields,
                size,
                alignment,
                is_opaque,
            } => {
                // keep field order (memory layout is order-dependent)
                let canonical_fields = fields
                    .iter()
                    .map(|f| CanonicalField {
                        name: f.name.clone(),
                        type_id: f.type_id,
                        offset: f.offset,
                        size: f.size,
                    })
                    .collect();

                CanonicalTypeKind::Struct(CanonicalStruct {
                    name: name.clone(),
                    fields: canonical_fields,
                    size: *size,
                    alignment: *alignment,
                    is_opaque: *is_opaque,
                })
            }

            BaseTypeKind::Union {
                name,
                variants,
                size,
                alignment,
            } => {
                // sort variants by name for canonical ordering
                let mut sorted_variants: Vec<_> = variants
                    .iter()
                    .map(|v| CanonicalUnionVariant {
                        name: v.name.clone(),
                        type_id: v.type_id,
                    })
                    .collect();
                sorted_variants.sort_by(|a, b| a.name.cmp(&b.name));

                CanonicalTypeKind::Union(CanonicalUnion {
                    name: name.clone(),
                    variants: sorted_variants,
                    size: *size,
                    alignment: *alignment,
                })
            }

            BaseTypeKind::Enum {
                name,
                backing_id,
                variants,
                size,
            } => {
                // sort variants by name for canonical ordering
                let mut sorted_variants: Vec<_> = variants
                    .iter()
                    .map(|v| CanonicalEnumVariant {
                        name: v.name.clone(),
                        value: v.value,
                    })
                    .collect();
                sorted_variants.sort_by(|a, b| a.name.cmp(&b.name));

                CanonicalTypeKind::Enum(CanonicalEnum {
                    name: name.clone(),
                    backing_id: *backing_id,
                    variants: sorted_variants,
                    size: *size,
                })
            }

            BaseTypeKind::Array {
                element_type_id,
                count,
                size,
            } => CanonicalTypeKind::Array(CanonicalArray {
                element_type_id: *element_type_id,
                count: *count,
                size: *size,
            }),

            BaseTypeKind::Typedef {
                name,
                aliased_type_id,
            } => CanonicalTypeKind::Typedef(CanonicalTypedef {
                name: name.clone(),
                aliased_type_id: *aliased_type_id,
            }),

            BaseTypeKind::Function {
                return_type_id,
                parameter_type_ids,
                is_variadic,
            } => {
                // keep parameter order (calling convention is order-dependent)
                CanonicalTypeKind::Function(CanonicalFunction {
                    return_type_id: *return_type_id,
                    parameter_type_ids: parameter_type_ids.clone(),
                    is_variadic: *is_variadic,
                })
            }
        }
    }
}

fn compute_type_id(
    kind: &BaseTypeKind,
    pointer_depth: usize,
    is_const: bool,
    is_volatile: bool,
) -> TypeId {
    use bincode::Options;
    use std::collections::hash_map::DefaultHasher;

    let canonical = kind.to_canonical();

    let bytes = bincode::DefaultOptions::new()
        .with_fixint_encoding() // Ensure consistent integer encoding
        .serialize(&(canonical, pointer_depth, is_const, is_volatile))
        .expect("serialization cannot fail");

    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    TypeId(hasher.finish())
}

/// central registry
#[derive(Debug, Clone)]
pub struct TypeRegistry {
    types: HashMap<TypeId, Type>,
    dwarf_to_id: HashMap<u64, TypeId>,
    name_to_ids: HashMap<String, Vec<TypeId>>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            dwarf_to_id: HashMap::new(),
            name_to_ids: HashMap::new(),
        }
    }

    /// register a new type with a content-addressed ID
    /// if an identical type already exists, returns its ID
    pub fn register_type(&mut self, mut type_: Type) -> TypeId {
        // compute content-addressed ID from type structure
        let id = compute_type_id(
            &type_.kind,
            type_.pointer_depth,
            type_.is_const,
            type_.is_volatile,
        );

        // check if already exists (automatic deduplication!)
        if self.types.contains_key(&id) {
            log::trace!("type already registered with id {:016x}", id.0);
            return id; // Same structure = same ID, already registered
        }

        type_.id = id;

        if let Some(offset) = type_.dwarf_offset {
            self.dwarf_to_id.insert(offset, id);
        }

        let name = type_.get_name();
        log::trace!("registered type {} with id {:016x}", name, id.0);

        self.name_to_ids
            .entry(name)
            .or_insert_with(Vec::new)
            .push(id);

        self.types.insert(id, type_);
        id
    }

    pub fn get_type(&self, id: TypeId) -> Option<&Type> {
        self.types.get(&id)
    }

    pub fn get_type_mut(&mut self, id: TypeId) -> Option<&mut Type> {
        self.types.get_mut(&id)
    }

    pub fn get_by_dwarf_offset(&self, offset: u64) -> Option<&Type> {
        self.dwarf_to_id
            .get(&offset)
            .and_then(|id| self.types.get(id))
    }

    pub fn get_by_name(&self, name: &str) -> Vec<&Type> {
        self.name_to_ids
            .get(name)
            .map(|ids: &Vec<TypeId>| ids.iter().filter_map(|id| self.types.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn all_types(&self) -> impl Iterator<Item = &Type> {
        self.types.values()
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// merge another registry into this one.
    pub fn merge(&mut self, other: TypeRegistry) {
        let initial_count = self.len();
        let merging_count = other.len();

        // union the types (content-addressed, so same ID = same type)
        for (id, type_) in other.types {
            self.types.entry(id).or_insert(type_);
        }

        // merge name index (deduplicate TypeIds)
        for (name, ids) in other.name_to_ids {
            let existing = self.name_to_ids.entry(name).or_insert_with(Vec::new);
            for id in ids {
                if !existing.contains(&id) {
                    existing.push(id);
                }
            }
        }

        // merge DWARF offset index
        for (offset, id) in other.dwarf_to_id {
            self.dwarf_to_id.entry(offset).or_insert(id);
        }

        let final_count = self.len();
        let added = final_count - initial_count;
        let duplicates = merging_count - added;
        log::debug!("merged type registry: {} types, {} new, {} duplicates",
                    merging_count, added, duplicates);
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Type {
    pub(crate) fn get_name(&self) -> String {
        match &self.kind {
            BaseTypeKind::Primitive { name, .. } => name.clone(),
            BaseTypeKind::Struct { name, .. } => name.clone(),
            BaseTypeKind::Union { name, .. } => name.clone(),
            BaseTypeKind::Enum { name, .. } => name.clone(),
            BaseTypeKind::Typedef { name, .. } => name.clone(),
            BaseTypeKind::Array { .. } => "<array>".to_string(),
            BaseTypeKind::Function { .. } => "<function>".to_string(),
        }
    }

    /// c code string representation
    pub fn to_c_string(&self, registry: &TypeRegistry) -> String {
        let mut base_str = match &self.kind {
            BaseTypeKind::Primitive { name, .. } => name.clone(),

            BaseTypeKind::Struct { name, .. } => format!("struct {}", name),

            BaseTypeKind::Union { name, .. } => format!("union {}", name),

            BaseTypeKind::Enum { name, .. } => name.clone(),

            BaseTypeKind::Array {
                element_type_id,
                count,
                ..
            } => {
                let elem = registry
                    .get_type(*element_type_id)
                    .map(|t| t.to_c_string(registry))
                    .unwrap_or_else(|| "void".to_string());
                format!("{}[{}]", elem, count)
            }

            BaseTypeKind::Typedef { name, .. } => name.clone(),

            BaseTypeKind::Function { .. } => "void (*)(...)".to_string(), // Simplified
        };

        if self.is_const {
            base_str = format!("const {}", base_str);
        }
        if self.is_volatile {
            base_str = format!("volatile {}", base_str);
        }

        for _ in 0..self.pointer_depth {
            base_str.push('*');
        }

        base_str
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_basic_operations() {
        let mut registry = TypeRegistry::new();

        let type_ = Type {
            id: TypeId(0), // Will be recomputed
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x1234),
        };

        let id = registry.register_type(type_);
        // Don't assert specific ID value (content-addressed)
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        // retrieve by ID
        let retrieved = registry.get_type(id).unwrap();
        assert_eq!(retrieved.id, id);
        match &retrieved.kind {
            BaseTypeKind::Primitive {
                name,
                size,
                alignment,
            } => {
                assert_eq!(name, "int");
                assert_eq!(*size, 4);
                assert_eq!(*alignment, 4);
            }
            _ => panic!("Expected primitive type"),
        }

        // by DWARF offset
        let by_offset = registry.get_by_dwarf_offset(0x1234).unwrap();
        assert_eq!(by_offset.id, id);
    }

    #[test]
    fn test_registry_multiple_types() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0), // will be recomputed
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x100),
        };

        let float_type = Type {
            id: TypeId(0), // will be recomputed
            kind: BaseTypeKind::Primitive {
                name: "float".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x200),
        };

        let int_id = registry.register_type(int_type);
        let float_id = registry.register_type(float_type);

        // don't assert specific IDs, just that they're different
        assert_ne!(int_id, float_id);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_get_by_name() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let id = registry.register_type(int_type);

        let types = registry.get_by_name("int");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].id, id);

        let no_types = registry.get_by_name("nonexistent");
        assert_eq!(no_types.len(), 0);
    }

    #[test]
    fn test_pointer_depth() {
        let mut registry = TypeRegistry::new();

        // int**
        let int_double_ptr = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 2,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let id = registry.register_type(int_double_ptr);
        let retrieved = registry.get_type(id).unwrap();
        assert_eq!(retrieved.pointer_depth, 2);
    }

    #[test]
    fn test_const_volatile_flags() {
        let mut registry = TypeRegistry::new();

        let const_int = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 1,
            is_const: true,
            is_volatile: false,
            dwarf_offset: None,
        };

        let id = registry.register_type(const_int);
        let retrieved = registry.get_type(id).unwrap();
        assert!(retrieved.is_const);
        assert!(!retrieved.is_volatile);
    }

    #[test]
    fn test_struct_type() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let int_id = registry.register_type(int_type);

        let point_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Struct {
                name: "Point".to_string(),
                fields: vec![
                    StructField {
                        name: "x".to_string(),
                        type_id: int_id,
                        offset: 0,
                        size: 4,
                    },
                    StructField {
                        name: "y".to_string(),
                        type_id: int_id,
                        offset: 4,
                        size: 4,
                    },
                ],
                size: 8,
                alignment: 4,
                is_opaque: false,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let point_id = registry.register_type(point_type);
        let retrieved = registry.get_type(point_id).unwrap();

        match &retrieved.kind {
            BaseTypeKind::Struct {
                name,
                fields,
                size,
                is_opaque,
                ..
            } => {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "x");
                assert_eq!(fields[1].name, "y");
                assert_eq!(*size, 8);
                assert!(!is_opaque);
            }
            _ => panic!("Expected struct type"),
        }
    }

    #[test]
    fn test_enum_type() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let int_id = registry.register_type(int_type);

        let status_enum = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Enum {
                name: "Status".to_string(),
                backing_id: int_id,
                variants: vec![
                    EnumVariant {
                        name: "STATUS_OK".to_string(),
                        value: 0,
                    },
                    EnumVariant {
                        name: "STATUS_ERROR".to_string(),
                        value: 1,
                    },
                ],
                size: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let enum_id = registry.register_type(status_enum);
        let retrieved = registry.get_type(enum_id).unwrap();

        match &retrieved.kind {
            BaseTypeKind::Enum {
                name,
                variants,
                backing_id: underlying_type_id,
                ..
            } => {
                assert_eq!(name, "Status");
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].name, "STATUS_OK");
                assert_eq!(variants[0].value, 0);
                assert_eq!(*underlying_type_id, int_id);
            }
            _ => panic!("Expected enum type"),
        }
    }

    #[test]
    fn test_array_type() {
        let mut registry = TypeRegistry::new();

        let char_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "char".to_string(),
                size: 1,
                alignment: 1,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let char_id = registry.register_type(char_type);

        let char_array = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Array {
                element_type_id: char_id,
                count: 64,
                size: 64,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let array_id = registry.register_type(char_array);
        let retrieved = registry.get_type(array_id).unwrap();

        match &retrieved.kind {
            BaseTypeKind::Array {
                element_type_id,
                count,
                size,
            } => {
                assert_eq!(*element_type_id, char_id);
                assert_eq!(*count, 64);
                assert_eq!(*size, 64);
            }
            _ => panic!("Expected array type"),
        }
    }

    #[test]
    fn test_typedef() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let int_id = registry.register_type(int_type);

        let size_t_typedef = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Typedef {
                name: "size_t".to_string(),
                aliased_type_id: int_id,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let typedef_id = registry.register_type(size_t_typedef);
        let retrieved = registry.get_type(typedef_id).unwrap();

        match &retrieved.kind {
            BaseTypeKind::Typedef {
                name,
                aliased_type_id,
            } => {
                assert_eq!(name, "size_t");
                assert_eq!(*aliased_type_id, int_id);
            }
            _ => panic!("Expected typedef"),
        }
    }

    #[test]
    fn test_merge_registries() {
        let mut registry1 = TypeRegistry::new();
        let mut registry2 = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x100),
        };
        registry1.register_type(int_type);

        let float_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "float".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x200),
        };
        registry2.register_type(float_type);

        registry1.merge(registry2);

        assert_eq!(registry1.len(), 2);
        assert!(registry1.get_by_name("int").len() == 1);
        assert!(registry1.get_by_name("float").len() == 1);
    }

    #[test]
    fn test_merge_with_references() {
        let mut registry1 = TypeRegistry::new();
        let mut registry2 = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let int_id = registry2.register_type(int_type);

        let point_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Struct {
                name: "Point".to_string(),
                fields: vec![StructField {
                    name: "x".to_string(),
                    type_id: int_id,
                    offset: 0,
                    size: 4,
                }],
                size: 4,
                alignment: 4,
                is_opaque: false,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        registry2.register_type(point_type);

        // Merge registry2 into registry1
        registry1.merge(registry2);

        assert_eq!(registry1.len(), 2);

        let point_types = registry1.get_by_name("Point");
        assert_eq!(point_types.len(), 1);

        // With content-addressing, the field's type_id should match int_id
        // because same type = same ID everywhere
        match &point_types[0].kind {
            BaseTypeKind::Struct { fields, .. } => {
                let field_type_id = fields[0].type_id;
                assert_eq!(field_type_id, int_id); // Same ID!

                let field_type = registry1.get_type(field_type_id).unwrap();
                match &field_type.kind {
                    BaseTypeKind::Primitive { name, .. } => {
                        assert_eq!(name, "int");
                    }
                    _ => panic!("Expected int primitive"),
                }
            }
            _ => panic!("Expected struct"),
        }
    }

    #[test]
    fn test_all_types_iterator() {
        let mut registry = TypeRegistry::new();

        registry.register_type(Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        });

        registry.register_type(Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "float".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        });

        let count = registry.all_types().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_deduplication_same_primitive_twice() {
        let mut registry = TypeRegistry::new();

        let int_type1 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x100),
        };

        let int_type2 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x200), // different DWARF offset
        };

        let id1 = registry.register_type(int_type1);
        let id2 = registry.register_type(int_type2);

        assert_eq!(id1, id2);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_deduplication_same_struct_twice() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let int_id = registry.register_type(int_type);

        let point1 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Struct {
                name: "Point".to_string(),
                fields: vec![
                    StructField {
                        name: "x".to_string(),
                        type_id: int_id,
                        offset: 0,
                        size: 4,
                    },
                    StructField {
                        name: "y".to_string(),
                        type_id: int_id,
                        offset: 4,
                        size: 4,
                    },
                ],
                size: 8,
                alignment: 4,
                is_opaque: false,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x1000),
        };

        let point2 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Struct {
                name: "Point".to_string(),
                fields: vec![
                    StructField {
                        name: "x".to_string(),
                        type_id: int_id,
                        offset: 0,
                        size: 4,
                    },
                    StructField {
                        name: "y".to_string(),
                        type_id: int_id,
                        offset: 4,
                        size: 4,
                    },
                ],
                size: 8,
                alignment: 4,
                is_opaque: false,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x2000), // different offset
        };

        let id1 = registry.register_type(point1);
        let id2 = registry.register_type(point2);

        assert_eq!(id1, id2);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_deduplication_same_enum_twice() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let int_id = registry.register_type(int_type);

        let enum1 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Enum {
                name: "Status".to_string(),
                backing_id: int_id,
                variants: vec![
                    EnumVariant {
                        name: "OK".to_string(),
                        value: 0,
                    },
                    EnumVariant {
                        name: "ERROR".to_string(),
                        value: 1,
                    },
                ],
                size: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x1000),
        };

        let enum2 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Enum {
                name: "Status".to_string(),
                backing_id: int_id,
                variants: vec![
                    EnumVariant {
                        name: "OK".to_string(),
                        value: 0,
                    },
                    EnumVariant {
                        name: "ERROR".to_string(),
                        value: 1,
                    },
                ],
                size: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x2000),
        };

        let id1 = registry.register_type(enum1);
        let id2 = registry.register_type(enum2);

        assert_eq!(id1, id2);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_no_deduplication_different_types() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let float_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "float".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let int_id = registry.register_type(int_type);
        let float_id = registry.register_type(float_type);

        assert_ne!(int_id, float_id);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_enum_variant_order_independence() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let int_id = registry.register_type(int_type);

        // order of enum variants: [OK, ERROR]
        let enum1 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Enum {
                name: "Status".to_string(),
                backing_id: int_id,
                variants: vec![
                    EnumVariant {
                        name: "OK".to_string(),
                        value: 0,
                    },
                    EnumVariant {
                        name: "ERROR".to_string(),
                        value: 1,
                    },
                ],
                size: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        // order of enum variants: [ERROR, OK]
        let enum2 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Enum {
                name: "Status".to_string(),
                backing_id: int_id,
                variants: vec![
                    EnumVariant {
                        name: "ERROR".to_string(),
                        value: 1,
                    },
                    EnumVariant {
                        name: "OK".to_string(),
                        value: 0,
                    },
                ],
                size: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let id1 = registry.register_type(enum1);
        let id2 = registry.register_type(enum2);

        // order does not matter
        assert_eq!(id1, id2);
        assert_eq!(registry.len(), 2); // int + Status
    }

    #[test]
    fn test_union_variant_order_independence() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let int_id = registry.register_type(int_type);

        let float_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "float".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let float_id = registry.register_type(float_type);

        // variants in order: [as_int, as_float]
        let union1 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Union {
                name: "DataUnion".to_string(),
                variants: vec![
                    UnionField {
                        name: "as_int".to_string(),
                        type_id: int_id,
                    },
                    UnionField {
                        name: "as_float".to_string(),
                        type_id: float_id,
                    },
                ],
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        // variants in different order: [as_float, as_int]
        let union2 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Union {
                name: "DataUnion".to_string(),
                variants: vec![
                    UnionField {
                        name: "as_float".to_string(),
                        type_id: float_id,
                    },
                    UnionField {
                        name: "as_int".to_string(),
                        type_id: int_id,
                    },
                ],
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let id1 = registry.register_type(union1);
        let id2 = registry.register_type(union2);

        // order does not matter - canonical form sorts by name
        assert_eq!(id1, id2);
        // int, float, DataUnion
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn test_struct_field_order_dependence() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let int_id = registry.register_type(int_type);

        // struct with fields [x, y]
        let struct1 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Struct {
                name: "Point".to_string(),
                fields: vec![
                    StructField {
                        name: "x".to_string(),
                        type_id: int_id,
                        offset: 0,
                        size: 4,
                    },
                    StructField {
                        name: "y".to_string(),
                        type_id: int_id,
                        offset: 4,
                        size: 4,
                    },
                ],
                size: 8,
                alignment: 4,
                is_opaque: false,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        // struct with fields in DIFFERENT order: [y, x]
        let struct2 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Struct {
                name: "Point".to_string(),
                fields: vec![
                    StructField {
                        name: "y".to_string(),
                        type_id: int_id,
                        offset: 0, // Different offset!
                        size: 4,
                    },
                    StructField {
                        name: "x".to_string(),
                        type_id: int_id,
                        offset: 4,
                        size: 4,
                    },
                ],
                size: 8,
                alignment: 4,
                is_opaque: false,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let id1 = registry.register_type(struct1);
        let id2 = registry.register_type(struct2);

        // field order matters for structs (memory layout)
        assert_ne!(id1, id2);
        // int, Point(x,y), Point(y,x)
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn test_function_param_order_dependence() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let int_id = registry.register_type(int_type);

        let float_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "float".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let float_id = registry.register_type(float_type);

        // function(int, float)
        let func1 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Function {
                return_type_id: None,
                parameter_type_ids: vec![int_id, float_id],
                is_variadic: false,
            },
            pointer_depth: 1, // Function pointer
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        // function(float, int)
        let func2 = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Function {
                return_type_id: None,
                parameter_type_ids: vec![float_id, int_id],
                is_variadic: false,
            },
            pointer_depth: 1,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let id1 = registry.register_type(func1);
        let id2 = registry.register_type(func2);

        // parameter order matters
        assert_ne!(id1, id2);
        // int, float, func1, func2
        assert_eq!(registry.len(), 4);
    }

    #[test]
    fn test_merge_complete_overlap() {
        let mut registry1 = TypeRegistry::new();
        let mut registry2 = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x100),
        };

        let float_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "float".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: Some(0x200),
        };

        // both registries have the same types
        registry1.register_type(int_type.clone());
        registry1.register_type(float_type.clone());

        registry2.register_type(int_type);
        registry2.register_type(float_type);

        assert_eq!(registry1.len(), 2);
        assert_eq!(registry2.len(), 2);

        registry1.merge(registry2);

        // no duplication - still only 2 types
        assert_eq!(registry1.len(), 2);
        assert_eq!(registry1.get_by_name("int").len(), 1);
        assert_eq!(registry1.get_by_name("float").len(), 1);
    }

    #[test]
    fn test_merge_partial_overlap() {
        let mut registry1 = TypeRegistry::new();
        let mut registry2 = TypeRegistry::new();

        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let float_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "float".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        let double_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "double".to_string(),
                size: 8,
                alignment: 8,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        // registry1 has int, float
        registry1.register_type(int_type.clone());
        registry1.register_type(float_type.clone());

        // registry2 has float, double (float is shared)
        registry2.register_type(float_type);
        registry2.register_type(double_type);

        assert_eq!(registry1.len(), 2);
        assert_eq!(registry2.len(), 2);

        registry1.merge(registry2);

        // int, float, double
        assert_eq!(registry1.len(), 3);
        assert_eq!(registry1.get_by_name("int").len(), 1);
        assert_eq!(registry1.get_by_name("float").len(), 1);
        assert_eq!(registry1.get_by_name("double").len(), 1);
    }

    #[test]
    fn test_merge_preserves_references() {
        let mut registry1 = TypeRegistry::new();
        let mut registry2 = TypeRegistry::new();

        // register int in registry2
        let int_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        let int_id_reg2 = registry2.register_type(int_type.clone());

        // register struct in registry2 that references int
        let point_type = Type {
            id: TypeId(0),
            kind: BaseTypeKind::Struct {
                name: "Point".to_string(),
                fields: vec![StructField {
                    name: "x".to_string(),
                    type_id: int_id_reg2,
                    offset: 0,
                    size: 4,
                }],
                size: 4,
                alignment: 4,
                is_opaque: false,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };
        registry2.register_type(point_type);

        // register int in registry1 independently
        let int_id_reg1 = registry1.register_type(int_type);

        // before merge
        assert_eq!(registry2.len(), 2);

        // Merge
        registry1.merge(registry2);

        // int + Point
        assert_eq!(registry1.len(), 2);

        // TypeIds match because content-addressing
        assert_eq!(int_id_reg1, int_id_reg2);

        // Point still references correct int TypeId
        let point_types = registry1.get_by_name("Point");
        assert_eq!(point_types.len(), 1);

        match &point_types[0].kind {
            BaseTypeKind::Struct { fields, .. } => {
                assert_eq!(fields[0].type_id, int_id_reg1);
                assert_eq!(fields[0].type_id, int_id_reg2);
            }
            _ => panic!("Expected struct"),
        }
    }
}
