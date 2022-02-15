use anyhow::{bail, ensure, Result};

use crate::strings::countable::Countable;

use super::keys::{Key, KeyItem, RecordId};

struct KeyColumns {
    original: Vec<Option<usize>>,
    sorted: Vec<usize>,
}

#[non_exhaustive]
pub struct SheetRow<'a> {
    sheet: &'a Sheet,
    id: RecordId,
}

#[non_exhaustive]
pub struct Sheet {
    rows: Vec<Vec<String>>,
    column_count: usize,
    input_index: usize,
    key_columns: KeyColumns,
}

pub struct SheetIterator<'a> {
    sheet: &'a Sheet,
    next_index: usize,
}

pub enum SheetRowSection<'a> {
    Key(&'a str),
    NonKey(Vec<&'a str>),
}

impl KeyColumns {
    fn sorted(original: &[Option<usize>]) -> Vec<usize> {
        let mut values: Vec<_> = original.iter().flat_map(
            |column| if let Some(index) = column { vec![*index] } else { vec![] }
        ).collect();
        values.sort();
        values
    }

    fn new(original: Vec<Option<usize>>) -> Self {
        let sorted = Self::sorted(&original);
        Self {
            original,
            sorted,
        }
    }

    fn split<'a>(&self, data: &[&'a str]) -> Vec<SheetRowSection<'a>> {
        let indices: Vec<_> = self.sorted.iter().map(|index| *index as i32).collect();
        let mut sections = vec![];
        for (&key, &next_key) in [-1].iter().chain(indices.iter()).zip(
            indices.iter().chain([data.len() as i32].iter())
        ) {
            if key >= 0 {
                sections.push(SheetRowSection::Key(data[key as usize]));
            }
            sections.push(SheetRowSection::NonKey(
                ((key + 1)..next_key).map(|index| data[index as usize]).collect(),
            ));
        }
        sections
    }
}

impl Sheet {
    fn normalize_column(column: i32, count: usize) -> Result<Option<usize>> {
        let count = count as i32;
        if column == 0 {
            Ok(None)
        } else {
            let normalized = if column > 0 {
                column - 1
            } else {
                count + column
            };
            ensure!(
                (0..count).contains(&normalized),
                "Column {column} is out of bounds (total columns: {count})",
            );
            Ok(Some(normalized as usize))
        }
    }

    fn check_rectangular(data: &[Vec<String>]) -> Result<usize> {
        if let Some(first_row) = data.first() {
            for (index, current_row) in data.iter().enumerate() {
                if current_row.len() != first_row.len() {
                    let count_columns = |row: &Vec<String>| row.len().count_with("column");
                    bail!(
                        "The first row has {first_columns}, but row #{n} has {nth_columns}.",
                        first_columns = count_columns(first_row),
                        nth_columns = count_columns(current_row),
                        n = index + 1,
                    )
                }
            }
            Ok(first_row.len())
        } else {
            Ok(0)
        }
    }

    fn check_convert_columns(columns: &[i32], count: usize) -> Result<Vec<Option<usize>>> {
        let mut result = vec![];
        for column in columns {
            let normalized = Self::normalize_column(*column, count)?;
            result.push(normalized);
        }
        let largest_positive = columns.iter().filter(|value| **value > 0).max();
        let smallest_negative = columns.iter().filter(|value| **value < 0).min();
        match (largest_positive, smallest_negative) {
            (Some(largest), Some(smallest)) => ensure!(
                largest - smallest <= count as i32,
                "Positively indexed columns must precede negatively indexed columns; \
                 got {smallest} ~ {smallest_normalized} <= {largest}",
                smallest_normalized = Self::normalize_column(*smallest, count)?.unwrap(),
            ),
            _ => (),
        }
        Ok(result)
    }

    pub fn new(rows: Vec<Vec<String>>, key_columns: &[i32], input_index: usize) -> Result<Self> {
        let column_count = Self::check_rectangular(&rows)?;
        Ok(Sheet {
            rows,
            column_count,
            input_index,
            key_columns: KeyColumns::new(Self::check_convert_columns(key_columns, column_count)?),
        })
    }

    pub fn split_empty_by_key<'a>(&self, filler: &'a str) -> Vec<SheetRowSection<'a>> {
        self.key_columns.split(&vec![filler; self.column_count])
    }
}

impl<'a> Iterator for SheetIterator<'a> {
    type Item = SheetRow<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let row_index = self.next_index;
        if row_index < self.sheet.rows.len() {
            self.next_index += 1;
            Some(SheetRow {
                sheet: self.sheet,
                id: RecordId {
                    input_index: self.sheet.input_index,
                    row_index,
                },
            })
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for &'a Sheet {
    type Item = SheetRow<'a>;
    type IntoIter = SheetIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SheetIterator {
            sheet: self,
            next_index: 0,
        }
    }
}

impl<'a> SheetRow<'a> {
    fn data(&self) -> &'a Vec<String> {
        &self.sheet.rows[self.id.row_index]
    }

    pub fn len(&self) -> usize {
        self.data().len()
    }

    pub fn key<'b>(&'b self) -> Key<'a, 'b> {
        Key::new(
            self.sheet.key_columns.original.iter().map(|column| {
                if let Some(index) = column {
                    KeyItem::Data(&self.data()[*index])
                } else {
                    KeyItem::Id(&self.id)
                }
            }).collect(),
        )
    }

    pub fn split_by_key(&self) -> Vec<SheetRowSection<'a>> {
        self.sheet.key_columns.split(&self.data().iter().map(String::as_str).collect::<Vec<_>>())
    }
}
