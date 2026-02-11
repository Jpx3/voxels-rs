use crate::common::{Block, Boundary};
use crate::stream::mojang_reader::MojangSchematicInputStream;
use crate::stream::shared_stream::{SharedStream, VirtualReader};
use crate::stream::sponge_reader::SpongeSchematicInputStream;
use crate::stream::stream::SchematicInputStream;
use crate::stream::vxl_reader::VXLSchematicInputStream;
use std::cell::RefCell;
use std::cmp::min;
use std::io::Read;
use std::rc::Rc;
use crate::stream::mcedit_reader::MCEditSchematicInputStream;

pub struct AnySchematicInputStream {
    options: Vec<(Box<dyn SchematicInputStream>, Vec<Block>)>,
    closed: bool,
}

impl SchematicInputStream for AnySchematicInputStream {
    fn read(
        &mut self,
        buffer: &mut Vec<Block>,
        offset: usize,
        length: usize,
    ) -> Result<Option<usize>, String> {
        if self.options.len() == 1 {
            return self.read_from_sole_provider(buffer, offset, length);
        }
        let mut to_remove = Vec::new();
        for (i, (stream, peek_buf)) in self.options.iter_mut().enumerate() {
            let mut temp_buf = Vec::new();
            match stream.read(&mut temp_buf, 0, length) {
                Ok(Some(_)) => {
                    peek_buf.extend(temp_buf);
                }
                Ok(None) => {
                    to_remove.push(i);
                    self.closed = true;
                }
                Err(_e) => {
                    to_remove.push(i);
                }
            }
        }

        for &i in to_remove.iter().rev() {
            self.options.remove(i);
        }

        match self.options.len() {
            0 => {
                if self.closed {
                    Ok(None)
                } else {
                    Err("No matching format found".to_string())
                }
            },
            1 => self.read_from_sole_provider(buffer, offset, length),
            _ => Ok(Some(0)),
        }
    }

    fn boundary(&mut self) -> Result<Option<Boundary>, String> {
        if self.options.len() == 1 {
            return self.options[0].0.boundary();
        }
        self.options.retain_mut(|(opt, _)| opt.boundary().is_ok());
        if self.options.len() == 1 {
            self.options[0].0.boundary()
        } else {
            Ok(None)
        }
    }
}

impl AnySchematicInputStream {
    pub fn new_from_known<R: Read + 'static>(
        source: R,
    ) -> Self {
        Self::new(
            source,
            vec![
                Box::new(|r| Box::new(SpongeSchematicInputStream::new(r))),
                Box::new(|r| Box::new(MojangSchematicInputStream::new(r))),
                Box::new(|r| Box::new(MCEditSchematicInputStream::new(r))),
                Box::new(|r| Box::new(VXLSchematicInputStream::new(r))),
            ]
        )
    }

    pub fn new<R: Read + 'static>(
        source: R,
        constructors: Vec<Box<dyn FnOnce(VirtualReader) -> Box<dyn SchematicInputStream>>>
    ) -> Self {
        let shared = Rc::new(RefCell::new(SharedStream::new(source)));
        let options = constructors
            .into_iter()
            .map(|constructor| {
                let fork = SharedStream::fork(Rc::clone(&shared));
                constructor(fork)
            })
            .map(|stream| (stream, Vec::new()))
            .collect();
        Self {
            options,
            closed: false,
        }
    }

    fn read_from_sole_provider(
        &mut self,
        buffer: &mut Vec<Block>,
        _offset: usize,
        length: usize,
    ) -> Result<Option<usize>, String> {
        let (stream, peek_buf) = &mut self.options[0];
        if !peek_buf.is_empty() {
            let count = min(peek_buf.len(), length);
            buffer.extend(peek_buf.drain(..count));
            return Ok(Some(count));
        }
        stream.read(buffer, _offset, length)
    }
}

#[cfg(test)]
mod tests {
    use crate::stream::any_reader::AnySchematicInputStream;
    use crate::stream::stream::SchematicInputStream;
    use flate2::read::GzDecoder;

    #[test]
    fn test_any_reader() {
        const TREE_SCHEMATIC: &[u8] = include_bytes!("test_schematics/tree.sponge");
        let reader = std::io::Cursor::new(TREE_SCHEMATIC);
        let reader = GzDecoder::new(reader);
        let mut any_stream = AnySchematicInputStream::new_from_known(reader);
        let blocks = any_stream.read_to_end_into_vec().expect("Failed to read schematic");
        assert!(!blocks.is_empty(), "Expected to read some blocks from the schematic");
    }
}