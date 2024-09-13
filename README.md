# InformixRust
InformixRust is a Rust library that provides a safe and efficient way to interact with Informix databases. It wraps the Informix CSDK (Client SDK) to offer a more Rust-friendly interface for database operations.

## Features

- Safe Rust wrapper around Informix CSDK
- Connection management with auto-reconnection support
- Prepared statements with parameter binding
- Efficient result set fetching
- Support for various SQL data types including dates
- Environment variable based configuration

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
informix_rust = "0.1.0"
```

## Usage


```
// File: examples/simple_query.rs
use informix_rust::Connection;
use chrono::NaiveDate;
use std::env;

fn main() -> Result<(), String> {
    println!("Starting the application");

    let conn = Connection::new()?;
    println!("Connection object created");

    let conn_string = &env::var("INFORMIXDB_CONN_PARAMS").expect("INFORMIXDB_CONN_PARAMS must be set");
    conn.connect_with_string(conn_string)?;

    let query = "SELECT * FROM your_table WHERE id = ? AND date <= ?";
    let stmt = conn.prepare(query)?;

    let id = 1;
    let date = NaiveDate::from_ymd_opt(2024, 9, 7).unwrap();

    stmt.bind_parameter(1, &id)?;
    stmt.bind_parameter(2, &date)?;

    stmt.execute()?;

    while let Some(row) = stmt.fetch()? {
        println!("{:?}", row);
    }

    Ok(())
}
```