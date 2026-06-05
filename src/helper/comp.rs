pub fn parse_authors() -> Vec<&'static str> {
    env!("CARGO_PKG_AUTHORS").split(':').collect()
}