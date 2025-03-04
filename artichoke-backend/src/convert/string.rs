use std::borrow::Cow;
use std::str;

use crate::convert::UnboxRubyError;
use crate::core::{ConvertMut, TryConvertMut};
use crate::error::Error;
use crate::types::Rust;
use crate::value::Value;
use crate::Artichoke;

impl ConvertMut<String, Value> for Artichoke {
    fn convert_mut(&mut self, value: String) -> Value {
        // Ruby `String`s are just bytes, so get a pointer to the underlying
        // `&[u8]` infallibly and convert that to a `Value`.
        self.convert_mut(value.as_bytes())
    }
}

impl ConvertMut<&str, Value> for Artichoke {
    fn convert_mut(&mut self, value: &str) -> Value {
        // Ruby `String`s are just bytes, so get a pointer to the underlying
        // `&[u8]` infallibly and convert that to a `Value`.
        self.convert_mut(value.as_bytes())
    }
}

impl<'a> ConvertMut<Cow<'a, str>, Value> for Artichoke {
    fn convert_mut(&mut self, value: Cow<'a, str>) -> Value {
        match value {
            Cow::Borrowed(string) => self.convert_mut(string),
            Cow::Owned(string) => self.convert_mut(string),
        }
    }
}

impl TryConvertMut<Value, String> for Artichoke {
    type Error = Error;

    fn try_convert_mut(&mut self, value: Value) -> Result<String, Self::Error> {
        TryConvertMut::<_, &str>::try_convert_mut(self, value).map(String::from)
    }
}

impl<'a> TryConvertMut<Value, &'a str> for Artichoke {
    type Error = Error;

    fn try_convert_mut(&mut self, value: Value) -> Result<&'a str, Self::Error> {
        let bytes = self.try_convert_mut(value)?;
        // This converter requires that the bytes be valid UTF-8 data. If the
        // `Value` contains binary data, use the `Vec<u8>` or `&[u8]` converter.
        let string = str::from_utf8(bytes).map_err(|_| UnboxRubyError::new(&value, Rust::String))?;
        Ok(string)
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::quickcheck;
    use std::convert::TryFrom;
    use std::slice;

    use crate::test::prelude::*;

    #[test]
    fn fail_convert() {
        let mut interp = interpreter().unwrap();
        // get a mrb_value that can't be converted to a primitive type.
        let value = interp.eval(b"Object.new").unwrap();
        let result = value.try_into_mut::<String>(&mut interp);
        assert!(result.is_err());
    }

    quickcheck! {
        #[allow(clippy::needless_pass_by_value)]
        fn convert_to_string(s: String) -> bool {
            let mut interp = interpreter().unwrap();
            let value = interp.convert_mut(s.clone());
            let string = unsafe {
                interp
                    .with_ffi_boundary(|mrb| {
                        let ptr = sys::mrb_string_value_ptr(mrb, value.inner());
                        let len = sys::mrb_string_value_len(mrb, value.inner());
                        let len = usize::try_from(len).unwrap();
                        slice::from_raw_parts(ptr as *const u8, len)
                    })
                    .unwrap()
            };
            s.as_bytes() == string
        }

        #[allow(clippy::needless_pass_by_value)]
        fn string_with_value(s: String) -> bool {
            let mut interp = interpreter().unwrap();
            let value = interp.convert_mut(s.clone());
            value.to_s(&mut interp) == s.as_bytes()
        }

        #[allow(clippy::needless_pass_by_value)]
        #[cfg(feature = "core-regexp")]
        fn utf8string_borrowed(string: String) -> bool {
            let mut interp = interpreter().unwrap();
            // Borrowed converter
            let value = interp.convert_mut(string.as_str());
            let len = value
                .funcall(&mut interp, "length", &[], None)
                .and_then(|value| value.try_into::<usize>(&interp))
                .unwrap();
            if len != string.chars().count() {
                return false;
            }
            let zero = interp.convert(0);
            let first = value
                .funcall(&mut interp, "[]", &[zero], None)
                .and_then(|value| value.try_into_mut::<Option<String>>(&mut interp))
                .unwrap();
            let mut iter = string.chars();
            if let Some(ch) = iter.next() {
                if first != Some(ch.to_string()) {
                    return false;
                }
            } else if first.is_some() {
                return false;
            }
            let recovered: String = interp.try_convert_mut(value).unwrap();
            if recovered != string {
                return false;
            }
            true
        }

        #[allow(clippy::needless_pass_by_value)]
        #[cfg(feature = "core-regexp")]
        fn utf8string_owned(string: String) -> bool {
            let mut interp = interpreter().unwrap();
            // Owned converter
            let value = interp.convert_mut(string.clone());
            let len = value
                .funcall(&mut interp, "length", &[], None)
                .and_then(|value| value.try_into::<usize>(&interp))
                .unwrap();
            if len != string.chars().count() {
                return false;
            }
            let zero = interp.convert(0);
            let first = value
                .funcall(&mut interp, "[]", &[zero], None)
                .and_then(|value| value.try_into_mut::<Option<String>>(&mut interp))
                .unwrap();
            let mut iter = string.chars();
            if let Some(ch) = iter.next() {
                if first != Some(ch.to_string()) {
                    return false;
                }
            } else if first.is_some() {
                return false;
            }
            let recovered: String = interp.try_convert_mut(value).unwrap();
            if recovered != string {
                return false;
            }
            true
        }

        #[allow(clippy::needless_pass_by_value)]
        fn roundtrip(s: String) -> bool {
            let mut interp = interpreter().unwrap();
            let value = interp.convert_mut(s.clone());
            let value = value.try_into_mut::<String>(&mut interp).unwrap();
            value == s
        }

        fn roundtrip_err(b: bool) -> bool {
            let mut interp = interpreter().unwrap();
            let value = interp.convert(b);
            let result = value.try_into_mut::<String>(&mut interp);
            result.is_err()
        }
    }
}
