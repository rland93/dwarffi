//! Load files and read them with DWARF
use anyhow::{Context, Result};
use gimli::{EndianRcSlice, RunTimeEndian};
use object::{Object, ObjectSection};
pub type DwarfReader = EndianRcSlice<RunTimeEndian>;

pub fn load_file(path: &std::path::Path) -> Result<Vec<u8>> {
    log::debug!("load file: {}", path.display());

    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open file: {}", path.display()))?;

    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    let data = mmap.to_vec();

    log::debug!("file load success, size: {} bytes", data.len());
    Ok(data)
}

pub fn object_section_loader(
    data: &[u8],
) -> Result<impl Fn(gimli::SectionId) -> Result<DwarfReader>> {
    let object_file = object::File::parse(data)?;
    log::debug!("parse object file success");
    let endianness = if object_file.is_little_endian() {
        RunTimeEndian::Little
    } else {
        RunTimeEndian::Big
    };

    let load_section = move |id: gimli::SectionId| -> Result<DwarfReader> {
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
        let rc_data = std::rc::Rc::from(owned_data);
        let reader = EndianRcSlice::new(rc_data, endianness);

        Ok(reader)
    };

    Ok(load_section)
}
