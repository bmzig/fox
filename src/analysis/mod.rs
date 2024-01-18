mod rings;
mod interpolation;
mod partitions;
mod matrix;

#[derive(Clone, Copy)]
pub struct Trade {
    pub price: f64,
    pub timestamp: u64,
}

pub struct Ring<const N: usize> {
    ring: [Trade; N],
    sum: f64,
    average: f64,
    index: usize,
    full: bool,
    size: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct Partition {
    sum: f64,
    average: f64,
    high: f64,
    low: f64,
    direction: f64,
    volume: usize,
}

#[derive(Debug)]
pub struct Matrix<const N: usize> {
    t: [u64; N],
    p: [f64; N],
}

pub struct Polynomial(Vec<f64>);
pub struct Interpolation;
