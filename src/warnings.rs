const DIVIDER: &str = "----------";

pub fn warn(lines: &[&str]) -> () {
    eprintln!("{}", DIVIDER);
    for line in lines {
        eprintln!("{}", line);
    }
}
