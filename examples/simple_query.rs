// File: examples/simple_query.rs
use chrono::NaiveDate;
use informix_rust::{errors::Result, Connection};

fn main() -> Result<()> {
    println!("Starting the application");

    let conn = Connection::new()?;
    println!("Connection object created");

    // Connect to the database
    let conn_string =
        "SERVER=myserver;DATABASE=mydb;HOST=localhost;SERVICE=9088;UID=username;PWD=password";
    conn.connect_with_string(conn_string)?;
    println!("Connected successfully");

    // Prepare a simple query
    let query = "SELECT * FROM mytable WHERE id = ? and date_col >= ? and date_col <= ?";

    let stmt = conn.prepare(query)?;
    println!("Statement prepared successfully");

    // Prepare parameters
    let id: i32 = 1;
    let date = NaiveDate::from_ymd_opt(2024, 9, 7).unwrap();

    // Bind parameters
    stmt.bind_parameter(1, &id)?;
    stmt.bind_parameter(2, &date)?;
    stmt.bind_parameter(3, &date)?;
    println!("Parameters bound successfully");

    // Execute the query
    stmt.execute()?;
    println!("Query executed successfully");

    // Fetch and print results
    println!("Fetching results:");

    let mut row_count = 0;
    while let Some(row) = stmt.fetch()? {
        row_count += 1;
        if row_count <= 5 {
            // Print only the first 5 rows
            println!("Row {}: {:?}", row_count, row);
        }
    }
    println!("Total rows fetched: {}", row_count);

    if row_count == 0 {
        println!("No rows returned. This might indicate an issue with the query or parameters.");
    }

    Ok(())
}
