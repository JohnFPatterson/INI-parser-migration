use inih_core::{ini_parse_string_with_alloc, HeapHooks, IniConfig, IniHandler};

struct H;
impl IniHandler for H {
    fn handle(&mut self, section: &str, name: Option<&str>, value: Option<&str>, _: i32) -> bool {
        println!(
            "H section={} name={} value={}",
            section,
            name.unwrap_or("(null)"),
            value.unwrap_or("(null)")
        );
        true
    }
}
struct A;
impl HeapHooks for A {
    fn malloc(&mut self, size: usize) {
        println!("M {size}");
    }
    fn free(&mut self) {
        println!("F");
    }
    fn realloc(&mut self, size: usize) {
        println!("R {size}");
    }
}
fn main() {
    let cfg = IniConfig {
        use_stack: false,
        allow_realloc: true,
        initial_alloc: 12,
        ..Default::default()
    };
    let mut h = H;
    let mut a = A;
    let e = ini_parse_string_with_alloc(
        &cfg,
        "[sec]\nfoo = bar\n  continued\nbazz = buzz\n",
        &mut h,
        &mut a,
    );
    println!("RESULT {e}");
}
