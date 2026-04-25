use server_less::http;

// #[http] applied to a generic impl block should produce a meaningful error.
#[http]
impl<T> Foo<T> {
    fn get_item(&self) -> String {
        String::new()
    }
}

fn main() {}
