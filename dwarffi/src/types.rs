use crate::type_registry::{TypeId, TypeRegistry};

/// c function parameters have a name and a type
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_id: TypeId,
}

/// struct to hold a complete function signature
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub return_type_id: TypeId,
    pub parameters: Vec<Parameter>,
    pub is_variadic: bool,
    pub is_exported: bool,
}

impl FunctionSignature {
    /// format the function signature as a C-style declaration
    pub fn to_string(&self, registry: &TypeRegistry) -> String {
        // Resolve return type
        let return_type_str = registry
            .get_type(self.return_type_id)
            .map(|t| t.to_c_string(registry))
            .unwrap_or_else(|| "void".to_string());

        let params = if self.parameters.is_empty() {
            "void".to_string()
        } else {
            let param_strings: Vec<String> = self
                .parameters
                .iter()
                .map(|p| {
                    let type_str = registry
                        .get_type(p.type_id)
                        .map(|t| t.to_c_string(registry))
                        .unwrap_or_else(|| "void".to_string());

                    if p.name.is_empty() {
                        type_str
                    } else {
                        format!("{} {}", type_str, p.name)
                    }
                })
                .collect();

            if self.is_variadic {
                format!("{}, ...", param_strings.join(", "))
            } else {
                param_strings.join(", ")
            }
        };

        format!("{} {}({})", return_type_str, self.name, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type_registry::{BaseTypeKind, Type};

    fn create_test_registry() -> TypeRegistry {
        let mut registry = TypeRegistry::new();

        // Register void
        registry.register_type(Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "void".to_string(),
                size: 0,
                alignment: 1,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        });

        // Register int
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

        // Register char
        registry.register_type(Type {
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
        });

        // Register const char*
        registry.register_type(Type {
            id: TypeId(0),
            kind: BaseTypeKind::Primitive {
                name: "char".to_string(),
                size: 1,
                alignment: 1,
            },
            pointer_depth: 1,
            is_const: true,
            is_volatile: false,
            dwarf_offset: None,
        });

        // Register Point struct
        registry.register_type(Type {
            id: TypeId(0),
            kind: BaseTypeKind::Struct {
                name: "Point".to_string(),
                fields: vec![],
                size: 8,
                alignment: 4,
                is_opaque: false,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        });

        registry
    }

    fn get_type_id(
        registry: &TypeRegistry,
        name: &str,
        pointer_depth: usize,
        is_const: bool,
    ) -> TypeId {
        registry
            .all_types()
            .find(|t| {
                let name_matches = match &t.kind {
                    BaseTypeKind::Primitive {
                        name: type_name, ..
                    } => type_name == name,
                    BaseTypeKind::Struct {
                        name: type_name, ..
                    } => type_name == name,
                    _ => false,
                };
                name_matches && t.pointer_depth == pointer_depth && t.is_const == is_const
            })
            .map(|t| t.id)
            .unwrap_or_else(|| {
                panic!(
                    "Type not found: {} (ptr={}, const={})",
                    name, pointer_depth, is_const
                )
            })
    }

    #[test]
    fn test_void_function_no_params() {
        let registry = create_test_registry();
        let void_id = get_type_id(&registry, "void", 0, false);

        let sig = FunctionSignature {
            name: "test_func".to_string(),
            return_type_id: void_id,
            parameters: vec![],
            is_variadic: false,
            is_exported: true,
        };

        assert_eq!(sig.to_string(&registry), "void test_func(void)");
    }

    #[test]
    fn test_function_with_single_param() {
        let registry = create_test_registry();
        let int_id = get_type_id(&registry, "int", 0, false);

        let sig = FunctionSignature {
            name: "test_func".to_string(),
            return_type_id: int_id,
            parameters: vec![Parameter {
                name: "x".to_string(),
                type_id: int_id,
            }],
            is_variadic: false,
            is_exported: true,
        };

        assert_eq!(sig.to_string(&registry), "int test_func(int x)");
    }

    #[test]
    fn test_function_with_multiple_params() {
        let registry = create_test_registry();
        let int_id = get_type_id(&registry, "int", 0, false);

        let sig = FunctionSignature {
            name: "add".to_string(),
            return_type_id: int_id,
            parameters: vec![
                Parameter {
                    name: "a".to_string(),
                    type_id: int_id,
                },
                Parameter {
                    name: "b".to_string(),
                    type_id: int_id,
                },
            ],
            is_variadic: false,
            is_exported: true,
        };

        assert_eq!(sig.to_string(&registry), "int add(int a, int b)");
    }

    #[test]
    fn test_variadic_function() {
        let registry = create_test_registry();
        let int_id = get_type_id(&registry, "int", 0, false);
        let const_char_ptr_id = get_type_id(&registry, "char", 1, true);

        let sig = FunctionSignature {
            name: "printf".to_string(),
            return_type_id: int_id,
            parameters: vec![Parameter {
                name: "format".to_string(),
                type_id: const_char_ptr_id,
            }],
            is_variadic: true,
            is_exported: true,
        };

        assert_eq!(
            sig.to_string(&registry),
            "int printf(const char* format, ...)"
        );
    }

    #[test]
    fn test_parameter_without_name() {
        let registry = create_test_registry();
        let void_id = get_type_id(&registry, "void", 0, false);
        let int_id = get_type_id(&registry, "int", 0, false);

        let sig = FunctionSignature {
            name: "test_func".to_string(),
            return_type_id: void_id,
            parameters: vec![Parameter {
                name: "".to_string(),
                type_id: int_id,
            }],
            is_variadic: false,
            is_exported: false,
        };

        assert_eq!(sig.to_string(&registry), "void test_func(int)");
    }

    #[test]
    fn test_pointer_return_type() {
        let registry = create_test_registry();
        let const_char_ptr_id = get_type_id(&registry, "char", 1, true);

        let sig = FunctionSignature {
            name: "get_string".to_string(),
            return_type_id: const_char_ptr_id,
            parameters: vec![],
            is_variadic: false,
            is_exported: true,
        };

        assert_eq!(sig.to_string(&registry), "const char* get_string(void)");
    }

    #[test]
    fn test_struct_return_type() {
        let registry = create_test_registry();
        let point_id = get_type_id(&registry, "Point", 0, false);
        let int_id = get_type_id(&registry, "int", 0, false);

        let sig = FunctionSignature {
            name: "create_point".to_string(),
            return_type_id: point_id,
            parameters: vec![
                Parameter {
                    name: "x".to_string(),
                    type_id: int_id,
                },
                Parameter {
                    name: "y".to_string(),
                    type_id: int_id,
                },
            ],
            is_variadic: false,
            is_exported: true,
        };

        assert_eq!(
            sig.to_string(&registry),
            "struct Point create_point(int x, int y)"
        );
    }
}
