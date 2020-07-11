use cacher::Cacher;

fn main() {
    let mut c = Cacher::new(|(a, b)| a + b);

    c.value((1, 1));
    c.value((2, 2));
    c.value((3, 3));

    println!("{:?}", c.values);
}
