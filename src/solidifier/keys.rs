use std::fmt::{Display, Formatter};

use crate::strings::literally::Literally;

#[derive(Hash, Eq, PartialEq)]
pub struct RecordId {
    pub input_index: usize,
    pub row_index: usize,
}

impl Display for RecordId {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", format!(
            "row #{row} of the input #{input}",
            row = self.row_index + 1,
            input = self.input_index + 1,
        ).literally())
    }
}

#[derive(Hash, Eq, PartialEq)]
pub enum KeyItem<'a, 'b> {
    Data(&'a str),
    Id(&'b RecordId),
}

impl<'a, 'b> Display for KeyItem<'a, 'b> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", match self {
            KeyItem::Data(data) => format!("{}", data),
            KeyItem::Id(id) => format!("{}", id),
        })
    }
}

#[derive(Hash, Eq, PartialEq)]
pub struct Key<'a, 'b> {
    rows: Vec<KeyItem<'a, 'b>>,
}

impl<'a, 'b> Key<'a, 'b> {
    pub fn new(rows: Vec<KeyItem<'a, 'b>>) -> Self {
        Self {
            rows,
        }
    }
}

impl<'a, 'b, 'c> IntoIterator for &'c Key<'a, 'b> {
    type Item = &'c KeyItem<'a, 'b>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.rows.iter().map(|row| row).collect::<Vec<_>>().into_iter()
    }
}

impl<'a, 'b> Display for Key<'a, 'b> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", if self.rows.is_empty() {
            "empty set of columns".literally()
        } else {
            self.rows.iter().map(|item| item.to_string()).collect::<Vec<_>>().join(", ")
        })
    }
}
