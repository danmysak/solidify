pub trait Literally {
    fn literally(&self) -> String;
}

impl Literally for str {
    fn literally(&self) -> String {
        format!("<{self}>")
    }
}
