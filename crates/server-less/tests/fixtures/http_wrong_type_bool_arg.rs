use server_less::http;

struct MyService;

// Passing a string where a bool is expected should produce a clear type error.
#[http(openapi = "yes")]
impl MyService {
    pub fn hello(&self) -> String {
        "hello".into()
    }
}

fn main() {}
