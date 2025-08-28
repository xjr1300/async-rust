use std::ptr;

struct SelfReferential {
    data: String,
    self_pointer: *const String,
}

impl SelfReferential {
    fn new(data: String) -> Self {
        let mut sr = Self {
            data,
            self_pointer: ptr::null(),
        };
        sr.self_pointer = &sr.data as *const String;
        sr
    }

    fn print(&self) {
        unsafe {
            println!("{}", *self.self_pointer);
        }
    }
}

fn main() {
    let first = SelfReferential::new(String::from("first"));
    let moved_first = first; // Move the struct
    moved_first.print();
}
