use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::market::Symbol;
use crate::orders::{Fill, Side};

/// Portfolio position for a specific symbol
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub symbol: Symbol,
    pub quantity: Decimal,
    pub average_price: Decimal,
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub last_updated: DateTime<Utc>,
}

impl Position {
    pub fn new(symbol: Symbol) -> Self {
        Self {
            symbol,
            quantity: Decimal::ZERO,
            average_price: Decimal::ZERO,
            market_value: Decimal::ZERO,
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            last_updated: Utc::now(),
        }
    }
    
    pub fn is_long(&self) -> bool {
        self.quantity > Decimal::ZERO
    }
    
    pub fn is_short(&self) -> bool {
        self.quantity < Decimal::ZERO
    }
    
    pub fn is_flat(&self) -> bool {
        self.quantity == Decimal::ZERO
    }
    
    pub fn apply_fill(&mut self, fill: &Fill) {
        let fill_quantity = match fill.side {
            Side::Buy => fill.quantity,
            Side::Sell => -fill.quantity,
        };
        
        let new_quantity = self.quantity + fill_quantity;
        
        // Handle position changes
        if self.quantity == Decimal::ZERO {
            // Opening new position
            self.quantity = new_quantity;
            self.average_price = fill.price;
        } else if (self.quantity > Decimal::ZERO && fill_quantity > Decimal::ZERO) || 
                   (self.quantity < Decimal::ZERO && fill_quantity < Decimal::ZERO) {
            // Adding to existing position
            let total_cost = self.quantity.abs() * self.average_price + fill_quantity.abs() * fill.price;
            let total_quantity = self.quantity.abs() + fill_quantity.abs();
            self.average_price = total_cost / total_quantity;
            self.quantity = new_quantity;
        } else {
            // Reducing or closing position
            let closed_quantity = fill_quantity.abs().min(self.quantity.abs());
            let remaining_quantity = self.quantity.abs() - closed_quantity;
            
            // Calculate realized P&L
            let realized_pnl = match self.quantity > Decimal::ZERO {
                true => (fill.price - self.average_price) * closed_quantity,
                false => (self.average_price - fill.price) * closed_quantity,
            };
            self.realized_pnl += realized_pnl;
            
            if remaining_quantity == Decimal::ZERO {
                // Position closed
                self.quantity = Decimal::ZERO;
                self.average_price = Decimal::ZERO;
            } else {
                // Position reduced
                self.quantity = match self.quantity > Decimal::ZERO {
                    true => remaining_quantity,
                    false => -remaining_quantity,
                };
            }
        }
        
        self.last_updated = fill.executed_at;
    }
    
    pub fn update_market_price(&mut self, market_price: Decimal) {
        self.market_value = self.quantity.abs() * market_price;
        self.unrealized_pnl = match self.quantity {
            q if q > Decimal::ZERO => (market_price - self.average_price) * self.quantity,
            q if q < Decimal::ZERO => (self.average_price - market_price) * self.quantity.abs(),
            _ => Decimal::ZERO,
        };
        self.last_updated = Utc::now();
    }
    
    pub fn total_pnl(&self) -> Decimal {
        self.realized_pnl + self.unrealized_pnl
    }
}

/// Portfolio state and management
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Portfolio {
    pub account_id: String,
    pub initial_capital: Decimal,
    pub cash: Decimal,
    pub positions: HashMap<Symbol, Position>,
    pub total_equity: Decimal,
    pub total_pnl: Decimal,
    pub total_realized_pnl: Decimal,
    pub total_unrealized_pnl: Decimal,
    pub total_commissions: Decimal,
    pub last_updated: DateTime<Utc>,
    pub daily_returns: Vec<DailyReturn>,
}

impl Portfolio {
    pub fn new(account_id: String, initial_capital: Decimal) -> Self {
        Self {
            account_id,
            initial_capital,
            cash: initial_capital,
            positions: HashMap::new(),
            total_equity: initial_capital,
            total_pnl: Decimal::ZERO,
            total_realized_pnl: Decimal::ZERO,
            total_unrealized_pnl: Decimal::ZERO,
            total_commissions: Decimal::ZERO,
            last_updated: Utc::now(),
            daily_returns: Vec::new(),
        }
    }
    
    pub fn apply_fill(&mut self, fill: &Fill) {
        // Update cash
        self.cash += fill.net_amount();
        self.total_commissions += fill.commission;
        
        // Update position
        let position = self.positions
            .entry(fill.symbol.clone())
            .or_insert_with(|| Position::new(fill.symbol.clone()));
        position.apply_fill(fill);
        
        // Remove flat positions
        if position.is_flat() {
            self.positions.remove(&fill.symbol);
        }
        
        self.last_updated = fill.executed_at;
        self.update_totals();
    }
    
