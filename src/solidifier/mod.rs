mod keys;
mod sheet;

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, ensure, Result};
use csv;
use lcs::LcsTable;

use crate::params::Params;

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

fn compare_strings(a: &str, b: &str) -> f64 {
    let [a, b] = [a, b].map(|side: &str| side.chars().collect::<Vec<_>>());
    let common = LcsTable::new(&a, &b).longest_common_subsequence().len();
    let total = a.len() + b.len();
    if total > 0 { (2 * common) as f64 / total as f64 } else { 1.0 }
}

fn compare_key_items(a: &KeyItem, b: &KeyItem) -> f64 {
    match (a, b) {
        (KeyItem::Data(a), KeyItem::Data(b)) => compare_strings(a, b),
        _ => if a == b { 1.0 } else { 0.0 },
    }
}

fn compare_keys(a: &Key, b: &Key) -> f64 {
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
                 consider passing the {flag} flag. The ambiguous record is: {key}.",
                flag = params.names.allow_multi_merge,
            );
            merged.append(&mut merge(&row_sets.iter().zip(sheets).collect::<Vec<_>>(), params));
            if params.warn_unmatched && row_sets.iter().any(|set| set.len() != row_sets[0].len()) {
                eprintln!("Unmatched records encountered: {key}.");
            }
            if let Some(similarity_warn_level) = params.similarity_warn_level {
                for another_key in by_key.keys() {
                    if compare_keys(key, another_key) >= similarity_warn_level {
                        eprintln!("Similar records encountered:\n{key}\n{another_key}\n");
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
        sheets.push(Sheet::new(read(path, &params)?, &params.common_columns, index).with_context(
            || format!("Could not process {}.", path.display())
        )?);
    }
    ensure_proper_delimiter(&sheets, &params)?;
    write(&params.output, &match_and_merge(&sheets, &params)?, params)?;
    Ok(())
}
