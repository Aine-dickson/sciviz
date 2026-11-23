//! Compile an item document to a structure file, so the output can be read.
//!
//! `cargo run --release -p svf-core --example compile_item -- fixtures/items/simple-circuit.item.ron out.ron`

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output] = args.as_slice() else {
        eprintln!("usage: compile_item <item.ron> <structure.ron>");
        std::process::exit(2);
    };
    let text = std::fs::read_to_string(input).unwrap_or_else(|e| {
        eprintln!("cannot read {input}: {e}");
        std::process::exit(1);
    });
    match svf_core::item::compile_str(&text) {
        Ok(item) => {
            std::fs::write(output, item.to_ron()).unwrap_or_else(|e| {
                eprintln!("cannot write {output}: {e}");
                std::process::exit(1);
            });
            let t = &item.structure.timelines()[0];
            println!("parts {}, beats {}, duration {:.1} s", item.structure.parts().len(), t.beats().len(), t.duration());
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