    pub fn update_market_prices(&mut self, prices: &HashMap<Symbol, Decimal>) {
        for (symbol, price) in prices {
            if let Some(position) = self.positions.get_mut(symbol) {
                position.update_market_price(*price);
            }
        }
        self.update_totals();
    }
    
    fn update_totals(&mut self) {
        self.total_realized_pnl = self.positions.values()
            .map(|p| p.realized_pnl)
            .sum();
            
        self.total_unrealized_pnl = self.positions.values()
            .map(|p| p.unrealized_pnl)
            .sum();
            
        self.total_pnl = self.total_realized_pnl + self.total_unrealized_pnl;
        
        let market_value: Decimal = self.positions.values()
            .map(|p| p.market_value)
            .sum();
            
        self.total_equity = self.cash + market_value;
    }
    
    pub fn get_position(&self, symbol: &Symbol) -> Option<&Position> {
        self.positions.get(symbol)
    }
    
    pub fn get_available_cash(&self) -> Decimal {
        // Simple implementation - could add margin calculations
        self.cash.max(Decimal::ZERO)
    }
    
    pub fn get_total_return(&self) -> Decimal {
        if self.initial_capital > Decimal::ZERO {
            (self.total_equity - self.initial_capital) / self.initial_capital
        } else {
            Decimal::ZERO
        }
    }
    
    pub fn add_daily_return(&mut self, date: DateTime<Utc>, daily_return: Decimal) {
        self.daily_returns.push(DailyReturn {
            date,
            portfolio_value: self.total_equity,
            daily_return,
            cumulative_return: self.get_total_return(),
        });
    }
    
    pub fn get_sharpe_ratio(&self, risk_free_rate: Decimal) -> Option<Decimal> {
        if self.daily_returns.len() < 2 {
            return None;
        }
        
        let returns: Vec<Decimal> = self.daily_returns.iter()
            .map(|r| r.daily_return)
            .collect();
            
        let mean_return = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
        let excess_return = mean_return - risk_free_rate / Decimal::from(252); // Daily risk-free rate
        
        let variance = returns.iter()
            .map(|r| {
                let diff = *r - mean_return;
                let diff_f64 = diff.to_f64().unwrap_or(0.0);
                Decimal::from_f64_retain(diff_f64 * diff_f64).unwrap_or_default()
            })
            .sum::<Decimal>() / Decimal::from(returns.len() - 1);
            
        let variance_f64 = variance.to_f64().unwrap_or(0.0);
        let std_dev = Decimal::from_f64_retain(variance_f64.sqrt()).unwrap_or_default();
        
        if std_dev > Decimal::ZERO {
            let annualization_factor = Decimal::from_f64_retain((252.0_f64).sqrt()).unwrap_or_default();
            Some(excess_return / std_dev * annualization_factor) // Annualized
        } else {
            None
        }
    }
    
    pub fn get_max_drawdown(&self) -> Decimal {
        if self.daily_returns.is_empty() {
            return Decimal::ZERO;
        }
        
        let mut max_value = self.initial_capital;
        let mut max_drawdown = Decimal::ZERO;
        
        for daily_return in &self.daily_returns {
            if daily_return.portfolio_value > max_value {
                max_value = daily_return.portfolio_value;
            }
            
            let drawdown = (max_value - daily_return.portfolio_value) / max_value;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
        
        max_drawdown
    }
}

/// Daily portfolio performance record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyReturn {
    pub date: DateTime<Utc>,
    pub portfolio_value: Decimal,
    pub daily_return: Decimal,
    pub cumulative_return: Decimal,
}

/// Portfolio event for the event-driven engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PortfolioEvent {
    PositionOpened { symbol: Symbol, position: Position },
    PositionClosed { symbol: Symbol, realized_pnl: Decimal },
    PositionUpdated { symbol: Symbol, position: Position },
    DailySnapshot { portfolio: Portfolio },
}

/// Risk management parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskLimits {
    pub max_position_size: Decimal,
    pub max_portfolio_leverage: Decimal,
    pub max_daily_loss: Decimal,
    pub max_drawdown: Decimal,
    pub position_concentration_limit: Decimal, // Max % of portfolio in single position
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_position_size: Decimal::from(10000),
            max_portfolio_leverage: Decimal::from(2),
            max_daily_loss: Decimal::new(5, 2), // 5%
            max_drawdown: Decimal::new(20, 2), // 20%
            position_concentration_limit: Decimal::new(25, 2), // 25%
        }
    }
} 