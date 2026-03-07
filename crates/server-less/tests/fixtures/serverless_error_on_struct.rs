use server_less::ServerlessError;

// ServerlessError can only be derived for enums, not structs.
#[derive(Debug, ServerlessError)]
struct MyError {
    code: u32,
    message: String,
}

fn main() {}
