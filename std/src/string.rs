use crate::Vec;
use crate::fmt;

pub struct String {
    internal: Vec<u8>,
}

impl String {
    pub fn new() -> Self {
        Self { internal: Vec::new() }
    }

    pub fn push(&mut self, c: char) {
        self.internal.push(c as u8);
    }

    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.internal) }
    }

    pub fn len(&self) -> usize {
        self.internal.len()
    }

    pub fn is_empty(&self) -> bool {
        self.internal.is_empty()
    }

    pub fn from_utf8(vec: Vec<u8>) -> Result<Self, core::str::Utf8Error> {
        core::str::from_utf8(&vec)?;
        Ok(Self { internal: vec })
    }
}

impl Default for String {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::ops::Deref for String {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for String {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
