use std::io::{Read, Result as IoResult};
use std::rc::{Rc, Weak};
use std::cell::RefCell;

pub struct SharedStream {
    inner: Box<dyn Read>,
    cache: Vec<u8>,
    eof_reached: bool,
    readers: Vec<Weak<RefCell<usize>>>,
}

pub struct VirtualReader {
    shared: Rc<RefCell<SharedStream>>,
    pos: Rc<RefCell<usize>>,
}

impl SharedStream {
    pub fn new(reader: impl Read + 'static) -> Self {
        Self {
            inner: Box::new(reader),
            cache: Vec::with_capacity(1024),
            eof_reached: false,
            readers: Vec::new(),
        }
    }

    pub fn fork(shared: Rc<RefCell<Self>>) -> VirtualReader {
        let pos = Rc::new(RefCell::new(0));
        shared.borrow_mut().readers.push(Rc::downgrade(&pos));
        VirtualReader { shared, pos }
    }

    pub fn auto_prune(&mut self) {
        let mut min_pos = None;
        self.readers.retain(|weak_ptr| {
            if let Some(pos_rc) = weak_ptr.upgrade() {
                let p = *pos_rc.borrow();
                min_pos = Some(min_pos.map_or(p, |m| std::cmp::min(m, p)));
                true
            } else {
                false
            }
        });

        if let Some(n) = min_pos {
            if n > 0 {
                self.cache.drain(0..n);
                for weak_ptr in &self.readers {
                    if let Some(pos_rc) = weak_ptr.upgrade() {
                        *pos_rc.borrow_mut() -= n;
                    }
                }
            }
        }
    }
}

impl Read for VirtualReader {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let mut stream = self.shared.borrow_mut();
        let mut current_pos = self.pos.borrow_mut();

        if *current_pos >= stream.cache.len() && !stream.eof_reached {
            let mut temp = [0u8; 1024];
            let n = stream.inner.read(&mut temp)?;
            if n == 0 {
                stream.eof_reached = true;
            } else {
                stream.cache.extend_from_slice(&temp[..n]);
            }
        }

        let available = stream.cache.len() - *current_pos;
        if available == 0 { return Ok(0); }

        let n = std::cmp::min(available, buf.len());
        buf[..n].copy_from_slice(&stream.cache[*current_pos..*current_pos + n]);
        *current_pos += n;

        drop(current_pos);
        stream.auto_prune();

        Ok(n)
    }
}