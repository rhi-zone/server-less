use server_less::cli;

struct MyApp;

// A parameter named `manual` collides with the injected `--manual` global flag.
// The collision guard turns clap's runtime panic into a spanned compile error.
#[cli]
impl MyApp {
    fn render(&self, manual: String) {
        println!("{manual}");
    }
}

fn main() {}
