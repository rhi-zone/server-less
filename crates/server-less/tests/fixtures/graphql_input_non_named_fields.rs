use server_less::graphql_input;

// #[graphql_input] on a tuple struct should error because only named fields are supported.
#[graphql_input]
struct MyInput(String, u32);

fn main() {}
