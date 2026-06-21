//! A small demonstration of the pregex API.

use pregex::{flags, Regex};

fn main() {
    let re = Regex::new(r"(?P<word>\w+)\s+(?P<num>\d+)").unwrap();
    let m = re.find("hello 42 world").unwrap();
    println!("match: {:?}", m.as_str());
    println!("word = {:?}", m.name("word"));
    println!("num  = {:?}", m.name("num"));

    // Repeated captures — a signature mrab-regex feature.
    let re = Regex::new(r"(\w)+").unwrap();
    let m = re.find("abc").unwrap();
    println!("captures of group 1: {:?}", m.captures(1));

    // Lookbehind (variable length) + case-insensitive.
    let re = Regex::new(r"(?i)(?<=foo)bar").unwrap();
    println!("lookbehind: {:?}", re.find("FOObar"));

    // Atomic group prevents catastrophic backtracking.
    let re = Regex::new(r"a(?>b*)b").unwrap();
    println!("atomic find: {:?}", re.find("abbbbc"));

    // Replace with named groups.
    let re = Regex::new(r"(?P<first>\w+),(?P<second>\w+)").unwrap();
    println!(
        "replace: {}",
        re.replace_all("a,b x,y", "${second} ${first}")
    );

    // Flags constant.
    let re = Regex::new_with_flags(r"hello", flags::IGNORECASE | flags::MULTILINE).unwrap();
    println!("flags: {:x}", re.flags().bits());
}
