# Solidify: CSV data consolidator

## Introduction

Solidify is a command line tool that allows to combine CSV/TSV files like so:

<table>
<thead><tr><td>
Input 1
</td><td>
Input 2
</td></tr></thead>
<tr><td><pre>
Country	Population
China	1.41B
India	1.39B
US	333M
</pre></td><td><pre>
Country	Area
Canada	10M km²
US	9.8M km²
China	9.6M km²
</pre></td></tr>
</table>

Output:

```
Country	Population	Area
China	1.41B	9.6M km²
India	1.39B	N/A
US	333M	9.8M km²
Canada	N/A	10M km²
```

## Installation

Install [Rust](https://www.rust-lang.org/), then run:

```
cargo install solidify
```

## Usage

### Basic usage

The [introductory example](#introduction) can be reproduced using the following command:

```
solidify -i 1.tsv 2.tsv -o out.tsv --shared 1 --filler N/A
```

Here `--shared 1` refers to the fact that the first column is [shared](#shared-columns) between `1.tsv` and `2.tsv`—and it is this column’s contents that are used to identify and match records across the files.

### Inputs

You can specify two or more input files to be combined using `-i` or `--inputs`:

```
-i 1.tsv 2.tsv
--inputs a.csv b.csv c.csv
```

### Output

You have to specify the output file with `-o` or `--output`:

```
-o out.tsv
--output combined.csv
```

To prevent accidental overriding of data, the output path must be different from all the input paths.

### Delimiter

Solidify does not attempt to autodetect delimiters used in your data, so you need to manually specify one (the same delimiter will also be applied to the output). If a delimiter is not provided, the default will be assumed: the tab character (`"	"`). To prevent any mistakes when specifying a delimiter, Solidify will exit with an error if each of the input files appears to have a single column (unless you explicitly [allow](#single-columned-inputs) it).

Only ASCII characters are currently accepted as delimiters. You can provide one with `-d` or `--delimiter`:

```
-d ,
--delimiter "	"
```

### Shared columns

Using `-s`, or `--shared`, you can specify which of the columns of your data are shared between input files (in case there are multiple columns, each value has to be provided separately by repeating the option):

```
-s 1
--shared 3
-s 2 -s 3 -s 8
```

These columns will be used to identify which records should be matched and merged.

#### Reverse indexing

Negative values refer to columns in reverse order, that is, `-1` refers to the last column, `-2` to the second-to-last, etc. To guarantee consistency of output data, negatively indexed columns are not allowed to precede any positively indexed column in any of the input files.

#### Merge all vs. merge none

If no shared columns are specified, any pair of records will be considered matching (given [multiway merge](#multiway-merge) is allowed).

For instance, running

```
solidify -i 1.tsv 2.tsv -o out.tsv --multi
```

against the [introductory example](#introduction) would produce the following output:

```
Country	Population	Country	Area
China	1.41B	Canada	10M km²
India	1.39B	US	9.8M km²
US	333M	China	9.6M km²
```

In contrast, if a special value of `0` is provided as the value of `-s`/`--shared`, no two records will be considered matching. Running

```
solidify -i 1.tsv 2.tsv -o out.tsv -s 0 -s 1 --filler N/A
```

will hence produce:

```
Country	Population	N/A
China	1.41B	N/A
India	1.39B	N/A
US	333M	N/A
Country	N/A	Area
Canada	N/A	10M km²
US	N/A	9.8M km²
China	N/A	9.6M km²
```

### Single-columned inputs

To prevent any mistakes when specifying a [delimiter](#delimiter), Solidify will exit with an error if each of the input files appears to have a single column. To allow processing such inputs, pass the `--single` flag.

### Multiway merge

When data admits multiple ways to match records, Solidify needs to be passed the `--multi` flag to proceed. If the flag is set, records will be matched in the order they appear in input files (see [Merge all vs. merge none](#merge-all-vs-merge-none) for an example).

### Filler

The value of `--filler` determines the content of unmatched cells (`N/A` in the [introductory example](#introduction)). If not provided, an empty string will be used.

### Warn on similar records

To track records not being matched due to typos, you may set `--warn-similar` to a positive integer. If the combined edit distance between a pair of records does not exceed this value, and yet the records are not identical, a warning will be displayed. Only values in columns declared as [shared](#shared-columns) are compared.

### Warn on unmatched records

When the flag `--warn-unmatched` is set, any records that could not be matched with any records in at least one of the other input files will be reported.
