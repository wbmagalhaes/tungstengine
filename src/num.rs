use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

const SUFFIXES: &[&str] = &[
    "", "K", "M", "B", "T", "Qa", "Qi", "Sx", "Sp", "Oc", "No", "Dc", "UDc", "DDc", "TDc", "QaDc",
    "QiDc", "SxDc", "SpDc", "OcDc", "NoDc", "Vg",
];

#[derive(Clone, Copy)]
pub struct BigNumber {
    mantissa: f64,
    exp: i64,
}

impl BigNumber {
    pub fn text(n: impl Into<BigNumber>) -> String {
        n.into().floor().to_string()
    }

    pub fn text_short(n: impl Into<BigNumber>) -> String {
        n.into().floor().short()
    }

    pub const ZERO: BigNumber = BigNumber {
        mantissa: 0.0,
        exp: 0,
    };

    pub fn new(mantissa: f64, exp: i64) -> Self {
        BigNumber { mantissa, exp }.normalized()
    }

    pub fn from_f64(v: f64) -> Self {
        BigNumber {
            mantissa: v,
            exp: 0,
        }
        .normalized()
    }

    pub fn is_zero(&self) -> bool {
        self.mantissa == 0.0
    }

    pub fn pow(base: f64, n: f64) -> Self {
        if base <= 0.0 {
            return BigNumber::ZERO;
        }
        let l = n * base.log10();
        let exp = l.floor();
        BigNumber::new(10f64.powf(l - exp), exp as i64)
    }

    pub fn log10(&self) -> f64 {
        if self.mantissa <= 0.0 {
            return f64::NEG_INFINITY;
        }
        self.mantissa.log10() + self.exp as f64
    }

    pub fn floor(self) -> Self {
        if self.exp >= 15 {
            return self;
        }
        BigNumber::from_f64(self.as_f64().floor())
    }

    pub fn ceil(self) -> Self {
        if self.exp >= 15 {
            return self;
        }
        BigNumber::from_f64(self.as_f64().ceil())
    }

    pub fn max(self, other: Self) -> Self {
        if other > self { other } else { self }
    }

    pub fn min(self, other: Self) -> Self {
        if other < self { other } else { self }
    }

    pub fn ratio(self, other: Self) -> f64 {
        if other.is_zero() {
            return f64::INFINITY;
        }
        (self.mantissa / other.mantissa)
            * 10f64.powi((self.exp - other.exp).clamp(-300, 300) as i32)
    }

