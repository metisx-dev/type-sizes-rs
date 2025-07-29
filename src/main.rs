use std::env;
use std::fs::File;
use std::io::{self, BufReader};

mod parse_layouts;
mod types;
use parse_layouts::parse_layouts;
use types::VerificationError;

fn main() -> io::Result<()> {
    let path = get_input_path();
    let layouts = parse_layouts(BufReader::new(File::open(path)?))?;

    for (i, layout) in layouts.iter().enumerate() {
        let mut found_error = false;

        println!("//---------- Layout {} ----------//", i + 1);

        if !layout.unhandled_lines.is_empty() {
            println!(
                "  - error reason: unhandled lines found\n{:?}\n",
                layout.unhandled_lines
            );
            found_error = true;
        }

        if let Err(e) = layout.verify() {
            println!("  - error reason: {}\n", format_verification_error(e));
            found_error = true;
        }

        if found_error {
            println!("\x1b[1;31m{:#?}\x1b[0m", layout);
        } else {
            println!("{:#?}", layout);
        }
    }

    Ok(())
}

fn get_input_path() -> String {
    env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo r -- <type-sizes-path>");
        std::process::exit(1);
    })
}

fn format_verification_error(e: VerificationError) -> String {
    use VerificationError::*;
    match e {
        StructSizeMismatch { expected, actual } => format!(
            "mismatch struct size (expected: {}, actual: {})",
            expected, actual
        ),
        VariantSizeMismatch {
            variant_name,
            expected,
            actual,
        } => format!(
            "mismatch variant size (name: {}, expected: {}, actual: {})",
            variant_name, expected, actual
        ),
        UnionSizeMismatch {
            expected,
            actual_max,
        } => format!(
            "mismatch union size (expected: {}, actual: {})",
            expected, actual_max
        ),
        EnumTotalSizeMismatch {
            expected,
            calculated_min,
        } => format!(
            "mismatch enum size (expected: {}, calculated_min: {})",
            expected, calculated_min
        ),
    }
}
