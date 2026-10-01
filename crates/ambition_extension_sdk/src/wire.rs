//! The portable wire encoding: little-endian, length-prefixed, no type tags.
//!
//! A loaded module and the host exchange bytes, never Rust values. Every
//! value on the wire has a known kind from its context (a port, a schema, a
//! descriptor field), so the bytes carry no tags. A reader that runs out of
//! bytes, finds a bad UTF-8 string or finds bytes left over returns a
//! [`WireError`]; it never guesses a default.

/// Why bytes could not be decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WireError {
    /// The bytes ended before the value did.
    Truncated,
    /// A string is not UTF-8.
    BadString,
    /// A tag or discriminant has no meaning here.
    BadTag(u8),
    /// The value ended before the bytes did.
    Trailing(usize),
    /// A length is larger than the limit for that place.
    TooLong(u64),
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// The largest sequence or string length a reader accepts.
pub const MAX_LEN: u64 = 1 << 24;

pub fn put_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}

pub fn put_bool(out: &mut Vec<u8>, v: bool) {
    out.push(u8::from(v));
}

pub fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub fn put_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub fn put_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// The IEEE bit pattern. The wire keeps `-0.0` and NaN payloads as they are;
/// the canonical CHECKSUM of a record is a different encoding
/// ([`crate::Value::encode`]).
pub fn put_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_bits().to_le_bytes());
}

pub fn put_vec2(out: &mut Vec<u8>, v: [f32; 2]) {
    put_f32(out, v[0]);
    put_f32(out, v[1]);
}

pub fn put_str(out: &mut Vec<u8>, s: &str) {
    put_u32(out, s.len() as u32);
    out.extend_from_slice(s.as_bytes());
}

pub fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    put_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

pub fn put_opt<T>(out: &mut Vec<u8>, v: Option<T>, put: impl FnOnce(&mut Vec<u8>, T)) {
    match v {
        None => out.push(0),
        Some(v) => {
            out.push(1);
            put(out, v);
        }
    }
}

/// A cursor over bytes.
pub struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        let end = self.pos.checked_add(n).ok_or(WireError::Truncated)?;
        let slice = self.bytes.get(self.pos..end).ok_or(WireError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], WireError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    pub fn u8(&mut self) -> Result<u8, WireError> {
        Ok(self.take(1)?[0])
    }

    pub fn bool(&mut self) -> Result<bool, WireError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(WireError::BadTag(other)),
        }
    }

    pub fn u16(&mut self) -> Result<u16, WireError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub fn u32(&mut self) -> Result<u32, WireError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub fn i32(&mut self) -> Result<i32, WireError> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    pub fn u64(&mut self) -> Result<u64, WireError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub fn f32(&mut self) -> Result<f32, WireError> {
        Ok(f32::from_bits(self.u32()?))
    }

    pub fn vec2(&mut self) -> Result<[f32; 2], WireError> {
        Ok([self.f32()?, self.f32()?])
    }

    /// A length prefix, checked against [`MAX_LEN`].
    pub fn read_len(&mut self) -> Result<usize, WireError> {
        let n = u64::from(self.u32()?);
        if n > MAX_LEN {
            return Err(WireError::TooLong(n));
        }
        Ok(n as usize)
    }

    pub fn str(&mut self) -> Result<&'a str, WireError> {
        let n = self.read_len()?;
        std::str::from_utf8(self.take(n)?).map_err(|_| WireError::BadString)
    }

    pub fn bytes(&mut self) -> Result<&'a [u8], WireError> {
        let n = self.read_len()?;
        self.take(n)
    }

    pub fn opt<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> Result<T, WireError>,
    ) -> Result<Option<T>, WireError> {
        if self.bool()? {
            Ok(Some(read(self)?))
        } else {
            Ok(None)
        }
    }

    /// Fails unless every byte was read.
    pub fn finish(&self) -> Result<(), WireError> {
        match self.bytes.len() - self.pos {
            0 => Ok(()),
            n => Err(WireError::Trailing(n)),
        }
    }
}

/// Decode a whole byte string with `read`, refusing trailing bytes.
pub fn decode_all<T>(
    bytes: &[u8],
    read: impl FnOnce(&mut Reader<'_>) -> Result<T, WireError>,
) -> Result<T, WireError> {
    let mut r = Reader::new(bytes);
    let value = read(&mut r)?;
    r.finish()?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip_and_a_short_read_is_an_error() {
        let mut out = Vec::new();
        put_u32(&mut out, 7);
        put_str(&mut out, "fan");
        put_opt(&mut out, Some([1.0f32, -2.5]), put_vec2);
        put_opt::<u64>(&mut out, None, put_u64);
        let mut r = Reader::new(&out);
        assert_eq!(r.u32(), Ok(7));
        assert_eq!(r.str(), Ok("fan"));
        assert_eq!(r.opt(Reader::vec2), Ok(Some([1.0, -2.5])));
        assert_eq!(r.opt(Reader::u64), Ok(None));
        assert_eq!(r.finish(), Ok(()));

        assert_eq!(Reader::new(&out[..5]).u64(), Err(WireError::Truncated));
        assert_eq!(decode_all(&out, |r| r.u32()), Err(WireError::Trailing(out.len() - 4)));
        assert_eq!(Reader::new(&[2]).bool(), Err(WireError::BadTag(2)));
    }
}
