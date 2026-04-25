use server_less::http;

// #[http] applied to an empty impl block should produce a meaningful error.
#[http]
impl MyService {}

fn main() {}
