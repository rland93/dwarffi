/// c function parameters have a name and a type
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_name: String,
}

/// struct to hold a complete function signature
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<Parameter>,
    pub is_variadic: bool,
    pub is_exported: bool,
}

impl FunctionSignature {
    /// format the function signature as a C-style declaration
    pub fn to_string(&self) -> String {
        let params = if self.parameters.is_empty() {
            "void".to_string()
        } else {
            let param_strings: Vec<String> = self
                .parameters
                .iter()
                .map(|p| {
                    if p.name.is_empty() {
                        p.type_name.clone()
                    } else {
                        format!("{} {}", p.type_name, p.name)
                    }
                })
                .collect();

            if self.is_variadic {
                format!("{}, ...", param_strings.join(", "))
            } else {
                param_strings.join(", ")
            }
        };

        format!("{} {}({})", self.return_type, self.name, params)
    }
}

/// information about a resolved type from DWARF
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub is_const: bool,
    pub is_volatile: bool,
}

impl TypeInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            is_const: false,
            is_volatile: false,
        }
    }

    pub fn with_const(mut self) -> Self {
        self.is_const = true;
        self
    }

    pub fn with_volatile(mut self) -> Self {
        self.is_volatile = true;
        self
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();

        if self.is_const {
            result.push_str("const ");
        }
        if self.is_volatile {
            result.push_str("volatile ");
        }

        result.push_str(&self.name);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_void_function_no_params() {
        let sig = FunctionSignature {
            name: "test_func".to_string(),
            return_type: "void".to_string(),
            parameters: vec![],
            is_variadic: false,
            is_exported: true,
        };

        assert_eq!(sig.to_string(), "void test_func(void)");
    }

    #[test]
    fn test_function_with_single_param() {
        let sig = FunctionSignature {
            name: "test_func".to_string(),
            return_type: "int".to_string(),
            parameters: vec![Parameter {
                name: "x".to_string(),
                type_name: "int".to_string(),
            }],
            is_variadic: false,
            is_exported: true,
        };

        assert_eq!(sig.to_string(), "int test_func(int x)");
    }

    #[test]
    fn test_function_with_multiple_params() {
        let sig = FunctionSignature {
            name: "add".to_string(),
            return_type: "int".to_string(),
            parameters: vec![
                Parameter {
                    name: "a".to_string(),
                    type_name: "int".to_string(),
                },
                Parameter {
                    name: "b".to_string(),
                    type_name: "int".to_string(),
                },
            ],
            is_variadic: false,
            is_exported: true,
        };

        assert_eq!(sig.to_string(), "int add(int a, int b)");
    }

    #[test]
    fn test_variadic_function() {
        let sig = FunctionSignature {
            name: "printf".to_string(),
            return_type: "int".to_string(),
            parameters: vec![Parameter {
                name: "format".to_string(),
                type_name: "const char*".to_string(),
            }],
            is_variadic: true,
            is_exported: true,
        };

        assert_eq!(sig.to_string(), "int printf(const char* format, ...)");
    }

    #[test]
    fn test_parameter_without_name() {
        let sig = FunctionSignature {
            name: "test_func".to_string(),
            return_type: "void".to_string(),
            parameters: vec![Parameter {
                name: "".to_string(),
                type_name: "int".to_string(),
            }],
            is_variadic: false,
            is_exported: false,
        };

        assert_eq!(sig.to_string(), "void test_func(int)");
    }

    #[test]
    fn test_pointer_return_type() {
        let sig = FunctionSignature {
            name: "get_string".to_string(),
            return_type: "const char*".to_string(),
            parameters: vec![],
            is_variadic: false,
            is_exported: true,
        };

        assert_eq!(sig.to_string(), "const char* get_string(void)");
    }

    #[test]
    fn test_struct_return_type() {
        let sig = FunctionSignature {
            name: "create_point".to_string(),
            return_type: "Point".to_string(),
            parameters: vec![
                Parameter {
                    name: "x".to_string(),
                    type_name: "int".to_string(),
                },
                Parameter {
                    name: "y".to_string(),
                    type_name: "int".to_string(),
                },
            ],
            is_variadic: false,
            is_exported: true,
        };

        assert_eq!(sig.to_string(), "Point create_point(int x, int y)");
    }

    #[test]
    fn test_type_info_plain() {
        let ti = TypeInfo::new("int".to_string());
        assert_eq!(ti.to_string(), "int");
    }

    #[test]
    fn test_type_info_const() {
        let ti = TypeInfo::new("char*".to_string()).with_const();
        assert_eq!(ti.to_string(), "const char*");
    }

    #[test]
    fn test_type_info_volatile() {
        let ti = TypeInfo::new("int".to_string()).with_volatile();
        assert_eq!(ti.to_string(), "volatile int");
    }

    #[test]
    fn test_type_info_const_volatile() {
        let ti = TypeInfo::new("int".to_string())
            .with_const()
            .with_volatile();
        assert_eq!(ti.to_string(), "const volatile int");
    }
}
