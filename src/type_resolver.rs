use anyhow::{Result, anyhow};
use gimli::{AttributeValue, DebuggingInformationEntry, Dwarf, ReaderOffset, Unit, UnitOffset};
use std::collections::HashMap;

/// resolve DWARF type information into human-readable C type strings
pub struct TypeResolver<'dwarf, R: gimli::Reader> {
    dwarf: &'dwarf Dwarf<R>,
    unit: &'dwarf Unit<R>,
    /// cache of resolved types keyed by offset
    type_cache: HashMap<UnitOffset<R::Offset>, String>,
}

impl<'dwarf, R: gimli::Reader> TypeResolver<'dwarf, R> {
    /// create new with empty cache with the given DWARF and unit
    pub fn new(dwarf: &'dwarf Dwarf<R>, unit: &'dwarf Unit<R>) -> Self {
        Self {
            dwarf,
            unit,
            type_cache: HashMap::new(),
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
}
