use crate::reader;
use crate::symbol_reader::SymbolReader;
use crate::type_registry::TypeRegistry;
use crate::type_resolver::TypeResolver;
use crate::types::{FunctionSignature, Parameter};
use anyhow::Result;
use gimli::{AttributeValue, Dwarf, Reader};
use std::collections::HashSet;

pub struct DwarfAnalyzer {
    data: Vec<u8>,
}

pub struct AnalysisResult {
    pub signatures: Vec<FunctionSignature>,
    pub type_registry: TypeRegistry,
}

impl DwarfAnalyzer {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// load the dynamic library from file path
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        let data = reader::load_file(path)?;
        Ok(Self::new(data))
    }

    /// get all exported function symbols (STT_FUNC)
    pub fn get_exported_symbols(&self) -> Result<HashSet<String>> {
        log::debug!("read exported symbols from binary");
        let symbol_reader = SymbolReader::new(&self.data)?;
        let symbols = symbol_reader.get_exported_symbols()?;
        Ok(symbols)
    }

    /// extract function signatures and type registry from DWARF debug info
    pub fn extract_analysis(&self, exported_only: bool) -> Result<AnalysisResult> {
        let section_loader = reader::object_section_loader(&self.data)?;
        let dwarf = Dwarf::load(section_loader)?;
        log::debug!("DWARF data load success");

        // export only?
        let exported_symbols = if exported_only {
            Some(self.get_exported_symbols()?)
        } else {
            None
        };

        let mut all_signatures = Vec::new();
        let mut combined_registry = TypeRegistry::new();
        let mut unit_iter = dwarf.units();
        let mut unit_count = 0;

        while let Some(header) = unit_iter.next()? {
            unit_count += 1;
            log::debug!("processing compilation unit {}", unit_count);

            let unit = dwarf.unit(header)?;
            let mut type_resolver = TypeResolver::new(&dwarf, &unit);

            // Extract function signatures with TypeId-based parameters
            let unit_sigs = self.extract_functions_from_unit(
                &dwarf,
                &unit,
                &exported_symbols,
                &mut type_resolver,
            )?;

            log::debug!("found {} functions in unit {}", unit_sigs.len(), unit_count);
            all_signatures.extend(unit_sigs);

            // Merge type registry from this unit
            let unit_registry = type_resolver.into_registry();
            combined_registry.merge(unit_registry);
        }

        log::info!(
            "processed {} compilation units, found {} functions, extracted {} types",
            unit_count,
            all_signatures.len(),
            combined_registry.len()
        );

        Ok(AnalysisResult {
            signatures: all_signatures,
            type_registry: combined_registry,
        })
    }

    fn extract_functions_from_unit(
        &self,
        dwarf: &Dwarf<reader::DwarfReader>,
        unit: &gimli::Unit<reader::DwarfReader>,
        exported_symbols: &Option<HashSet<String>>,
        type_resolver: &mut TypeResolver<reader::DwarfReader>,
    ) -> Result<Vec<FunctionSignature>> {
        let mut signatures = Vec::new();
        let mut function_count = 0;
        let mut entries = unit.entries();

        // DWARF entries are tree-like. functions are grouped with their return
        // types, parameters, etc. dfs will pull out children i.e. parameters,
        // return types together.
        while let Some((_, entry)) = entries.next_dfs()? {
            // function definitions marked with DW_TAG_subprogram
            if entry.tag() != gimli::DW_TAG_subprogram {
                continue;
            }

            // skip function declarations (keep only definitions)
            if Self::attr_flag_is_true(entry.attr(gimli::DW_AT_declaration).ok().flatten()) {
                log::trace!("skip function declaration at {:#010x}", entry.offset().0);
                continue;
            }

            function_count += 1;

            // skip no-name functions
            let name = match self.get_function_name(dwarf, unit, entry) {
                Some(n) => {
                    log::trace!("found function: {}", n);
                    n
                }
                None => {
                    log::trace!("skip unnamed function");
                    continue;
                }
            };

            // check against exported symbols
            let is_exported = exported_symbols
                .as_ref()
                .map(|symbols| {
                    // macOS prepends an underscore to symbol name
                    symbols.contains(&name) || symbols.contains(&format!("_{}", name))
                })
                .unwrap_or(true);

            // skip if not exported
            if exported_symbols.is_some() && !is_exported {
                log::trace!("skip non-exported function: {}", name);
                continue;
            }

            // extract the return type TypeId
            let return_type_id = if let Some(type_attr) = entry.attr(gimli::DW_AT_type)? {
                if let AttributeValue::UnitRef(offset) = type_attr.value() {
                    type_resolver.build_type_registry_entry(offset)?
                } else {
                    type_resolver.get_void_type_id()?
                }
            } else {
                type_resolver.get_void_type_id()?
            };

            log::debug!("{:>12} {:#010x}: {}()", "function", entry.offset().0, name);

            // extract the parameters
            let (parameters, is_variadic) =
                self.extract_parameters(dwarf, unit, entry, type_resolver)?;

            signatures.push(FunctionSignature {
                name: name.clone(),
                return_type_id,
                parameters,
                is_variadic,
                is_exported,
            });
        }

        log::debug!(
            "{:>12} {} function entries, {} signatures extracted",
            "DONE",
            function_count,
            signatures.len()
        );
        Ok(signatures)
    }

    // attempt to extract the function name from the unit. returns None if no
    // name can be found. note in some instances if library is stripped or
    // partially stripped this cannot detect those cases, it is the
    // responsibility of the programmer to compile the library with full,
    // unstripped debug information!
    fn get_function_name(
        &self,
        dwarf: &Dwarf<reader::DwarfReader>,
        unit: &gimli::Unit<reader::DwarfReader>,
        entry: &gimli::DebuggingInformationEntry<reader::DwarfReader>,
    ) -> Option<String> {
        // skip artificial
        if Self::attr_flag_is_true(entry.attr(gimli::DW_AT_artificial).ok().flatten()) {
            log::trace!("skip artificial subprogram @{:#010x}", entry.offset().0);
            return None;
        }

        // direct name
        if let Some(name) = Self::read_entry_name(dwarf, unit, entry) {
            return Some(name);
        }

        // try to resolve name references
        if let Some(name) = Self::resolve_name_reference(
            dwarf,
            unit,
            entry.attr(gimli::DW_AT_specification).ok().flatten(),
        ) {
            log::trace!(
                "use DW_AT_specification name for subprogram @{:#010x}: {}",
                entry.offset().0,
                name
            );
            return Some(name);
        }

        // try abstract origin
        if let Some(name) = Self::resolve_name_reference(
            dwarf,
            unit,
            entry.attr(gimli::DW_AT_abstract_origin).ok().flatten(),
        ) {
            log::trace!(
                "use DW_AT_abstract_origin name for subprogram @{:#010x}: {}",
                entry.offset().0,
                name
            );
            return Some(name);
        }

        log::trace!(
            "subprogram at offset {:#010x} has no discoverable name",
            entry.offset().0
        );

        None
    }

    /// helper to grab the name from an entry
    fn read_entry_name(
        dwarf: &Dwarf<reader::DwarfReader>,
        unit: &gimli::Unit<reader::DwarfReader>,
        entry: &gimli::DebuggingInformationEntry<reader::DwarfReader>,
    ) -> Option<String> {
        // linkage names
        if let Ok(Some(attr)) = entry.attr(gimli::DW_AT_linkage_name) {
            if let Some(name) = Self::read_attr_string(dwarf, unit, &attr) {
                return Some(name);
            }
        }

        // regular names
        if let Ok(Some(attr)) = entry.attr(gimli::DW_AT_name) {
            if let Some(name) = Self::read_attr_string(dwarf, unit, &attr) {
                return Some(name);
            }
        }

        None
    }

    /// sometimes compilers emit smaller DIEs that reference other DIEs, which
    /// contain the name. this will resolve a name from such entries by
    /// following the reference.
    fn resolve_name_reference(
        dwarf: &Dwarf<reader::DwarfReader>,
        unit: &gimli::Unit<reader::DwarfReader>,
        attr: Option<gimli::Attribute<reader::DwarfReader>>,
    ) -> Option<String> {
        let attr = attr?;

        let name_entry_offset = match attr.value() {
            AttributeValue::UnitRef(offset) => offset,
            _ => return None,
        };

        let mut entries = unit.entries_at_offset(name_entry_offset).ok()?;
        let (_, referenced) = (entries.next_dfs().ok()?)?;

        Self::read_entry_name(dwarf, unit, referenced)
    }

    /// check if an attribute is a flag and is true
    fn attr_flag_is_true(attr: Option<gimli::Attribute<reader::DwarfReader>>) -> bool {
        let Some(attr) = attr else {
            return false;
        };

        match attr.value() {
            AttributeValue::Flag(true) => true,
            AttributeValue::Data1(value) => value != 0,
            AttributeValue::Data2(value) => value != 0,
            AttributeValue::Data4(value) => value != 0,
            AttributeValue::Data8(value) => value != 0,
            AttributeValue::Sdata(value) => value != 0,
            AttributeValue::Udata(value) => value != 0,
            _ => false,
        }
    }

    /// read a string attribute from an entry
    fn read_attr_string(
        dwarf: &Dwarf<reader::DwarfReader>,
        unit: &gimli::Unit<reader::DwarfReader>,
        attr: &gimli::Attribute<reader::DwarfReader>,
    ) -> Option<String> {
        match attr.value() {
            // DWARF 5 may inline strings
            AttributeValue::String(s) => match s.to_string_lossy() {
                Ok(cow) => Some(cow.to_string()),
                Err(e) => {
                    log::warn!("failed to decode inline string: {:?}", e);
                    None
                }
            },
            // older versions do a string reference to .debug_str section
            _ => {
                let r = dwarf.attr_string(unit, attr.value()).ok()?;
                match r.to_string_lossy() {
                    Ok(cow) => Some(cow.to_string()),
                    Err(e) => {
                        log::warn!("failed to decode string reference: {:?}", e);
                        None
                    }
                }
            }
        }
    }

    /// parameters are always direct children of the function entry. They could
    /// be DW_TAG_formal_parameter or DW_TAG_unspecified_parameters denoting
    /// standard parameters vs variadic (i.e. ...). if a function has a
    /// parameter that is unspecified, that means that it is variadic.
    ///
    /// therefore the returned tuple contains the list of parameters and whether
    /// the function from whom the parameters are extracted is variadic.
    ///
    /// We also carry the stateful type resolver with us and update it, since we
    /// may encounter types that are not yet analyzed in the parameters.
    fn extract_parameters(
        &self,
        dwarf: &Dwarf<reader::DwarfReader>,
        unit: &gimli::Unit<reader::DwarfReader>,
        func_entry: &gimli::DebuggingInformationEntry<reader::DwarfReader>,
        type_resolver: &mut TypeResolver<reader::DwarfReader>,
    ) -> Result<(Vec<Parameter>, bool)> {
        let mut parameters = Vec::new();
        let mut is_variadic = false;

        let offset = func_entry.offset();
        let mut tree = unit.entries_tree(Some(offset))?;
        let func_node = tree.root()?;

        // parameters are direct children
        let mut children = func_node.children();
        while let Some(child) = children.next()? {
            let child_entry = child.entry();

            match child_entry.tag() {
                // formal are named params with types
                gimli::DW_TAG_formal_parameter => {
                    let param_name = child_entry
                        .attr(gimli::DW_AT_name)
                        .ok()
                        .flatten()
                        .and_then(|attr| Self::read_attr_string(dwarf, unit, &attr))
                        .unwrap_or_default();

                    // Get parameter type TypeId
                    let param_type_id =
                        if let Ok(Some(type_attr)) = child_entry.attr(gimli::DW_AT_type) {
                            if let AttributeValue::UnitRef(offset) = type_attr.value() {
                                type_resolver.build_type_registry_entry(offset)?
                            } else {
                                type_resolver.get_void_type_id()?
                            }
                        } else {
                            type_resolver.get_void_type_id()?
                        };

                    log::debug!(
                        "{:>12} {:#010x}: {}",
                        "parameter",
                        child_entry.offset().0,
                        param_name,
                    );

                    parameters.push(Parameter {
                        name: param_name,
                        type_id: param_type_id,
                    });
                }

                // unspecified -> variadic
                gimli::DW_TAG_unspecified_parameters => {
                    is_variadic = true;
                }

                // we ONLY care about parameters.
                _ => {
                    // it's normal to hit non-parameter tags. these can be
                    // variables, lexical blocks, etc. depending on compiler
                    // optimization.
                    log::trace!(
                        "non parameter tag {} @{:#010x}",
                        child_entry.tag(),
                        child_entry.offset().0,
                    );
                }
            }
        }

        Ok((parameters, is_variadic))
    }
}
