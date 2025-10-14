use crate::type_registry::{BaseTypeKind, Type, TypeId, TypeRegistry};
use anyhow::{Result, anyhow};
use gimli::{AttributeValue, DebuggingInformationEntry, Dwarf, ReaderOffset, Unit, UnitOffset};
use std::collections::HashMap;

/// resolve DWARF type information into human-readable C type strings
pub struct TypeResolver<'dwarf, R: gimli::Reader> {
    dwarf: &'dwarf Dwarf<R>,
    unit: &'dwarf Unit<R>,
    /// old: string based
    type_cache: HashMap<UnitOffset<R::Offset>, String>,
    /// new: structured
    type_registry: TypeRegistry,
}

impl<'dwarf, R: gimli::Reader> TypeResolver<'dwarf, R> {
    /// create new with empty cache with the given DWARF and unit
    pub fn new(dwarf: &'dwarf Dwarf<R>, unit: &'dwarf Unit<R>) -> Self {
        Self {
            dwarf,
            unit,
            type_cache: HashMap::new(),
            type_registry: TypeRegistry::new(),
        }
    }

    /// resolve a type from a DIE offset
    pub fn resolve_type(&mut self, offset: UnitOffset<R::Offset>) -> Result<String> {
        // check to see if it hasn't already been resolved
        if let Some(cached) = self.type_cache.get(&offset) {
            log::trace!(
                "cache hit for offset @{:#010x}: {}",
                offset.0.into_u64(),
                cached
            );
            return Ok(cached.clone());
        }

        let mut entries = self.unit.entries_at_offset(offset)?;
        let (_, entry) = entries
            .next_dfs()?
            .ok_or_else(|| anyhow!("no entry at offset"))?;

        let type_name = self.resolve_type_entry(entry)?;

        // add new type to cache
        self.type_cache.insert(offset, type_name.clone());
        log::debug!(
            "{:>12} {:#010x}: {}",
            "type",
            offset.0.into_u64(),
            type_name
        );

        // NEW: Also build structured type registry entry
        let _ = self.build_type_registry_entry(offset);

        Ok(type_name)
    }

    /// resolve a type from a DIE entry
    fn resolve_type_entry(&mut self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        match entry.tag() {
            gimli::DW_TAG_base_type => self.resolve_base_type(entry),
            gimli::DW_TAG_pointer_type => self.resolve_pointer_type(entry),
            gimli::DW_TAG_const_type => self.resolve_const_type(entry),
            gimli::DW_TAG_volatile_type => self.resolve_volatile_type(entry),
            gimli::DW_TAG_typedef => self.resolve_typedef(entry),
            gimli::DW_TAG_structure_type => self.resolve_structure_type(entry),
            gimli::DW_TAG_union_type => self.resolve_union_type(entry),
            gimli::DW_TAG_enumeration_type => self.resolve_enumeration_type(entry),
            gimli::DW_TAG_array_type => self.resolve_array_type(entry),
            gimli::DW_TAG_subroutine_type => self.resolve_subroutine_type(entry),
            gimli::DW_TAG_unspecified_parameters => Ok("...".to_string()),
            _ => Ok(format!("<unknown:{}>", entry.tag())),
        }
    }

    /// resolve a base type (e.g., int, float)
    fn resolve_base_type(&self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        let name = self.get_name(entry)?;
        Ok(name)
    }

    /// resolve a pointer type (e.g., int*)
    fn resolve_pointer_type(&mut self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        if let Some(type_attr) = entry.attr(gimli::DW_AT_type)? {
            if let AttributeValue::UnitRef(offset) = type_attr.value() {
                let base_type = self.resolve_type(offset)?;
                return Ok(format!("{}*", base_type));
            }
        }

        // void pointer if no type specified
        Ok("void*".to_string())
    }

    /// resolve a const type (e.g., const int)
    fn resolve_const_type(&mut self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        if let Some(type_attr) = entry.attr(gimli::DW_AT_type)? {
            if let AttributeValue::UnitRef(offset) = type_attr.value() {
                let base_type = self.resolve_type(offset)?;
                return Ok(format!("const {}", base_type));
            }
        }

        // const void if no type specified
        Ok("const void".to_string())
    }

