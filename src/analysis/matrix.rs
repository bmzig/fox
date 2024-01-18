use crate::analysis::Matrix;
use crate::dydx::Position;

use std::{time, thread, sync::Arc};

#[derive(Debug)]
pub struct RegressionLines {
    upper_alpha: f64,
    lower_alpha: f64,
    alpha: f64,
    beta: f64,
}

impl<const N: usize> Matrix<N> {

    pub fn new(t: [u64; N], p: [f64; N]) -> Self {
        Self {
            t, 
            p,
        }
    }

    // I don't actually use this function except for testing.
    pub fn linreg(&self) -> (f64, f64) {
        let a = N as f64;
        let b = self.t.iter().sum::<u64>() as f64;
        let c = b as f64;
        let d = self.t.iter().map(|tx| tx * tx).sum::<u64>() as f64;

        let det = (a * d) - (b * c);
        let idet = 1f64/det;
        let na = d * idet;
        let nb = -1f64 * b * idet;
        let nc = -1f64 * c * idet;
        let nd = a * idet;

        let y1 = self.p.iter().sum::<f64>();
        let y2 = {
            let mut sum = 0f64;
            for (p, t) in self.p.iter().zip(self.t) {
                sum += p * t as f64;
            }
            sum
        };

        let a0 = (na * y1) + (nb * y2);
        let a1 = (nc * y1) + (nd * y2);
        (a0, a1)
    }

    // Get resistance lines, where each line is 'x' standard deviations from the original fit.
    pub fn resistances(&self, x: f64) -> RegressionLines {
     
        // Calculate linear regression alpha and beta
        let a = N as f64;
        let b = self.t.iter().sum::<u64>() as f64;
        let c = b as f64;
        let d = self.t.iter().map(|tx| tx * tx).sum::<u64>() as f64;

        let det = (a * d) - (b * c);
        let idet = 1f64/det;
        let na = d * idet;
        let nb = -1f64 * b * idet;
        let nc = -1f64 * c * idet;
        let nd = a * idet;

        let y1 = self.p.iter().sum::<f64>();
        let y2 = {
            let mut sum = 0f64;
            for (p, t) in self.p.iter().zip(self.t) {
                sum += p * t as f64;
            }
            sum
        };

        let alpha = (na * y1) + (nb * y2);
        let beta = (nc * y1) + (nd * y2);

        // Get standard deviation of data
        let mean = y1/N as f64;
        let mut variance = 0f64;

        for p in self.p { variance += (p - mean) * (p - mean); }
        variance /= (N-1) as f64;

        let sigma = variance.sqrt();
        let upper_alpha = alpha + (sigma * x);
        let lower_alpha = alpha - (sigma * x);

        RegressionLines::new(upper_alpha, lower_alpha, alpha, beta)
    }
}

impl RegressionLines {

    pub fn new(upper_alpha: f64, lower_alpha: f64, alpha: f64, beta: f64) -> Self {
        Self {
            upper_alpha,
            lower_alpha,
            alpha,
            beta,
        }
    }

    pub fn upper_alpha(&self) -> f64 {
        self.upper_alpha
    }

    pub fn lower_alpha(&self) -> f64 {
        self.lower_alpha
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    pub fn beta(&self) -> f64 {
        self.beta
    }

}

#[cfg(test)]
mod regression_tests {

    use super::*;
    use std::time;
    use rand::Rng;
    
    #[test]
    fn resistance_lines() {
        let t = [1, 2, 3, 3, 4, 5, 5, 6, 6];
        let p = [50.1, 50.0, 49.9, 50.2, 50.5, 50.2, 49.8, 49.6, 49.2];

        eprintln!("{:?}", p.iter().sum::<f64>()/p.len() as f64);
        let matrix = Matrix::new(t, p);
        let (alpha, beta) = matrix.linreg();
        eprintln!("{:?}", alpha);
        eprintln!("{:?}", beta);

        // 1 standard deviation away from mean.
        let resistances = matrix.resistances(1f64);
        eprintln!("{:?}", resistances);
        panic!();
    }
}
