use crate::symbol_reader::SymbolReader;
use crate::type_resolver::TypeResolver;
use crate::types::{FunctionSignature, Parameter};
use anyhow::{Context, Result};
use gimli::{AttributeValue, Dwarf, EndianRcSlice, Reader, RunTimeEndian};
use log;
use object::{Object, ObjectSection};
use std::collections::HashSet;
use std::rc::Rc;

type DwarfReader = EndianRcSlice<RunTimeEndian>;

pub struct DwarfAnalyzer {
    data: Vec<u8>,
}

impl DwarfAnalyzer {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// load the dynamic library from file path
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        log::debug!("load file: {}", path.display());

        let file = std::fs::File::open(path)
            .with_context(|| format!("failed to open file: {}", path.display()))?;

        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        let data = mmap.to_vec();

        log::debug!("file load success, size: {} bytes", data.len());
        Ok(Self::new(data))
    }

    /// get all exported function symbols (STT_FUNC)
    pub fn get_exported_symbols(&self) -> Result<HashSet<String>> {
        log::debug!("read exported symbols from binary");
        let symbol_reader = SymbolReader::new(&self.data)?;
        let symbols = symbol_reader.get_exported_symbols()?;
        Ok(symbols)
    }

    /// extract all function signatures from DWARF debug info
    pub fn extract_signatures(&self, exported_only: bool) -> Result<Vec<FunctionSignature>> {
        log::debug!("start extract symbols, exported_only: {}", exported_only);

        let object_file = object::File::parse(&*self.data)?;
        log::debug!("parse object file success");

        let endian = if object_file.is_little_endian() {
            RunTimeEndian::Little
        } else {
            RunTimeEndian::Big
        };
        log::debug!(
            "endianness: {:?}",
            if endian == RunTimeEndian::Little {
                "little endian"
            } else {
                "big endian"
            }
        );

        // loader function for extracting DWARF sections from the object file
        let load_section = |id: gimli::SectionId| -> Result<DwarfReader> {
            let section_name = id.name();
            let section_data = match object_file.section_by_name(section_name) {
                Some(section) => {
                    match section.uncompressed_data() {
                        Ok(data) => data,
                        // could not decompress
                        Err(_) => {
                            log::warn!("decompress section fail, section: {}", section_name);
                            std::borrow::Cow::Borrowed(&[][..])
                        }
                    }
                }
                // name does not exist
                None => std::borrow::Cow::Borrowed(&[][..]),
            };

            // copies out of section data
            let owned_data = section_data.into_owned();
            let rc_data = Rc::from(owned_data);
            let reader = EndianRcSlice::new(rc_data, endian);

            Ok(reader)
        };

        let dwarf = Dwarf::load(load_section)?;
        log::debug!("DWARF data load success");

        // export only?
        let exported_symbols = if exported_only {
            Some(self.get_exported_symbols()?)
        } else {
            None
        };

        // now we'll build up signatures
        let mut signatures = Vec::new();

        let mut unit_iter = dwarf.units();
        let mut unit_count = 0;

        while let Some(header) = unit_iter.next()? {
            unit_count += 1;
            log::debug!("processing compilation unit {}", unit_count);

            let unit = dwarf.unit(header)?;

            // get the signatures
            let unit_sigs = self.extract_functions_from_unit(&dwarf, &unit, &exported_symbols)?;

            log::debug!("found {} functions in unit {}", unit_sigs.len(), unit_count);
            signatures.extend(unit_sigs);
        }

        log::info!(
            "process {} compilation units, found {} total functions",
            unit_count,
            signatures.len()
        );
        Ok(signatures)
    }

    fn extract_functions_from_unit(
        &self,
        dwarf: &Dwarf<DwarfReader>,
        unit: &gimli::Unit<DwarfReader>,
        exported_symbols: &Option<HashSet<String>>,
    ) -> Result<Vec<FunctionSignature>> {
        let mut signatures = Vec::new();
        // type resolver is a stateful object that is carried along to extract
        // types. it builds up an internal cache of types it sees, which is then
        // resolved at the end. This is because we may encounter types in e.g.
        // function parameters or return types that aren't defined until later
        // in the DWARF info.
        let mut type_resolver = TypeResolver::new(dwarf, unit);
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

            // extract the return type. no return type means void
            let return_type = if let Some(type_attr) = entry.attr(gimli::DW_AT_type)? {
                if let AttributeValue::UnitRef(offset) = type_attr.value() {
                    type_resolver.resolve_type(offset)?
                } else {
                    "void".to_string()
                }
            } else {
                "void".to_string()
            };

            log::debug!(
                "{:>12} {:#010x}: {} {}()",
                "function",
                entry.offset().0,
                return_type,
                name
            );

            // extract the parameters
            // parameters could have types,
            let (parameters, is_variadic) =
                self.extract_parameters(dwarf, unit, entry, &mut type_resolver)?;

            signatures.push(FunctionSignature {
                name: name.clone(),
                return_type,
                parameters,
                is_variadic,
                is_exported,
            });
        }

        log::debug!(
            "{:>12} {} function entries, {} signatures, {} types",
            "DONE",
            function_count,
            signatures.len(),
            type_resolver.cache_len()
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
        dwarf: &Dwarf<DwarfReader>,
        unit: &gimli::Unit<DwarfReader>,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
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
        dwarf: &Dwarf<DwarfReader>,
        unit: &gimli::Unit<DwarfReader>,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
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
        dwarf: &Dwarf<DwarfReader>,
        unit: &gimli::Unit<DwarfReader>,
        attr: Option<gimli::Attribute<DwarfReader>>,
    ) -> Option<String> {
        let attr = attr?;

        let name_entry_offset = match attr.value() {
            AttributeValue::UnitRef(offset) => offset,
            _ => return None,
        };

        let mut entries = unit.entries_at_offset(name_entry_offset).ok()?;
        let Some((_, referenced)) = entries.next_dfs().ok()? else {
            return None;
        };

        Self::read_entry_name(dwarf, unit, &referenced)
    }

    /// check if an attribute is a flag and is true
    fn attr_flag_is_true(attr: Option<gimli::Attribute<DwarfReader>>) -> bool {
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
        dwarf: &Dwarf<DwarfReader>,
        unit: &gimli::Unit<DwarfReader>,
        attr: &gimli::Attribute<DwarfReader>,
    ) -> Option<String> {
        // reader of bytes
        let attribute_string = match dwarf.attr_string(unit, attr.value()) {
            Ok(value) => value,
            Err(err) => {
                log::trace!("failed to load attribute string: {}", err);
                return None;
            }
        };

        // to byte slice
        let bytes = match attribute_string.to_slice() {
            Ok(slice) => slice,
            Err(err) => {
                log::trace!("failed to read attribute bytes: {}", err);
                return None;
            }
        };

        // parse as string
        match String::from_utf8(bytes.to_vec()) {
            Ok(text) => Some(text),
            Err(err) => {
                log::trace!("attribute string is not valid UTF-8: {}", err);
                None
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
        dwarf: &Dwarf<DwarfReader>,
        unit: &gimli::Unit<DwarfReader>,
        func_entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        type_resolver: &mut TypeResolver<DwarfReader>,
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

                    // Get parameter type
                    let param_type =
                        if let Ok(Some(type_attr)) = child_entry.attr(gimli::DW_AT_type) {
                            if let AttributeValue::UnitRef(offset) = type_attr.value() {
                                type_resolver.resolve_type(offset)?
                            } else {
                                "void".to_string()
                            }
                        } else {
                            "void".to_string()
                        };

                    log::debug!(
                        "{:>12} {:#010x}: {} {}",
                        "parameter",
                        child_entry.offset().0,
                        param_type,
                        param_name,
                    );

                    parameters.push(Parameter {
                        name: param_name,
                        type_name: param_type,
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
