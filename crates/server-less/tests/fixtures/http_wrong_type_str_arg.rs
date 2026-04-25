use server_less::http;

struct MyService;

// Passing a bool where a string is expected should produce a clear type error.
#[http(prefix = false)]
impl MyService {
    pub fn hello(&self) -> String {
        "hello".into()
    }
}

fn main() {}
