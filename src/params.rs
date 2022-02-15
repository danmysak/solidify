use std::path::PathBuf;

pub struct ParamNames {
    pub allow_single_column: String,
    pub allow_multi_merge: String,
}

pub struct Params {
    pub inputs: Vec<PathBuf>,
    pub output: PathBuf,
    pub delimiter: u8,
    pub common_columns: Vec<i32>,
    pub allow_single_column: bool,
    pub allow_multi_merge: bool,
    pub filler: String,
    pub similarity_warn_level: Option<f64>,
    pub warn_unmatched: bool,
    pub names: ParamNames,
}
