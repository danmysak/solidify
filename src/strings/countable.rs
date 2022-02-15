use std::fmt::Display;

use num::Integer;

pub trait Countable {
    fn count_with(&self, noun: &str) -> String;
}

impl<T> Countable for T where T: Display + Integer {
    fn count_with(&self, noun: &str) -> String {
        format!("{} {}{}", self, noun, if self == &T::one() {""} else {"s"})
    }
}
