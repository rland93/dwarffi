use crate::type_registry::{BaseTypeKind, Type, TypeId, TypeRegistry};
use anyhow::{Result, anyhow};
use gimli::{AttributeValue, DebuggingInformationEntry, Dwarf, ReaderOffset, Unit, UnitOffset};
use log;

/// resolve DWARF type information into structured type registry
pub struct TypeResolver<'dwarf, R: gimli::Reader> {
    dwarf: &'dwarf Dwarf<R>,
    unit: &'dwarf Unit<R>,
    type_registry: TypeRegistry,
}

impl<'dwarf, R: gimli::Reader> TypeResolver<'dwarf, R> {
    /// create new with empty registry with the given DWARF and unit
    pub fn new(dwarf: &'dwarf Dwarf<R>, unit: &'dwarf Unit<R>) -> Self {
        Self {
            dwarf,
            unit,
            type_registry: TypeRegistry::new(),
        }
    }

    pub fn build_type_registry_entry(&mut self, offset: UnitOffset<R::Offset>) -> Result<TypeId> {
        let dwarf_offset = offset.0.into_u64();

        if let Some(type_) = self.type_registry.get_by_dwarf_offset(dwarf_offset) {
            log::trace!("type already registered at offset {:#010x}", dwarf_offset);
            return Ok(type_.id);
        }

        let mut entries = self.unit.entries_at_offset(offset)?;
        let (_, entry) = entries
            .next_dfs()?
            .ok_or_else(|| anyhow!("no entry at offset"))?;

        log::trace!("extracting type at offset {:#010x}", dwarf_offset);

        let (kind, pointer_depth, is_const, is_volatile) =
            self.extract_type_metadata(entry, offset)?;

        let extracted_type = Type {
            id: TypeId(0),
            kind,
            pointer_depth,
            is_const,
            is_volatile,
            dwarf_offset: Some(dwarf_offset),
        };

        let id = self.type_registry.register_type(extracted_type);
        Ok(id)
    }

    pub fn get_void_type_id(&mut self) -> Result<TypeId> {
        self.get_or_create_void_type()
    }

