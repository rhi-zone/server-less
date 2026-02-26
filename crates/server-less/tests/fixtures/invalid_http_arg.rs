use server_less::http;

struct MyService;

#[http(invalid_arg = true)]
impl MyService {
    pub fn hello(&self) -> String {
        "hello".into()
    }
}

fn main() {}
