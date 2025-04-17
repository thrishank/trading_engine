#[cfg(test)]
mod tests {
    use trading_engine::{Order, OrderBook, get_current_timestamp};

    #[test]
    fn test_simple_trade_match() {
        let mut order_book = OrderBook::new();

        // Create a sell order
        let sell_order = Order {
            type_op: "CREATE".to_string(),
            account_id: "1".to_string(),
            amount: "1.0".to_string(),
            order_id: "1".to_string(),
            pair: "BTC/USDC".to_string(),
            limit_price: "50000.0".to_string(),
            side: "SELL".to_string(),
            timestamp: get_current_timestamp(),
        };

        // Add the sell order to the order book
        let trades = order_book.process_order(sell_order);
        assert_eq!(trades.len(), 0); // No trades yet

        // Create a matching buy order
        let buy_order = Order {
            type_op: "CREATE".to_string(),
            account_id: "2".to_string(),
            amount: "0.5".to_string(),
            order_id: "2".to_string(),
            pair: "BTC/USDC".to_string(),
            limit_price: "50000.0".to_string(),
            side: "BUY".to_string(),
            timestamp: get_current_timestamp(),
        };

        // Add the buy order to the order book
        let trades = order_book.process_order(buy_order);

        // Check that a trade was created
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].taker_order_id, "2");
        assert_eq!(trades[0].maker_order_id, "1");
        assert_eq!(trades[0].price, "50000.0");
        assert_eq!(trades[0].amount, "0.5");

        // Check that the sell order is still in the order book with reduced amount
        let order_book_entries = order_book.generate_order_book_output();
        assert_eq!(order_book_entries.len(), 1);
        assert_eq!(order_book_entries[0].order_id, "1");
        assert_eq!(order_book_entries[0].amount, "0.5");
    }

    #[test]
    fn test_complete_fill() {
        let mut order_book = OrderBook::new();

        // Create a sell order
        let sell_order = Order {
            type_op: "CREATE".to_string(),
            account_id: "1".to_string(),
            amount: "1.0".to_string(),
            order_id: "1".to_string(),
            pair: "BTC/USDC".to_string(),
            limit_price: "50000.0".to_string(),
            side: "SELL".to_string(),
            timestamp: get_current_timestamp(),
        };

        // Add the sell order to the order book
        order_book.process_order(sell_order);

        // Create a matching buy order that completely fills the sell order
        let buy_order = Order {
            type_op: "CREATE".to_string(),
            account_id: "2".to_string(),
            amount: "1.0".to_string(),
            order_id: "2".to_string(),
            pair: "BTC/USDC".to_string(),
            limit_price: "50000.0".to_string(),
            side: "BUY".to_string(),
            timestamp: get_current_timestamp(),
        };

        // Add the buy order to the order book
        let trades = order_book.process_order(buy_order);

        // Check that a trade was created
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].amount, "1.0");

        // Check that both orders are removed from the order book
        let order_book_entries = order_book.generate_order_book_output();
        assert_eq!(order_book_entries.len(), 0);
    }

    #[test]
    fn test_price_priority() {
        let mut order_book = OrderBook::new();

        // Create sell orders at different prices
        let sell_order_1 = Order {
            type_op: "CREATE".to_string(),
            account_id: "1".to_string(),
            amount: "1.0".to_string(),
            order_id: "1".to_string(),
            pair: "BTC/USDC".to_string(),
            limit_price: "51000.0".to_string(),
            side: "SELL".to_string(),
            timestamp: get_current_timestamp(),
        };

        let sell_order_2 = Order {
            type_op: "CREATE".to_string(),
            account_id: "1".to_string(),
            amount: "1.0".to_string(),
            order_id: "2".to_string(),
            pair: "BTC/USDC".to_string(),
            limit_price: "50000.0".to_string(),
            side: "SELL".to_string(),
            timestamp: get_current_timestamp(),
        };

        // Add the sell orders to the order book
        order_book.process_order(sell_order_1);
        order_book.process_order(sell_order_2);

        // Create a matching buy order
        let buy_order = Order {
            type_op: "CREATE".to_string(),
            account_id: "2".to_string(),
            amount: "1.0".to_string(),
            order_id: "3".to_string(),
            pair: "BTC/USDC".to_string(),
            limit_price: "51000.0".to_string(),
            side: "BUY".to_string(),
            timestamp: get_current_timestamp(),
        };

        // Add the buy order to the order book
        let trades = order_book.process_order(buy_order);

        // Check that the buy order matched with the lowest-priced sell order
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].maker_order_id, "2");
        assert_eq!(trades[0].price, "50000.0");
    }

    #[test]
    fn test_delete_order() {
        let mut order_book = OrderBook::new();

        // Create a sell order
        let sell_order = Order {
            type_op: "CREATE".to_string(),
            account_id: "1".to_string(),
            amount: "1.0".to_string(),
            order_id: "1".to_string(),
            pair: "BTC/USDC".to_string(),
            limit_price: "50000.0".to_string(),
            side: "SELL".to_string(),
            timestamp: get_current_timestamp(),
        };

        // Add the sell order to the order book
        order_book.process_order(sell_order.clone());

        // Check that the order is in the order book
        let order_book_entries = order_book.generate_order_book_output();
        assert_eq!(order_book_entries.len(), 1);

        // Delete the order
        let mut delete_order = sell_order.clone();
        delete_order.type_op = "DELETE".to_string();
        order_book.process_order(delete_order);

        // Check that the order is removed from the order book
        let order_book_entries = order_book.generate_order_book_output();
        assert_eq!(order_book_entries.len(), 0);
    }
}
