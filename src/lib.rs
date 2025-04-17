use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub type_op: String,
    pub account_id: String,
    pub amount: String,
    pub order_id: String,
    pub pair: String,
    pub limit_price: String,
    pub side: String,
    #[serde(skip)]
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookEntry {
    pub order_id: String,
    pub account_id: String,
    pub pair: String,
    pub side: String,
    pub amount: String,
    pub price: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub trade_id: String,
    pub taker_order_id: String,
    pub maker_order_id: String,
    pub pair: String,
    pub price: String,
    pub amount: String,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct OrderBook {
    pub bids: BTreeMap<Decimal, Vec<Order>>, // Buy orders, sorted by price in descending order
    pub asks: BTreeMap<Decimal, Vec<Order>>, // Sell orders, sorted by price in ascending order
    pub trades: Vec<Trade>,
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            trades: Vec::new(),
        }
    }

    pub fn process_order(&mut self, order: Order) -> Vec<Trade> {
        let mut new_trades = Vec::new();

        match order.type_op.as_str() {
            "CREATE" => {
                if order.side == "BUY" {
                    new_trades = self.match_buy_order(order.clone());
                    // If the order is not completely filled, add it to the order book
                    if let Some(remaining_order) = self.get_remaining_order(&order, &new_trades) {
                        self.add_order(remaining_order);
                    }
                } else if order.side == "SELL" {
                    new_trades = self.match_sell_order(order.clone());
                    // If the order is not completely filled, add it to the order book
                    if let Some(remaining_order) = self.get_remaining_order(&order, &new_trades) {
                        self.add_order(remaining_order);
                    }
                }
            }
            "DELETE" => {
                self.remove_order(&order);
            }
            _ => {
                eprintln!("Unknown order type: {}", order.type_op);
            }
        }

        // Add new trades to the trade history
        self.trades.extend(new_trades.clone());

        new_trades
    }

    pub fn match_buy_order(&mut self, order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        let mut remaining_amount = Decimal::from_str(&order.amount).unwrap();
        let buy_price = Decimal::from_str(&order.limit_price).unwrap();

        // Look for matching sell orders
        let mut asks_to_remove = Vec::new();
        let mut orders_to_update = Vec::new();

        for (ask_price, ask_orders) in self.asks.iter_mut() {
            if *ask_price > buy_price {
                break;
            }

            for ask_order in ask_orders.iter_mut() {
                if remaining_amount <= Decimal::ZERO {
                    break;
                }

                let ask_amount = Decimal::from_str(&ask_order.amount).unwrap();

                let trade_amount = remaining_amount.min(ask_amount);

                let trade = Trade {
                    trade_id: Uuid::new_v4().to_string(),
                    taker_order_id: order.order_id.clone(),
                    maker_order_id: ask_order.order_id.clone(),
                    pair: order.pair.clone(),
                    price: ask_price.to_string(),
                    amount: trade_amount.to_string(),
                    timestamp: get_current_timestamp(),
                };

                trades.push(trade);

                // Update the remaining amount
                remaining_amount -= trade_amount;

                if trade_amount < ask_amount {
                    // Partial fill
                    let new_amount = (ask_amount - trade_amount).to_string();
                    ask_order.amount = new_amount;
                } else {
                    // Complete fill
                    // Mark this order to be removed
                    orders_to_update.push((ask_price.clone(), ask_order.order_id.clone()));
                }
            }

            // Check if all orders at this price level are filled
            if ask_orders.is_empty()
                || ask_orders
                    .iter()
                    .all(|o| Decimal::from_str(&o.amount).unwrap() <= Decimal::ZERO)
            {
                asks_to_remove.push(*ask_price);
            }
        }

        // Remove filled orders
        for (price, order_id) in orders_to_update {
            if let Some(orders) = self.asks.get_mut(&price) {
                orders.retain(|o| o.order_id != order_id);
                if orders.is_empty() {
                    asks_to_remove.push(price);
                }
            }
        }

        // Remove empty price levels
        for price in asks_to_remove {
            self.asks.remove(&price);
        }

        trades
    }

    fn match_sell_order(&mut self, order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        let mut remaining_amount = Decimal::from_str(&order.amount).unwrap();
        let sell_price = Decimal::from_str(&order.limit_price).unwrap();

        // We need to iterate through bids in reverse order (highest price first)
        let mut bids_to_process: Vec<(Decimal, Vec<Order>)> = self
            .bids
            .iter()
            .filter(|(bid_price, _)| **bid_price >= sell_price)
            .map(|(price, orders)| (*price, orders.clone()))
            .collect();

        // Sort by price (highest first)
        bids_to_process.sort_by(|(price_a, _), (price_b, _)| price_b.cmp(price_a));

        // Process each bid
        for (bid_price, mut bid_orders) in bids_to_process {
            if remaining_amount <= Decimal::ZERO {
                break;
            }

            let mut orders_to_update = Vec::new();

            for bid_order in bid_orders.iter_mut() {
                if remaining_amount <= Decimal::ZERO {
                    break;
                }

                let bid_amount = Decimal::from_str(&bid_order.amount).unwrap();

                // Calculate the amount that can be matched
                let trade_amount = remaining_amount.min(bid_amount);

                // Create a new trade
                let trade = Trade {
                    trade_id: Uuid::new_v4().to_string(),
                    taker_order_id: order.order_id.clone(),
                    maker_order_id: bid_order.order_id.clone(),
                    pair: order.pair.clone(),
                    price: bid_price.to_string(),
                    amount: trade_amount.to_string(),
                    timestamp: get_current_timestamp(),
                };

                trades.push(trade);

                // Update the remaining amount
                remaining_amount -= trade_amount;

                // Update the bid order in the actual orderbook
                if let Some(orders) = self.bids.get_mut(&bid_price) {
                    for o in orders.iter_mut() {
                        if o.order_id == bid_order.order_id {
                            if trade_amount < bid_amount {
                                // Partial fill
                                o.amount = (bid_amount - trade_amount).to_string();
                            } else {
                                // Complete fill
                                orders_to_update.push(o.order_id.clone());
                            }
                            break;
                        }
                    }
                }
            }

            // Remove filled orders
            if let Some(orders) = self.bids.get_mut(&bid_price) {
                orders.retain(|o| !orders_to_update.contains(&o.order_id));
                if orders.is_empty() {
                    self.bids.remove(&bid_price);
                }
            }
        }

        trades
    }

    pub fn add_order(&mut self, order: Order) {
        let price = Decimal::from_str(&order.limit_price).unwrap();

        if order.side == "BUY" {
            self.bids.entry(price).or_insert_with(Vec::new).push(order)
        } else {
            self.asks.entry(price).or_insert_with(Vec::new).push(order)
        }
    }

    pub fn remove_order(&mut self, order: &Order) {
        let price = Decimal::from_str(&order.limit_price).unwrap();

        if order.side == "BUY" {
            if let Some(orders) = self.bids.get_mut(&price) {
                orders.retain(|o| o.order_id != order.order_id);
                if orders.is_empty() {
                    self.bids.remove(&price);
                }
            }
        } else {
            if let Some(orders) = self.asks.get_mut(&price) {
                orders.retain(|o| o.order_id != order.order_id);
                if orders.is_empty() {
                    self.asks.remove(&price);
                }
            }
        }
    }

    fn get_remaining_order(&self, original_order: &Order, trades: &[Trade]) -> Option<Order> {
        let original_amount = Decimal::from_str(&original_order.amount).unwrap();

        // Calculate traded amount
        let traded_amount: Decimal = trades
            .iter()
            .filter(|t| t.taker_order_id == original_order.order_id)
            .map(|t| Decimal::from_str(&t.amount).unwrap())
            .sum();

        // Calculate remaining amount
        let remaining_amount = original_amount - traded_amount;

        if remaining_amount > Decimal::ZERO {
            // Create a new order with the remaining amount
            let mut remaining_order = original_order.clone();
            remaining_order.amount = remaining_amount.to_string();
            Some(remaining_order)
        } else {
            None
        }
    }
    pub fn generate_order_book_output(&self) -> Vec<OrderBookEntry> {
        let mut entries = Vec::new();

        for (price, orders) in &self.bids {
            for order in orders {
                entries.push(OrderBookEntry {
                    order_id: order.order_id.clone(),
                    account_id: order.account_id.clone(),
                    pair: order.pair.clone(),
                    side: order.side.clone(),
                    amount: order.amount.clone(),
                    price: price.to_string(),
                    timestamp: order.timestamp,
                });
            }
        }

        for (price, orders) in &self.asks {
            for order in orders {
                entries.push(OrderBookEntry {
                    order_id: order.order_id.clone(),
                    account_id: order.account_id.clone(),
                    pair: order.pair.clone(),
                    side: order.side.clone(),
                    amount: order.amount.clone(),
                    price: price.to_string(),
                    timestamp: order.timestamp,
                });
            }
        }

        entries
    }
}

// Get current timestamp in milliseconds
pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