    fn get_name(&self, entry: &DebuggingInformationEntry<R>) -> Result<String> {
        if let Some(attr) = entry.attr(gimli::DW_AT_name)? {
            let name_reader = self.dwarf.attr_string(self.unit, attr.value())?;
            let bytes = name_reader.to_slice()?;
            let name_str = String::from_utf8(bytes.to_vec())?;
            return Ok(name_str);
        }

        Err(anyhow!("no name attribute"))
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
                    // follow to pointee
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
                    // follow to inner type
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

                gimli::DW_TAG_structure_type => {
                    let kind = self.extract_struct_type(entry, current_offset)?;
                    return Ok((kind, pointer_depth, is_const, is_volatile));
                }

                gimli::DW_TAG_union_type => {
                    let kind = self.extract_union_type(entry, current_offset)?;
                    return Ok((kind, pointer_depth, is_const, is_volatile));
                }

                gimli::DW_TAG_enumeration_type => {
                    let kind = self.extract_enum_type(entry, current_offset)?;
                    return Ok((kind, pointer_depth, is_const, is_volatile));
                }

                gimli::DW_TAG_array_type => {
                    let kind = self.extract_array_type(entry, current_offset)?;
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

        log::trace!("{:>12} {} ({} bytes)", "primitive", name, size);

        Ok(BaseTypeKind::Primitive {
            name,
            size,
            alignment: size, // alignment = size for primitives
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

        log::debug!("{:>12} {}", "typedef", name);

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
        };

        Ok(self.type_registry.register_type(void_type))
    }

    fn extract_struct_type(
        &mut self,
        entry: &DebuggingInformationEntry<R>,
        offset: UnitOffset<R::Offset>,
    ) -> Result<BaseTypeKind> {
        let name = self
            .get_name(entry)
            .unwrap_or_else(|_| "<anonymous>".to_string());

        let size = entry
            .attr(gimli::DW_AT_byte_size)?
            .and_then(|attr| attr.udata_value())
            .unwrap_or(0) as usize;

        //check if opaque (declaration only, no byte_size)
        let is_opaque = size == 0 && entry.attr(gimli::DW_AT_declaration)?.is_some();

        if is_opaque {
            log::debug!("{:>12} {:#010x}: {} (opaque)", "struct", offset.0.into_u64(), name);
        } else {
            log::debug!("{:>12} {:#010x}: {} ({} bytes)", "struct", offset.0.into_u64(), name, size);
        }

        // extract fields (children of struct entry)
        let fields = self.extract_struct_fields(offset)?;

        let alignment = fields.iter().map(|f| f.size).max().unwrap_or(1);

        Ok(BaseTypeKind::Struct {
            name,
            fields,
            size,
            alignment,
            is_opaque,
        })
    }

    fn extract_struct_fields(
        &mut self,
        struct_offset: UnitOffset<R::Offset>,
    ) -> Result<Vec<crate::type_registry::StructField>> {
        let mut fields = Vec::new();
        let mut tree = self.unit.entries_tree(Some(struct_offset))?;
        let struct_node = tree.root()?;

        let mut children = struct_node.children();
        while let Some(child) = children.next()? {
            let entry = child.entry();

            if entry.tag() != gimli::DW_TAG_member {
                continue;
            }

            let name = self.get_name(entry).unwrap_or_default();

            let type_id = if let Some(attr) = entry.attr(gimli::DW_AT_type)? {
                if let AttributeValue::UnitRef(offset) = attr.value() {
                    self.build_type_registry_entry(offset)?
                } else {
                    log::trace!("skip field {} with invalid type reference", name);
                    continue;
                }
            } else {
                log::trace!("skip field {} with no type", name);
                continue;
            };

            let offset = entry
                .attr(gimli::DW_AT_data_member_location)?
                .and_then(|attr| attr.udata_value())
                .unwrap_or(0) as usize;

            // Get size from the field's type
            let field_type = self.type_registry.get_type(type_id);
            let size = if let Some(ft) = field_type {
                match &ft.kind {
                    BaseTypeKind::Primitive { size, .. } => *size,
                    BaseTypeKind::Struct { size, .. } => *size,
                    BaseTypeKind::Array { size, .. } => *size,
                    _ => 0,
                }
            } else {
                0
            };

            log::trace!("{:>12} {:#010x}: {} @ offset {}", "field", entry.offset().0.into_u64(), name, offset);

            fields.push(crate::type_registry::StructField {
                name,
                type_id,
                offset,
                size,
            });
        }

        log::debug!("extracted {} fields", fields.len());
        Ok(fields)
    }

    fn extract_union_type(
        &mut self,
        entry: &DebuggingInformationEntry<R>,
        offset: UnitOffset<R::Offset>,
    ) -> Result<BaseTypeKind> {
        let name = self
            .get_name(entry)
            .unwrap_or_else(|_| "<anonymous>".to_string());

        let size = entry
            .attr(gimli::DW_AT_byte_size)?
            .and_then(|attr| attr.udata_value())
            .unwrap_or(0) as usize;

        log::debug!("{:>12} {:#010x}: {} ({} bytes)", "union", offset.0.into_u64(), name, size);

        let variants = self.extract_union_fields(offset)?;

        let alignment = variants
            .iter()
            .filter_map(|v| {
                self.type_registry
                    .get_type(v.type_id)
                    .and_then(|t| match &t.kind {
                        BaseTypeKind::Primitive { alignment, .. } => Some(*alignment),
                        BaseTypeKind::Struct { alignment, .. } => Some(*alignment),
                        _ => None,
                    })
            })
            .max()
            .unwrap_or(1);

        Ok(BaseTypeKind::Union {
            name,
            variants,
            size,
            alignment,
        })
    }

    fn extract_union_fields(
        &mut self,
        union_offset: UnitOffset<R::Offset>,
    ) -> Result<Vec<crate::type_registry::UnionField>> {
        let mut variants = Vec::new();
        let mut tree = self.unit.entries_tree(Some(union_offset))?;
        let union_node = tree.root()?;

        let mut children = union_node.children();
        while let Some(child) = children.next()? {
            let entry = child.entry();

            if entry.tag() != gimli::DW_TAG_member {
                continue;
            }

            let name = self.get_name(entry).unwrap_or_default();

            let type_id = if let Some(attr) = entry.attr(gimli::DW_AT_type)? {
                if let AttributeValue::UnitRef(offset) = attr.value() {
                    self.build_type_registry_entry(offset)?
                } else {
                    log::trace!("skip variant {} with invalid type reference", name);
                    continue;
                }
            } else {
                log::trace!("skip variant {} with no type", name);
                continue;
            };

            log::trace!("{:>12} {}", "variant", name);
            variants.push(crate::type_registry::UnionField { name, type_id });
        }

        log::debug!("extracted {} variants", variants.len());
        Ok(variants)
    }

    fn extract_enum_type(
        &mut self,
        entry: &DebuggingInformationEntry<R>,
        offset: UnitOffset<R::Offset>,
    ) -> Result<BaseTypeKind> {
        let name = self
            .get_name(entry)
            .unwrap_or_else(|_| "<anonymous>".to_string());

        let size = entry
            .attr(gimli::DW_AT_byte_size)?
            .and_then(|attr| attr.udata_value())
            .unwrap_or(4) as usize; // Default to int size

        log::debug!("{:>12} {:#010x}: {} ({} bytes)", "enum", offset.0.into_u64(), name, size);

        // extract underlying type (DWARF DW_AT_type on enum)
        let backing_id = if let Some(attr) = entry.attr(gimli::DW_AT_type)? {
            if let AttributeValue::UnitRef(type_offset) = attr.value() {
                self.build_type_registry_entry(type_offset)?
            } else {
                self.get_or_create_int_type()?
            }
        } else {
            self.get_or_create_int_type()?
        };

        let variants = self.extract_enum_variants(offset)?;

        Ok(BaseTypeKind::Enum {
            name,
            backing_id,
            variants,
            size,
        })
    }

    fn extract_enum_variants(
        &mut self,
        enum_offset: UnitOffset<R::Offset>,
    ) -> Result<Vec<crate::type_registry::EnumVariant>> {
        let mut variants = Vec::new();
        let mut tree = self.unit.entries_tree(Some(enum_offset))?;
        let enum_node = tree.root()?;

        let mut children = enum_node.children();
        while let Some(child) = children.next()? {
            let entry = child.entry();

            if entry.tag() != gimli::DW_TAG_enumerator {
                continue;
            }

            let name = self.get_name(entry).unwrap_or_default();

            let value = entry
                .attr(gimli::DW_AT_const_value)?
                .and_then(|attr| attr.sdata_value())
                .unwrap_or(0);

            log::trace!("{:>12} {} = {}", "enumerator", name, value);
            variants.push(crate::type_registry::EnumVariant { name, value });
        }

        log::debug!("extracted {} enumerators", variants.len());
        Ok(variants)
    }

    fn extract_array_type(
        &mut self,
        entry: &DebuggingInformationEntry<R>,
        offset: UnitOffset<R::Offset>,
    ) -> Result<BaseTypeKind> {
        // get element type
        let element_type_id = if let Some(attr) = entry.attr(gimli::DW_AT_type)? {
            if let AttributeValue::UnitRef(type_offset) = attr.value() {
                self.build_type_registry_entry(type_offset)?
            } else {
                return Err(anyhow!("array missing element type"));
            }
        } else {
            return Err(anyhow!("array missing element type"));
        };

        // get array dimensions (subrange children)
        let count = self.extract_array_count(offset)?;

        // calculate size
        let element_type = self
            .type_registry
            .get_type(element_type_id)
            .ok_or_else(|| anyhow!("element type not found"))?;
        let element_size = match &element_type.kind {
            BaseTypeKind::Primitive { size, .. } => *size,
            BaseTypeKind::Struct { size, .. } => *size,
            BaseTypeKind::Array { size, .. } => *size,
            _ => 0,
        };

        let total_size = element_size * count;

        log::debug!("{:>12} {:#010x}: [{}] ({} bytes)", "array", offset.0.into_u64(), count, total_size);

        Ok(BaseTypeKind::Array {
            element_type_id,
            count,
            size: total_size,
        })
    }

    fn extract_array_count(&mut self, array_offset: UnitOffset<R::Offset>) -> Result<usize> {
        let mut tree = self.unit.entries_tree(Some(array_offset))?;
        let array_node = tree.root()?;

        let mut children = array_node.children();
        while let Some(child) = children.next()? {
            let entry = child.entry();

            if entry.tag() == gimli::DW_TAG_subrange_type {
                // DW_AT_upper_bound or DW_AT_count
                if let Some(attr) = entry.attr(gimli::DW_AT_count)? {
                    if let Some(count) = attr.udata_value() {
                        return Ok(count as usize);
                    }
                }

                if let Some(attr) = entry.attr(gimli::DW_AT_upper_bound)? {
                    if let Some(upper) = attr.udata_value() {
                        // Count = upper_bound + 1 (0-indexed)
                        return Ok((upper + 1) as usize);
                    }
                }
            }
        }

        // unknown/unbounded array
        Ok(0)
    }

    fn get_or_create_int_type(&mut self) -> Result<TypeId> {
        let int_types = self.type_registry.get_by_name("int");
        if let Some(int_type) = int_types.first() {
            return Ok(int_type.id);
        }

        // create a default int type if not found
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

        Ok(self.type_registry.register_type(int_type))
    }

    pub fn into_registry(self) -> TypeRegistry {
        self.type_registry
    }

    #[allow(dead_code)]
    pub fn get_registry(&self) -> &TypeRegistry {
        &self.type_registry
    }
}
