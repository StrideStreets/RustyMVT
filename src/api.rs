use axum::extract::Path;
use std::collections::HashMap;

struct TableRegistry {
    name: String,
    schemas: HashMap<String, Schema>,
}

struct Schema {
    name: String,
    tables: Vec<String>,
}

struct Tile {
    z: isize,
    x: isize,
    y: isize,
}

pub fn load_table_registry() {
    //Will autopopulate a table registry for a given database, in the mold of the placeholder defined below
    todo!()
}

fn table_registry() -> TableRegistry {
    let public = Schema {
        name: "public".to_owned(),
        tables: vec![
            String::from("nodes"),
            String::from("edges"),
            String::from("centerlines"),
        ],
    };

    let topo = Schema {
        name: "topo".to_owned(),
        tables: vec![
            String::from("nodes"),
            String::from("edges"),
            String::from("centerlines"),
        ],
    };

    let mut schemas: HashMap<String, Schema> = HashMap::new();
    schemas.insert("public".to_owned(), public);
    schemas.insert("topo".to_owned(), topo);

    let registry = TableRegistry {
        name: "test_registry".to_owned(),
        schemas: schemas,
    };

    return registry;
}

pub fn serve_tile(
    Path((schema, table, x, y, z, ext)): Path<(String, String, isize, isize, isize, String)>,
) -> Option<String> {
    let table_registry_placeholder = table_registry();

    fn check_registry_for_table(
        r: TableRegistry,
        schema: String,
        table: String,
    ) -> Result<(), &'static str> {
        let res = match r.schemas.get(&schema) {
            Some(s) => s.tables.contains(&table),
            None => false,
        };

        match res {
            true => return Ok(()),
            false => return Err("Table not found."),
        }
    }

    fn parse_coordinates(t: Tile) {
        todo!()
    }

    fn make_query() {
        todo!()
    }

    fn execute_query() {
        todo!()
    }

    fn handle_results() {
        todo!()
    }

    return Some("Placeholder String!".to_owned());
}