    /// resolve a volatile type (e.g., volatile int)
    fn resolve_volatile_type(&mut self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        if let Some(type_attr) = entry.attr(gimli::DW_AT_type)? {
            if let AttributeValue::UnitRef(offset) = type_attr.value() {
                let base_type = self.resolve_type(offset)?;
                return Ok(format!("volatile {}", base_type));
            }
        }

        // volatile void if no type specified
        Ok("volatile void".to_string())
    }

    /// resolve a typedef (e.g., typedef int my_int;)
    fn resolve_typedef(&mut self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        // first try to get the name
        if let Ok(name) = self.get_name(entry) {
            return Ok(name);
        }

        // then try to get the underlying type, if no name is specified
        if let Some(type_attr) = entry.attr(gimli::DW_AT_type)? {
            if let AttributeValue::UnitRef(offset) = type_attr.value() {
                return self.resolve_type(offset);
            }
        }

        Ok("void".to_string())
    }

    /// resolve a structure type (e.g., struct my_struct)
    fn resolve_structure_type(&self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        if let Ok(name) = self.get_name(entry) {
            Ok(format!("struct {}", name))
        } else {
            Ok("struct <anonymous>".to_string())
        }
    }

    /// resolve a union type (e.g., union my_union)
    fn resolve_union_type(&self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        if let Ok(name) = self.get_name(entry) {
            Ok(format!("union {}", name))
        } else {
            Ok("union <anonymous>".to_string())
        }
    }

    /// resolve an enumeration type (e.g., enum my_enum)
    fn resolve_enumeration_type(&self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        if let Ok(name) = self.get_name(entry) {
            Ok(name)
        } else {
            Ok("enum <anonymous>".to_string())
        }
    }

    /// resolve an array type (e.g., int[])
    /// TODO FIXME!! arrays shown as pointer types for now
    fn resolve_array_type(&mut self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        if let Some(type_attr) = entry.attr(gimli::DW_AT_type)? {
            if let AttributeValue::UnitRef(offset) = type_attr.value() {
                let base_type = self.resolve_type(offset)?;
                return Ok(format!("{}*", base_type));
            }
        }

        Ok("void*".to_string())
    }

    /// resolve a subroutine type (e.g., function pointer)
    /// TODO FIXME!!all function pointers are kept generic
    fn resolve_subroutine_type(&self, _entry: &DebuggingInformationEntry<R>) -> Result<String> {
        Ok("void (*)(...)".to_string())
    }

    /// helper to get name attribute
    /// TODO FIXME!! unify this with logic in dwarf_analyzer module
    fn get_name(&self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        if let Some(attr) = entry.attr(gimli::DW_AT_name)? {
            let name_reader = self.dwarf.attr_string(self.unit, attr.value())?;
            let bytes = name_reader.to_slice()?;
            let name_str = String::from_utf8(bytes.to_vec())?;
            return Ok(name_str);
        }

        Err(anyhow!("no name attribute"))
    }

    /// return the current length of the cache
    pub fn cache_len(&self) -> usize {
        self.type_cache.len()
    }

    fn build_type_registry_entry(&mut self, offset: UnitOffset<R::Offset>) -> Result<TypeId> {
        let dwarf_offset = offset.0.into_u64();

        if let Some(type_) = self.type_registry.get_by_dwarf_offset(dwarf_offset) {
            return Ok(type_.id);
        }

        let mut entries = self.unit.entries_at_offset(offset)?;
        let (_, entry) = entries
            .next_dfs()?
            .ok_or_else(|| anyhow!("no entry at offset"))?;

        let (kind, pointer_depth, is_const, is_volatile) =
            self.extract_type_metadata(entry, offset)?;

        let extracted_type = Type {
            id: 0,
            kind,
            pointer_depth,
            is_const,
            is_volatile,
            dwarf_offset: Some(dwarf_offset),
        };

        let id = self.type_registry.register_type(extracted_type);
        Ok(id)
    }

