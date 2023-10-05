use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

static JSON: &str = include_str!("data/srid_units.json");

fn main() {
    let json: HashMap<i32, &'static str> = serde_json::from_str(JSON).unwrap();

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());
    let mut unit_by_srid = phf_codegen::Map::new();

    for (srid, unit) in json.iter() {
        unit_by_srid.entry(*srid, &format!("\"{}\"", unit));
    }

    writeln!(
        &mut file,
        "static UNIT_BY_SRID: phf::Map<i32, &'static str> = \n{};\n",
        unit_by_srid.build()
    )
    .unwrap();
}
