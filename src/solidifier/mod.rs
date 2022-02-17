mod keys;
mod sheet;

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, ensure, Result};
use csv;
use edit_distance::edit_distance;

use crate::params::Params;
use crate::strings::countable::Countable;
use crate::warnings::warn;

use keys::{Key, KeyItem};
use sheet::{Sheet, SheetRow, SheetRowSection};

fn read(path: &PathBuf, params: &Params) -> Result<Vec<Vec<String>>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(params.delimiter)
        .from_path(path)
        .with_context(|| format!("Could not open {}.", path.display()))?;
    let mut data: Vec<Vec<String>> = vec![];
    for (index, result) in reader.records().enumerate() {
        let record = result.with_context(
            || format!("Could not process row #{} of {}.", index + 1, path.display())
        )?;
        data.push(record.iter().map(String::from).collect());
    }
    Ok(data)
}

fn write(path: &PathBuf, rows: &[Vec<&str>], params: &Params) -> Result<()> {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(params.delimiter)
        .from_path(path)
        .with_context(|| format!("Could not open {} for writing.", path.display()))?;
    for row in rows {
        writer.write_record(row).with_context(
            || format!("Could not write data to {}.", path.display())
        )?;
    }
    Ok(())
}

fn compare_strings(a: &str, b: &str) -> u32 {
    edit_distance(a, b) as u32
}

fn compare_key_items(a: &KeyItem, b: &KeyItem) -> u32 {
    match (a, b) {
        (KeyItem::Data(a), KeyItem::Data(b)) => compare_strings(a, b),
        _ => 0,
    }
}

fn compare_keys(a: &Key, b: &Key) -> u32 {
    a.into_iter().zip(b.into_iter()).map(|(a, b)| compare_key_items(a, b)).sum()
}

fn merge_row<'a>(data: &[(Option<&SheetRow<'a>>, &'a Sheet)], params: &'a Params) -> Vec<&'a str> {
    let split: Vec<_> = data.iter().map(|(row, sheet)| if let Some(row) = row {
        (row.split_by_key(), true)
    } else {
        (sheet.split_empty_by_key(&params.filler), false)
    }).collect();
    if let Some((first, _)) = split.first() {
        let count = first.len();
        assert!(split.iter().all(|(row, _)| row.len() == count));
        let mut result = vec![];
        for section_index in 0..count {
            let mut seen_filled = false;
            for (split_data, is_filled) in split.iter() {
                match &split_data[section_index] {
                    SheetRowSection::Key(value) => if *is_filled && !seen_filled {
                        seen_filled = true;
                        result.push(*value);
                    },
                    SheetRowSection::NonKey(values) => {
                        for value in values {
                            result.push(value);
                        }
                    },
                }
            }
        }
        result
    } else {
        vec![]
    }
}

fn merge<'a>(data: &[(&Vec<&SheetRow<'a>>, &'a Sheet)], params: &'a Params) -> Vec<Vec<&'a str>> {
    let max_length = data.iter().map(|(set, _)| set.len()).max().unwrap_or(0);
    (0..max_length).map(|index| merge_row(&data.iter().map(
        |&(set, sheet)| (set.get(index).map(|&v| v), sheet)
    ).collect::<Vec<_>>(), params)).collect()
}

fn match_and_merge<'a>(sheets: &'a [Sheet], params: &'a Params) -> Result<Vec<Vec<&'a str>>> {
    let rows: Vec<_> = sheets.iter().enumerate().flat_map(
        |(sheet_index, sheet)| sheet.into_iter().map(
            move |row| (row, sheet_index)
        )
    ).collect();
    let keys: Vec<_> = rows.iter().map(|(row, _)| row.key()).collect();
    let mut by_key: HashMap<&Key, Vec<Vec<&SheetRow>>> = HashMap::new();
    for ((row, sheet_index), key) in rows.iter().zip(keys.iter()) {
        let entry = by_key.entry(key).or_insert_with(|| vec![vec![]; sheets.len()]);
        entry[*sheet_index].push(&row);
    }
    let mut merged = vec![];
    for key in &keys {
        if let Some(row_sets) = by_key.remove(key) {
            ensure!(
                params.allow_multi_merge
                || row_sets.iter().all(|set| set.len() <= 1)
                || row_sets.iter().filter(|set| set.len() > 0).count() <= 1,
                "There are multiple ways to merge records. If this is intended, \
                 consider passing the {flag} flag. The ambiguous record is:\n{key}",
                flag = params.names.allow_multi_merge,
            );
            merged.append(&mut merge(&row_sets.iter().zip(sheets).collect::<Vec<_>>(), params));
            if params.warn_unmatched && row_sets.iter().any(|set| set.len() != row_sets[0].len()) {
                let comparison_key = |(_, set): &(usize, &Vec<&SheetRow>)| set.len();
                let (max_index, max_set) =
                    row_sets.iter().enumerate().max_by_key(comparison_key).unwrap();
                let (min_index, min_set) =
                    row_sets.iter().enumerate().min_by_key(comparison_key).unwrap();
                warn(&[
                    &format!(
                        "{unmatched_records} encountered \
                        (found {max_records} in {max_input}, but {min_records} in {min_input}):",
                        unmatched_records = (max_set.len() - min_set.len())
                            .count_with("unmatched record"),
                        max_records = max_set.len().count_with("such record"),
                        max_input = max_index + 1,
                        min_records = min_set.len().count_with("such record"),
                        min_input = min_index + 1,
                    ),
                    &key.to_string(),
                ]);
            }
            if params.similarity_warn_level > 0 {
                for another_key in by_key.keys() {
                    let distance = compare_keys(key, another_key);
                    if distance <= params.similarity_warn_level {
                        warn(&[
                            &format!("Similar records encountered (edit distance = {distance}):"),
                            &key.to_string(),
                            &another_key.to_string(),
                        ]);
                    }
                }
            }
        }
    }
    Ok(merged)
}

fn ensure_proper_delimiter(sheets: &[Sheet], params: &Params) -> Result<()> {
    ensure!(
        params.allow_single_column || sheets.iter().any(
            |sheet| sheet.into_iter().any(|row| row.len() > 1)
        ),
        "Your data seems not to contain any records with more than one column. \
         Did you specify the delimiter correctly? If so, consider passing the {flag} flag.",
        flag = params.names.allow_single_column,
    );
    Ok(())
}

pub fn solidify(params: &Params) -> Result<()> {
    let mut sheets = vec![];
    for (index, path) in params.inputs.iter().enumerate() {
        sheets.push(Sheet::new(read(path, &params)?, &params.shared_columns, index).with_context(
            || format!("Could not process {}.", path.display())
        )?);
    }
    ensure_proper_delimiter(&sheets, &params)?;
    write(&params.output, &match_and_merge(&sheets, &params)?, params)?;
    Ok(())
}
