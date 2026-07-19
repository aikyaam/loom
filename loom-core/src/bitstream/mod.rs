use std::io;

pub struct BitWriter {
    pub bytes: Vec<u8>,
    current_byte: u8,
    num_bits: u8,
}

impl BitWriter {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            current_byte: 0,
            num_bits: 0,
        }
    }

    pub fn write_bit(&mut self, bit: bool) {
        self.current_byte <<= 1;
        if bit {
            self.current_byte |= 1;
        }
        self.num_bits += 1;
        if self.num_bits == 8 {
            self.bytes.push(self.current_byte);
            self.current_byte = 0;
            self.num_bits = 0;
        }
    }

    pub fn write_bits(&mut self, value: u64, n: usize) {
        if n == 0 {
            return;
        }
        for i in (0..n).rev() {
            let bit = ((value >> i) & 1) != 0;
            self.write_bit(bit);
        }
    }

    pub fn write_unary(&mut self, value: u64) {
        for _ in 0..value {
            self.write_bit(false);
        }
        self.write_bit(true);
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_bits(b as u64, 8);
        }
    }

    pub fn write_utf8_uint(&mut self, val: u64) {
        let mut v = val;
        let mut bytes = 1;
        if v >= 0x80 {
            bytes = 2;
        }
        if v >= 0x800 {
            bytes = 3;
        }
        if v >= 0x10000 {
            bytes = 4;
        }
        if v >= 0x200000 {
            bytes = 5;
        }
        if v >= 0x4000000 {
            bytes = 6;
        }
        if v >= 0x80000000 {
            bytes = 7;
        }

        let mut buf = [0u8; 7];
        if bytes == 1 {
            buf[0] = v as u8;
        } else {
            for i in (1..bytes).rev() {
                buf[i] = 0x80 | ((v & 0x3F) as u8);
                v >>= 6;
            }
            let mask = (0xFF << (8 - bytes)) & 0xFF;
            buf[0] = mask as u8 | (v as u8);
        }
        for i in 0..bytes {
            self.write_bits(buf[i] as u64, 8);
        }
    }

    pub fn align_to_byte(&mut self) {
        if self.num_bits > 0 {
            self.write_bits(0, 8 - (self.num_bits as usize));
        }
    }

    pub fn flush(&mut self) {
        if self.num_bits > 0 {
            self.current_byte <<= 8 - self.num_bits;
            self.bytes.push(self.current_byte);
            self.current_byte = 0;
            self.num_bits = 0;
        }
    }

    pub fn into_bytes(mut self) -> Vec<u8> {
        self.flush();
        self.bytes
    }
}

pub struct BitReader<'a> {
    bytes: &'a [u8],
    byte_index: usize,
    bit_index: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte_index: 0,
            bit_index: 0,
        }
    }

    pub fn read_bit(&mut self) -> io::Result<bool> {
        if self.byte_index >= self.bytes.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"));
        }
        let b = self.bytes[self.byte_index];
        let bit = ((b >> (7 - self.bit_index)) & 1) != 0;
        self.bit_index += 1;
        if self.bit_index == 8 {
            self.byte_index += 1;
            self.bit_index = 0;
        }
        Ok(bit)
    }

    pub fn read_bits(&mut self, n: usize) -> io::Result<u64> {
        if n == 0 {
            return Ok(0);
        }
        let mut value = 0u64;
        for _ in 0..n {
            value <<= 1;
            if self.read_bit()? {
                value |= 1;
            }
        }
        Ok(value)
    }

    pub fn read_unary(&mut self) -> io::Result<u64> {
        let mut count = 0u64;
        while !self.read_bit()? {
            count += 1;
        }
        Ok(count)
    }

    pub fn read_bytes(&mut self, buf: &mut [u8]) -> io::Result<()> {
        for b in buf.iter_mut() {
            *b = self.read_bits(8)? as u8;
        }
        Ok(())
    }

    pub fn read_utf8_uint(&mut self) -> io::Result<u64> {
        let first = self.read_bits(8)? as u8;
        if first < 0x80 {
            return Ok(first as u64);
        }

        let bytes;
        let mut val;

        if (first & 0xE0) == 0xC0 {
            bytes = 1;
            val = (first & 0x1F) as u64;
        } else if (first & 0xF0) == 0xE0 {
            bytes = 2;
            val = (first & 0x0F) as u64;
        } else if (first & 0xF8) == 0xF0 {
            bytes = 3;
            val = (first & 0x07) as u64;
        } else if (first & 0xFC) == 0xF8 {
            bytes = 4;
            val = (first & 0x03) as u64;
        } else if (first & 0xFE) == 0xFC {
            bytes = 5;
            val = (first & 0x01) as u64;
        } else if first == 0xFE {
            bytes = 6;
            val = 0;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid UTF-8 sequence",
            ));
        }

        for _ in 0..bytes {
            let next = self.read_bits(8)? as u8;
            if (next & 0xC0) != 0x80 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid UTF-8 continuation byte",
                ));
            }
            val = (val << 6) | (next & 0x3F) as u64;
        }

        Ok(val)
    }

    pub fn align_to_byte(&mut self) {
        if self.bit_index > 0 {
            self.byte_index += 1;
            self.bit_index = 0;
        }
    }

    pub fn byte_offset(&self) -> usize {
        self.byte_index
    }

    pub fn seek_to_byte(&mut self, offset: usize) -> io::Result<()> {
        if offset > self.bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Seek out of bounds",
            ));
        }
        self.byte_index = offset;
        self.bit_index = 0;
        Ok(())
    }

    pub fn bits_left(&self) -> usize {
        if self.byte_index >= self.bytes.len() {
            0
        } else {
            (self.bytes.len() - self.byte_index) * 8 - self.bit_index as usize
        }
    }

    pub fn peek_remaining_bytes(&self) -> &[u8] {
        if self.byte_index >= self.bytes.len() {
            &[]
        } else {
            &self.bytes[self.byte_index..]
        }
    }
}
