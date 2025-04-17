use std::fs::File;
use std::io::{self, Read, Write};
use trading_engine::{Order, OrderBook, get_current_timestamp};

fn main() -> io::Result<()> {
    // Read orders from file
    let mut file = File::open("orders.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse orders
    let orders: Vec<Order> = serde_json::from_str(&contents)?;

    // Create order book
    let mut order_book = OrderBook::new();

    // Process orders
    for mut order in orders {
        // Set timestamp
        order.timestamp = get_current_timestamp();

        // Process order
        order_book.process_order(order);
    }

    let order_book_output = order_book.generate_order_book_output();
    let trades_output = order_book.trades;

    let order_book_json = serde_json::to_string_pretty(&order_book_output)?;
    let mut order_book_file = File::create("orderbook.json")?;
    order_book_file.write_all(order_book_json.as_bytes())?;

    let trades_json = serde_json::to_string_pretty(&trades_output)?;
    let mut trades_file = File::create("trades.json")?;
    trades_file.write_all(trades_json.as_bytes())?;

    println!("Processing complete. Check orderbook.json and trades.json for results.");

    Ok(())
}
