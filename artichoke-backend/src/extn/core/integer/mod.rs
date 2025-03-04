use std::convert::TryFrom;
use std::mem;

use crate::extn::core::numeric::{self, Coercion, Outcome};
use crate::extn::prelude::*;

pub mod mruby;
pub mod trampoline;

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Integer(Int);

impl Convert<Integer, Value> for Artichoke {
    #[inline]
    fn convert(&self, from: Integer) -> Value {
        self.convert(from.0)
    }
}

impl TryConvert<Value, Integer> for Artichoke {
    type Error = Error;

    #[inline]
    fn try_convert(&self, value: Value) -> Result<Integer, Self::Error> {
        let num = self.try_convert(value)?;
        Ok(Integer(num))
    }
}

impl From<Int> for Integer {
    #[inline]
    fn from(int: Int) -> Self {
        Self(int)
    }
}

impl From<Integer> for i64 {
    #[inline]
    fn from(int: Integer) -> Self {
        int.as_i64()
    }
}

impl From<Integer> for f64 {
    #[inline]
    fn from(int: Integer) -> Self {
        int.as_f64()
    }
}

impl From<Integer> for Outcome {
    #[inline]
    fn from(int: Integer) -> Self {
        Self::Integer(int.into())
    }
}

impl From<Int> for Outcome {
    #[inline]
    fn from(int: Int) -> Self {
        Self::Integer(int)
    }
}

