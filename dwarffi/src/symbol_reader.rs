use anyhow::{Context, Result};
use object::{Object, ObjectSymbol};
use std::collections::HashSet;

/// Extracts exported function symbols from a dynamic library
pub struct SymbolReader<'data> {
    object_file: object::File<'data>,
}

impl<'data> SymbolReader<'data> {
    pub fn new(data: &'data [u8]) -> Result<Self> {
        log::debug!("create symbol reader for {} bytes", data.len());
        let object_file = object::File::parse(data).context("failed to parse object file")?;

        log::debug!("object file format: {:?}", object_file.format());
        Ok(Self { object_file })
    }

    /// get unique symbol names
    pub fn get_exported_symbols(&self) -> Result<HashSet<String>> {
        let mut symbols = HashSet::new();

        log::debug!("check dynamic symbols");
        let mut dynamic_count = 0;

        // try dynamic symbols first
        for symbol in self.object_file.dynamic_symbols() {
            dynamic_count += 1;
            if symbol.is_definition() && symbol.kind() == object::SymbolKind::Text {
                if let Ok(name) = symbol.name() {
                    log::trace!("symbol: {}", name);
                    symbols.insert(name.to_string());
                }
            }
        }

        log::debug!(
            "process {} dynamic symbols, found {} function symbols",
            dynamic_count,
            symbols.len()
        );

        // regular symbol table
        if symbols.is_empty() {
            log::debug!("no dynamic symbols found, check regular symbol table");
            let mut regular_count = 0;

            for symbol in self.object_file.symbols() {
                regular_count += 1;
                if symbol.is_definition() && symbol.kind() == object::SymbolKind::Text {
                    // if global, then its exported.
                    if symbol.is_global() {
                        if let Ok(name) = symbol.name() {
                            log::trace!("regular symbol: {}", name);
                            symbols.insert(name.to_string());
                        }
                    }
                }
            }

            log::debug!(
                "processed {} regular symbols, found {} function symbols",
                regular_count,
                symbols.len()
            );
        }

        log::info!("total exported function symbols found: {}", symbols.len());
        Ok(symbols)
    }
}
