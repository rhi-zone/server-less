use server_less::cli;

struct Parent {
    child: Child,
}

#[derive(Clone)]
struct Child;

// A slug-mount param named `manual` collides with the child command's injected
// `--manual` global flag. The parent disables its own manual surface, but the
// guard must still catch the collision on the child (which keeps `--manual`).
#[cli(manual = false)]
impl Parent {
    fn items(&self, manual: String) -> &Child {
        let _ = manual;
        &self.child
    }
}

#[cli]
impl Child {
    fn show(&self) {
        println!("show");
    }
}

fn main() {}