impl Integer {
    /// Constructs a new, default, zero `Integer`.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline]
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        self.0
    }

    #[allow(clippy::cast_precision_loss)]
    #[inline]
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0 as f64
    }

    pub fn chr(self, interp: &mut Artichoke, encoding: Option<Value>) -> Result<Vec<u8>, Error> {
        if let Some(encoding) = encoding {
            let mut message = b"encoding parameter of Integer#chr (given ".to_vec();
            message.extend(encoding.inspect(interp));
            message.extend(b") not supported");
            Err(NotImplementedError::from(message).into())
        } else {
            // When no encoding is supplied, MRI assumes the encoding is
            // either ASCII or ASCII-8BIT.
            //
            // - `Integer`s from 0..127 result in a `String` with ASCII
            //   encoding.
            // - `Integer`s from 128..256 result in a `String` with binary
            //   (ASCII-8BIT) encoding.
            // - All other integers raise a `RangeError`.
            //
            // ```txt
            // [2.6.3] > [0.chr, 0.chr.encoding]
            // => ["\x00", #<Encoding:US-ASCII>]
            // [2.6.3] > [127.chr, 127.chr.encoding]
            // => ["\x7F", #<Encoding:US-ASCII>]
            // [2.6.3] > [128.chr, 128.chr.encoding]
            // => ["\x80", #<Encoding:ASCII-8BIT>]
            // [2.6.3] > [255.chr, 255.chr.encoding]
            // => ["\xFF", #<Encoding:ASCII-8BIT>]
            // [2.6.3] > [256.chr, 256.chr.encoding]
            // Traceback (most recent call last):
            //         5: from /usr/local/var/rbenv/versions/2.6.3/bin/irb:23:in `<main>'
            //         4: from /usr/local/var/rbenv/versions/2.6.3/bin/irb:23:in `load'
            //         3: from /usr/local/var/rbenv/versions/2.6.3/lib/ruby/gems/2.6.0/gems/irb-1.0.0/exe/irb:11:in `<top (required)>'
            //         2: from (irb):9
            //         1: from (irb):9:in `chr'
            // RangeError (256 out of char range)
            // ```
            match u8::try_from(self.as_i64()) {
                // ASCII encoding | Binary/ASCII-8BIT encoding
                // Without `Encoding` support, these two arms are the same
                Ok(chr @ 0..=127) | Ok(chr @ 128..=255) => {
                    // Create a single byte `String` from the character given by
                    // `self`.
                    Ok(vec![chr])
                }
                _ => {
                    let mut message = String::new();
                    itoa::fmt(&mut message, self.as_i64()).map_err(WriteError::from)?;
                    message.push_str(" out of char range");
                    Err(RangeError::from(message).into())
                }
            }
        }
    }

    #[inline]
    pub fn bit(self, bit: Int) -> Result<Self, Error> {
        if let Ok(bit) = u32::try_from(bit) {
            Ok(self.as_i64().checked_shr(bit).map_or(0, |v| v & 1).into())
        } else {
            Ok(Self(0))
        }
    }

    pub fn div(self, interp: &mut Artichoke, denominator: Value) -> Result<Outcome, Error> {
        match denominator.ruby_type() {
            Ruby::Fixnum => {
                let denom = denominator.try_into::<Int>(interp)?;
                let value = self.as_i64();
                if denom == 0 {
                    Err(ZeroDivisionError::with_message("divided by 0").into())
                } else if value < 0 && (value % denom) != 0 {
                    Ok(((value / denom) - 1).into())
                } else {
                    Ok((value / denom).into())
                }
            }
            Ruby::Float => {
                let denom = denominator.try_into::<Fp>(interp)?;
                Ok((self.as_f64() / denom).into())
            }
            _ => {
                let x = interp.convert(self);
                let coerced = numeric::coerce(interp, x, denominator)?;
                match coerced {
                    Coercion::Float(_, denom) if denom == 0.0 => {
                        Err(ZeroDivisionError::with_message("divided by 0").into())
                    }
                    Coercion::Integer(_, 0) => Err(ZeroDivisionError::with_message("divided by 0").into()),
                    Coercion::Float(numer, denom) => Ok((numer / denom).into()),
                    Coercion::Integer(numer, denom) if numer < 0 && (numer % denom) != 0 => {
                        Ok(((numer / denom) - 1).into())
                    }
                    Coercion::Integer(numer, denom) => Ok((numer / denom).into()),
                }
            }
        }
    }

    #[must_use]
    pub const fn size() -> usize {
        mem::size_of::<Int>()
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::quickcheck;

    use crate::test::prelude::*;

    quickcheck! {
        fn positive_integer_division_vm_opcode(x: u8, y: u8) -> bool {
            let mut interp = interpreter().unwrap();
            match (x, y) {
                (0, 0) => interp.eval(b"0 / 0").is_err(),
                (x, 0) | (0, x) => {
                    let expr = format!("{} / 0", x).into_bytes();
                    if interp.eval(expr.as_slice()).is_ok() {
                        return false;
                    }
                    let expr = format!("0 / {}", x).into_bytes();
                    let quotient = interp.eval(expr.as_slice()).unwrap().try_into::<Int>(&interp).unwrap();
                    quotient == 0
                }
                (x, y) => {
                    let expr = format!("{} / {}", x, y).into_bytes();
                    let quotient = interp.eval(expr.as_slice()).unwrap().try_into::<Int>(&interp).unwrap();
                    let expected = Int::from(x) / Int::from(y);
                    quotient == expected
                }
            }
        }

        fn positive_integer_division_send(x: u8, y: u8) -> bool {
            let mut interp = interpreter().unwrap();
            match (x, y) {
                (0, 0) => interp.eval(b"0.send('/', 0)").is_err(),
                (x, 0) | (0, x) => {
                    let expr = format!("{}.send('/', 0)", x).into_bytes();
                    if interp.eval(expr.as_slice()).is_ok() {
                        return false;
                    }
                    let expr = format!("0.send('/', {})", x).into_bytes();
                    let quotient = interp.eval(expr.as_slice()).unwrap().try_into::<Int>(&interp).unwrap();
                    quotient == 0
                }
                (x, y) => {
                    let expr = format!("{}.send('/', {})", x, y).into_bytes();
                    let quotient = interp.eval(expr.as_slice()).unwrap().try_into::<Int>(&interp).unwrap();
                    let expected = Int::from(x) / Int::from(y);
                    quotient == expected
                }
            }
        }

        fn negative_integer_division_vm_opcode(x: u8, y: u8) -> bool {
            let mut interp = interpreter().unwrap();
            match (x, y) {
                (0, 0) => interp.eval(b"-0 / 0").is_err(),
                (x, 0) | (0, x) => {
                    let expr = format!("-{} / 0", x).into_bytes();
                    if interp.eval(expr.as_slice()).is_ok() {
                        return false;
                    }
                    let expr = format!("0 / -{}", x).into_bytes();
                    let quotient = interp.eval(expr.as_slice()).unwrap().try_into::<Int>(&interp).unwrap();
                    quotient == 0
                }
                (x, y) => {
                    let expr = format!("-{} / {}", x, y).into_bytes();
                    let quotient = interp.eval(expr.as_slice()).unwrap().try_into::<Int>(&interp).unwrap();
                    if x % y == 0 {
                        let expected = -Int::from(x) / Int::from(y);
                        quotient == expected
                    } else {
                        // Round negative integer division toward negative infinity.
                        let expected = (-Int::from(x) / Int::from(y)) - 1;
                        quotient == expected
                    }
                }
            }
        }

        fn negative_integer_division_send(x: u8, y: u8) -> bool {
            let mut interp = interpreter().unwrap();
            match (x, y) {
                (0, 0) => interp.eval(b"-0.send('/', 0)").is_err(),
                (x, 0) | (0, x) => {
                    let expr = format!("-{}.send('/', 0)", x).into_bytes();
                    if interp.eval(expr.as_slice()).is_ok() {
                        return false;
                    }
                    let expr = format!("0.send('/', -{})", x).into_bytes();
                    let quotient = interp.eval(expr.as_slice()).unwrap().try_into::<Int>(&interp).unwrap();
                    quotient == 0
                }
                (x, y) => {
                    let expr = format!("-{}.send('/', {})", x, y).into_bytes();
                    let quotient = interp.eval(expr.as_slice()).unwrap().try_into::<Int>(&interp).unwrap();
                    if x % y == 0 {
                        let expected = -Int::from(x) / Int::from(y);
                        quotient == expected
                    } else {
                        // Round negative integer division toward negative infinity.
                        let expected = (-Int::from(x) / Int::from(y)) - 1;
                        quotient == expected
                    }
                }
            }
        }
    }
}
