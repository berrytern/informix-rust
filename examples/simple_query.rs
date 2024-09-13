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
    println!("Connected successfully");

    let query = "SELECT catgtin.*, prod.*, pauta.*, pautaemb.*, unimedida.*, tpemb.*, categ.nocategproduto, pgtin.* 
             FROM tbleg_gtinproduto prod, tbleg_catgtincat catgtin, tbfis_pautagtin pgtin, tbfis_pauta pauta, 
             tbleg_categproduto categ, tbsic_unidmedida unimedida, 
             OUTER (tbfis_pautaprodemb pautaemb, tbleg_tpembalagem tpemb) 
             WHERE categ.sqcategproduto = catgtin.sqcategproduto 
             AND catgtin.sqgtinproduto = prod.sqgtinproduto 
             AND catgtin.sqgtinproduto = pgtin.sqgtinproduto 
             AND pgtin.sqpauta = pauta.sqpauta 
             AND pgtin.sqpauta = pautaemb.sqpauta 
             AND pautaemb.sqtpembalagem = tpemb.sqtpembalagem 
             AND pauta.squnidmedida = unimedida.squnidmedida 
             AND pauta.stregistro = 1 AND pauta.stpauta = 1 
             AND categ.sqcategproduto IN (?) 
             AND (pauta.dtinicial <= ? AND (pauta.dtfinal IS NULL OR pauta.dtfinal >= ?))";

    let stmt = conn.prepare(query)?;
    println!("Statement prepared successfully");
    
    // Prepare parameters
    let category_id = 21i32;
    let date = NaiveDate::from_ymd_opt(2024, 9, 7).unwrap();

    // Bind parameters
    stmt.bind_parameter(1, &category_id)?;
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
        if row_count <= 5 {  // Print only the first 5 rows
            println!("Row {}: {:?}", row_count, row);
        }
    }
    println!("Total rows fetched: {}", row_count);

    if row_count == 0 {
        println!("No rows returned. This might indicate an issue with the query or parameters.");
    }

    Ok(())
}