    fn extract_type_metadata(
        &mut self,
        _entry: &DebuggingInformationEntry<R>,
        offset: UnitOffset<R::Offset>,
    ) -> Result<(BaseTypeKind, usize, bool, bool)> {
        let mut pointer_depth = 0;
        let mut is_const = false;
        let mut is_volatile = false;
        let mut current_offset = offset;

        loop {
            let mut entries = self.unit.entries_at_offset(current_offset)?;
            let (_, entry) = entries
                .next_dfs()?
                .ok_or_else(|| anyhow!("no entry at offset"))?;

            match entry.tag() {
                gimli::DW_TAG_pointer_type => {
                    pointer_depth += 1;
                    // Follow to pointee
                    if let Some(attr) = entry.attr(gimli::DW_AT_type)? {
                        if let AttributeValue::UnitRef(next_offset) = attr.value() {
                            current_offset = next_offset;
                            continue;
                        }
                    }
                    // void* if no type attribute
                    let kind = BaseTypeKind::Primitive {
                        name: "void".to_string(),
                        size: 0,
                        alignment: 1,
                    };
                    return Ok((kind, pointer_depth, is_const, is_volatile));
                }

                gimli::DW_TAG_const_type => {
                    is_const = true;
                    // Follow to inner type
                    if let Some(attr) = entry.attr(gimli::DW_AT_type)? {
                        if let AttributeValue::UnitRef(next_offset) = attr.value() {
                            current_offset = next_offset;
                            continue;
                        }
                    }
                    // const void if no type
                    let kind = BaseTypeKind::Primitive {
                        name: "void".to_string(),
                        size: 0,
                        alignment: 1,
                    };
                    return Ok((kind, pointer_depth, is_const, is_volatile));
                }

                gimli::DW_TAG_volatile_type => {
                    is_volatile = true;
                    // Follow to inner type
                    if let Some(attr) = entry.attr(gimli::DW_AT_type)? {
                        if let AttributeValue::UnitRef(next_offset) = attr.value() {
                            current_offset = next_offset;
                            continue;
                        }
                    }
                    let kind = BaseTypeKind::Primitive {
                        name: "void".to_string(),
                        size: 0,
                        alignment: 1,
                    };
                    return Ok((kind, pointer_depth, is_const, is_volatile));
                }

                gimli::DW_TAG_base_type => {
                    let kind = self.extract_primitive_type(entry)?;
                    return Ok((kind, pointer_depth, is_const, is_volatile));
                }

                gimli::DW_TAG_typedef => {
                    let kind = self.extract_typedef_type(entry)?;
                    return Ok((kind, pointer_depth, is_const, is_volatile));
                }

                _ => {
                    // Placeholder for now
                    let kind = BaseTypeKind::Primitive {
                        name: format!("<unknown:{}>", entry.tag()),
                        size: 0,
                        alignment: 1,
                    };
                    return Ok((kind, pointer_depth, is_const, is_volatile));
                }
            }
        }
    }

    fn extract_primitive_type(&self, entry: &DebuggingInformationEntry<R>) -> Result<BaseTypeKind> {
        let name = self.get_name(entry)?;
        let size = entry
            .attr(gimli::DW_AT_byte_size)?
            .and_then(|attr| attr.udata_value())
            .unwrap_or(0) as usize;

        Ok(BaseTypeKind::Primitive {
            name,
            size,
            alignment: size, // Assume alignment = size for primitives
        })
    }

    fn extract_typedef_type(
        &mut self,
        entry: &DebuggingInformationEntry<R>,
    ) -> Result<BaseTypeKind> {
        let name = self.get_name(entry)?;

        let aliased_type_id = if let Some(attr) = entry.attr(gimli::DW_AT_type)? {
            if let AttributeValue::UnitRef(offset) = attr.value() {
                self.build_type_registry_entry(offset)?
            } else {
                self.get_or_create_void_type()?
            }
        } else {
            self.get_or_create_void_type()?
        };

        Ok(BaseTypeKind::Typedef {
            name,
            aliased_type_id,
        })
    }

    fn get_or_create_void_type(&mut self) -> Result<TypeId> {
        let void_types = self.type_registry.get_by_name("void");
        if let Some(void_type) = void_types.first() {
            return Ok(void_type.id);
        }

        let void_type = Type {
            id: 0,
            kind: BaseTypeKind::Primitive {
                name: "void".to_string(),
                size: 0,
                alignment: 1,
            },
            pointer_depth: 0,
            is_const: false,
            is_volatile: false,
            dwarf_offset: None,
        };

        Ok(self.type_registry.register_type(void_type))
    }

    pub fn into_registry(self) -> TypeRegistry {
        self.type_registry
    }

    #[allow(dead_code)]
    pub fn get_registry(&self) -> &TypeRegistry {
        &self.type_registry
    }
}
