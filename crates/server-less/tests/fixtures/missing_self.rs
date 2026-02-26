use server_less::http;

struct MyService;

#[http]
impl MyService {
    pub fn no_self() -> String {
        "hello".into()
    }
}

fn main() {}
