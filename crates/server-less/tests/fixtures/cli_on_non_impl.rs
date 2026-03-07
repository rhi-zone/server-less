use server_less::cli;

// #[cli] applied to a function (not an impl block) should produce a meaningful error.
// The macro calls `parse_macro_input!(item as ItemImpl)` so this should fail to parse.
#[cli]
fn my_function() -> String {
    "hello".into()
}

fn main() {}
