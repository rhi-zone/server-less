use server_less::http;

// #[http] applied to a struct (not an impl block) should produce a meaningful error.
#[http]
struct MyService {
    name: String,
}

fn main() {}
