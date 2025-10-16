/// type registry for storing and managing C type information extracted from DWARF
use std::collections::HashMap;

pub type TypeId = usize;

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
        is_opaque: bool, // True if forward declaration only
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
    pub offset: usize, // Offset in bytes from struct start
    pub size: usize,   // Size in bytes
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

/// central registry
#[derive(Debug, Clone)]
pub struct TypeRegistry {
    types: HashMap<TypeId, Type>,
    dwarf_to_id: HashMap<u64, TypeId>,
    name_to_ids: HashMap<String, Vec<TypeId>>,
    next_id: TypeId,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            dwarf_to_id: HashMap::new(),
            name_to_ids: HashMap::new(),
            next_id: 0,
        }
    }

    /// register a new type with an incremented ID
    pub fn register_type(&mut self, mut type_: Type) -> TypeId {
        let id = self.next_id;
        self.next_id += 1;

        type_.id = id;

        if let Some(offset) = type_.dwarf_offset {
            self.dwarf_to_id.insert(offset, id);
        }

        let name = type_.get_name();
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
            .map(|ids| ids.iter().filter_map(|id| self.types.get(id)).collect())
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

    pub fn merge(&mut self, other: TypeRegistry) -> HashMap<TypeId, TypeId> {
        let mut id_map = HashMap::new();

        // sort other before merging
        let mut other_types: Vec<_> = other.types.into_iter().collect();
        other_types.sort_by_key(|(_, t)| t.id);

        for (old_id, mut type_) in other_types {
            // Remap referenced type IDs
            type_.remap_type_ids(&id_map);

            let new_id = self.register_type(type_);
            id_map.insert(old_id, new_id);
        }

        id_map
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

    fn remap_type_ids(&mut self, id_map: &HashMap<TypeId, TypeId>) {
        match &mut self.kind {
            BaseTypeKind::Struct { fields, .. } => {
                for field in fields {
                    if let Some(&new_id) = id_map.get(&field.type_id) {
                        field.type_id = new_id;
                    }
                }
            }
            BaseTypeKind::Union { variants, .. } => {
                for variant in variants {
                    if let Some(&new_id) = id_map.get(&variant.type_id) {
                        variant.type_id = new_id;
                    }
                }
            }
            BaseTypeKind::Enum {
                backing_id: underlying_type_id,
                ..
            } => {
                if let Some(&new_id) = id_map.get(underlying_type_id) {
                    *underlying_type_id = new_id;
                }
            }
            BaseTypeKind::Array {
                element_type_id, ..
            } => {
                if let Some(&new_id) = id_map.get(element_type_id) {
                    *element_type_id = new_id;
                }
            }
            BaseTypeKind::Typedef {
                aliased_type_id, ..
            } => {
                if let Some(&new_id) = id_map.get(aliased_type_id) {
                    *aliased_type_id = new_id;
                }
            }
            BaseTypeKind::Function {
                return_type_id,
                parameter_type_ids,
                ..
            } => {
                if let Some(ret_id) = return_type_id {
                    if let Some(&new_id) = id_map.get(ret_id) {
                        *ret_id = new_id;
                    }
                }
                for param_id in parameter_type_ids {
                    if let Some(&new_id) = id_map.get(param_id) {
                        *param_id = new_id;
                    }
                }
            }
            BaseTypeKind::Primitive { .. } => {
                // no type references in primitives
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_basic_operations() {
        let mut registry = TypeRegistry::new();

        let type_ = Type {
            id: 0,
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
        assert_eq!(id, 0);
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
            id: 0,
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
            id: 0,
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

        assert_eq!(int_id, 0);
        assert_eq!(float_id, 1);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_get_by_name() {
        let mut registry = TypeRegistry::new();

        let int_type = Type {
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
    /// TODO FIXME!! this one is flaky. i think has to do with how we use the
    /// integer and pass it in and out when we register, I'm pretty sure. we
    /// need an atomic + thread safe ID instead of a simple usize.
    fn test_merge_with_references() {
        let mut registry1 = TypeRegistry::new();
        let mut registry2 = TypeRegistry::new();

        let int_type = Type {
            id: 0,
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
            id: 0,
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

        let id_map = registry1.merge(registry2);

        assert_eq!(registry1.len(), 2);

        let point_types = registry1.get_by_name("Point");
        assert_eq!(point_types.len(), 1);

        match &point_types[0].kind {
            BaseTypeKind::Struct { fields, .. } => {
                let field_type_id = fields[0].type_id;
                let new_int_id = id_map.get(&int_id).unwrap();
                assert_eq!(field_type_id, *new_int_id);

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
            id: 0,
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
            id: 0,
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
}
