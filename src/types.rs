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
