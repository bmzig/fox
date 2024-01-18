use crate::dydx::{InternalAccount, Markets, Exposure};

use crate::strategy::{
    market_make::MarketMake,
    second_derivative_trading::SecondDerivative,
    resistance::Resistance,
    gradient_boosting::GradientBoosting,
    market_exposure::MarketExposure
};

mod market_make;
mod second_derivative_trading;
mod resistance;
mod gradient_boosting;
mod market_exposure;

#[derive(Debug, PartialEq)]
pub enum Strategy {
    MarketMake,
    SecondDerivative,
    Resistance,
    MarketExposure,
    GradientBoosting,
}

impl Strategy {

    pub async fn run(&self, account: InternalAccount, market: Markets, exposure: Option<Exposure>, testnet: bool) -> anyhow::Result<()> {
        match *self {
            Strategy::MarketMake => {
                MarketMake::run(account, market, exposure.unwrap(), testnet).await?;
            },
            Strategy::SecondDerivative => {
                SecondDerivative::run(account, market, testnet).await?;
            }
            Strategy::MarketExposure => {
                MarketExposure::run(account, exposure.unwrap(), testnet).await?
            }
            Strategy::Resistance => {
                Resistance::run(account, market, exposure.unwrap(), testnet).await?
            }
            Strategy::GradientBoosting => {
                GradientBoosting::run(account, market, exposure.unwrap(), testnet).await?
            }

        }
        Ok(())
    }
}
