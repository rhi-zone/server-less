use server_less::cli;

struct MyApp;

#[cli]
impl MyApp {
    #[cli(default)]
    fn greet(&self) {
        println!("hello");
    }

    #[cli(default)]
    fn farewell(&self) {
        println!("goodbye");
    }
}

fn main() {}
