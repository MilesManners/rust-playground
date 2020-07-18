use regex::bytes::Regex;

fn main() {
    let text = b"Retroactively relinquishing remunerations is reprehensible.";
    for mat in Regex::new(r"(.*?)(?:\s|$)").unwrap().find_iter(text) {
        println!("{}", String::from_utf8_lossy(&text[mat.range()]))
    }
}
