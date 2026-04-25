use server_less::cli;

// #[cli] applied to an empty impl block should produce a meaningful error.
#[cli]
impl MyApp {}

fn main() {}
