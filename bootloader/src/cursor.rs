#[derive(Debug)]
pub struct Cursor<'a> {
    buffer: &'a [u8],
    pos: usize,
}

#[allow(unused)]
impl<'a> Cursor<'a> {
    pub fn new<B: AsRef<[u8]> + ?Sized>(buffer: &'a B) -> Self {
        let buffer = buffer.as_ref();
        Self { buffer, pos: 0 }
    }
}

#[allow(unused)]
impl Cursor<'_> {
    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.pos
    }

    pub fn skip(&mut self, n: usize) {
        if self.remaining() < n {
            panic!("out of bounds: not enough bytes remaining");
        }
        self.pos += n;
    }

    pub fn peek_u8(&self, n: usize) -> u8 {
        if self.remaining() < n + 1 {
            panic!("out of bounds: not enough bytes remaining");
        }
        return self.buffer[self.pos + n];
    }

    pub fn read_u8(&mut self) -> u8 {
        if self.remaining() < 1 {
            panic!("out of bounds: not enough bytes remaining");
        }
        let x = self.buffer[self.pos];
        self.pos += 1;
        return x;
    }

    fn get_bytes<const N: usize>(&mut self) -> [u8; N] {
        if self.remaining() < N {
            panic!("out of bounds: not enough bytes remaining in buffer");
        }
        let x = &self.buffer[self.pos..(self.pos + N)];
        self.pos += N;
        return x.try_into().unwrap();
    }

    pub fn read(&mut self, buffer: &mut [u8]) {
        if self.remaining() < buffer.len() {
            panic!("out of bounds: not enough bytes remaining in buffer");
        }
        buffer.copy_from_slice(&self.buffer[self.pos..(self.pos + buffer.len())]);
        self.pos += buffer.len();
    }

    pub fn read_bytes(&mut self, count: usize) -> &[u8] {
        if self.remaining() < count {
            panic!("out of bounds: not enough bytes remaining in buffer");
        }
        let res = &self.buffer[self.pos..(self.pos + count)];
        self.pos += count;
        return res;
    }
}

// #![feature(macro_metavar_expr_concat)]

macro_rules! cursor_impl_read {
    { $($name_ne:ident, $name_le:ident, $name_be:ident, $t:ty);* $(;)? } => {
        #[allow(unused)]
        impl Cursor<'_> {
            $(
                pub fn $name_ne(&mut self) -> $t {
                    <$t>::from_ne_bytes(self.get_bytes())
                }

                pub fn $name_le(&mut self) -> $t {
                    <$t>::from_ne_bytes(self.get_bytes())
                }

                pub fn $name_be(&mut self) -> $t {
                    <$t>::from_ne_bytes(self.get_bytes())
                }
            )*
        }
    }
}

cursor_impl_read! {
    read_u16, read_u16_le, read_u16_be, u16;
    read_i16, read_i16_le, read_i16_be, i16;
    read_u32, read_u32_le, read_u32_be, u32;
    read_i32, read_i32_le, read_i32_be, i32;
    read_u64, read_u64_le, read_u64_be, u64;
    read_i64, read_i64_le, read_i64_be, i64;
    read_u128, read_u128_le, read_u128_be, u128;
    read_i128, read_i128_le, read_i128_be, i128;
    read_usize, read_usize_le, read_usize_be, usize;
    read_isize, read_isize_le, read_isize_be, isize;
}
