
use ropey::Rope;
use std::ops::Deref;

use super::super::Text;


impl Deref for Text {
    type Target = Rope;
    fn deref(&self) -> &Self::Target { &self.rope }
}
