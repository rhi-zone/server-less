use server_less::http;

struct MyService;

// Passing an integer where #[param(env)] expects a string should produce a clear error.
#[http]
impl MyService {
    pub fn hello(&self, #[param(env = 42)] name: String) -> String {
        name
    }
}

fn main() {}