    pub fn as_f64(&self) -> f64 {
        self.mantissa * 10f64.powi(self.exp.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
    }

    fn normalized(mut self) -> Self {
        if self.mantissa == 0.0 || !self.mantissa.is_finite() {
            return BigNumber::ZERO;
        }
        let shift = self.mantissa.abs().log10().floor();
        self.mantissa /= 10f64.powf(shift);
        self.exp += shift as i64;
        if self.mantissa.abs() >= 10.0 {
            self.mantissa /= 10.0;
            self.exp += 1;
        }
        self
    }
}

impl Default for BigNumber {
    fn default() -> Self {
        BigNumber::ZERO
    }
}

impl From<f64> for BigNumber {
    fn from(v: f64) -> Self {
        BigNumber::from_f64(v)
    }
}

impl From<u64> for BigNumber {
    fn from(v: u64) -> Self {
        BigNumber::from_f64(v as f64)
    }
}

impl From<u32> for BigNumber {
    fn from(v: u32) -> Self {
        BigNumber::from_f64(v as f64)
    }
}

impl From<f32> for BigNumber {
    fn from(v: f32) -> Self {
        BigNumber::from_f64(v as f64)
    }
}

impl Add for BigNumber {
    type Output = BigNumber;
    fn add(self, rhs: BigNumber) -> BigNumber {
        if self.is_zero() {
            return rhs;
        }
        if rhs.is_zero() {
            return self;
        }
        let (hi, lo) = if self.exp >= rhs.exp {
            (self, rhs)
        } else {
            (rhs, self)
        };
        let diff = hi.exp - lo.exp;
        if diff > 17 {
            return hi;
        }
        let m = hi.mantissa + lo.mantissa / 10f64.powi(diff as i32);
        BigNumber::new(m, hi.exp)
    }
}

impl Sub for BigNumber {
    type Output = BigNumber;
    fn sub(self, rhs: BigNumber) -> BigNumber {
        self + BigNumber {
            mantissa: -rhs.mantissa,
            exp: rhs.exp,
        }
    }
}

impl Mul for BigNumber {
    type Output = BigNumber;
    fn mul(self, rhs: BigNumber) -> BigNumber {
        if self.is_zero() || rhs.is_zero() {
            return BigNumber::ZERO;
        }
        BigNumber::new(self.mantissa * rhs.mantissa, self.exp + rhs.exp)
    }
}

impl Mul<f64> for BigNumber {
    type Output = BigNumber;
    fn mul(self, rhs: f64) -> BigNumber {
        BigNumber::new(self.mantissa * rhs, self.exp)
    }
}

impl Mul<f32> for BigNumber {
    type Output = BigNumber;
    fn mul(self, rhs: f32) -> BigNumber {
        self * rhs as f64
    }
}

impl Div for BigNumber {
    type Output = BigNumber;
    fn div(self, rhs: BigNumber) -> BigNumber {
        if rhs.is_zero() {
            return BigNumber::ZERO;
        }
        BigNumber::new(self.mantissa / rhs.mantissa, self.exp - rhs.exp)
    }
}

impl Div<f64> for BigNumber {
    type Output = BigNumber;
    fn div(self, rhs: f64) -> BigNumber {
        BigNumber::new(self.mantissa / rhs, self.exp)
    }
}

impl AddAssign for BigNumber {
    fn add_assign(&mut self, rhs: BigNumber) {
        *self = *self + rhs;
    }
}

impl SubAssign for BigNumber {
    fn sub_assign(&mut self, rhs: BigNumber) {
        *self = *self - rhs;
    }
}

impl PartialEq for BigNumber {
    fn eq(&self, other: &Self) -> bool {
        self.exp == other.exp && self.mantissa == other.mantissa
    }
}

impl PartialOrd for BigNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let sl = self.mantissa.signum();
        let sr = other.mantissa.signum();
        if sl != sr {
            return sl.partial_cmp(&sr);
        }
        if self.exp != other.exp {
            let ord = self.exp.cmp(&other.exp);
            return Some(if sl < 0.0 { ord.reverse() } else { ord });
        }
        self.mantissa.partial_cmp(&other.mantissa)
    }
}

impl fmt::Debug for BigNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl Serialize for BigNumber {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.exp < 15 {
            s.serialize_f64(self.as_f64())
        } else {
            s.serialize_str(&format!("{}e{}", self.mantissa, self.exp))
        }
    }
}

impl<'de> Deserialize<'de> for BigNumber {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Num(f64),
            Text(String),
        }
        Ok(match Raw::deserialize(d)? {
            Raw::Num(v) => BigNumber::from_f64(v),
            Raw::Text(t) => {
                let (m, e) = t.split_once('e').unwrap_or((&t, "0"));
                BigNumber::new(m.parse().unwrap_or(0.0), e.parse().unwrap_or(0))
            }
        })
    }
}

impl BigNumber {
    pub fn short(&self) -> String {
        if self.exp < 3 || (self.exp / 3) as usize >= SUFFIXES.len() {
            return self.to_string();
        }
        let scaled = self.mantissa.abs() * 10f64.powi((self.exp % 3) as i32);
        let sign = if self.mantissa < 0.0 { "-" } else { "" };
        let digits = if scaled >= 100.0 { 0 } else { 1 };
        format!(
            "{sign}{scaled:.digits$}{}",
            SUFFIXES[(self.exp / 3) as usize]
        )
    }
}

impl fmt::Display for BigNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            return write!(f, "0");
        }
        let neg = self.mantissa < 0.0;
        let exp = self.exp;

        if exp < 3 {
            let v = self.as_f64();
            if exp < 0 {
                return write!(f, "{v:.2}");
            }
            return write!(f, "{}", v.floor() as i64);
        }

        let tier = (exp / 3) as usize;
        if tier < SUFFIXES.len() {
            let scaled = self.mantissa.abs() * 10f64.powi((exp % 3) as i32);
            let sign = if neg { "-" } else { "" };
            let digits = if scaled >= 100.0 {
                0
            } else if scaled >= 10.0 {
                1
            } else {
                2
            };
            return write!(f, "{sign}{scaled:.digits$}{}", SUFFIXES[tier]);
        }

        let sign = if neg { "-" } else { "" };
        write!(f, "{sign}{:.2}e{}", self.mantissa.abs(), exp)
    }
}
