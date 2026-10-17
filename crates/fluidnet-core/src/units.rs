//! Quantities with units in model files, and time schedules.
//!
//! A parameter may be a bare number (SI), a string with a unit ("300 bar",
//! "20 degC", "6 mm"), or, where time-varying values are allowed, a table
//! `[[t, value], ...]` with t in seconds (or a unit string).

use serde_json::Value;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dim {
    Pressure,
    Temperature,
    Length,
    Area,
    Volume,
    MassFlow,
    Time,
    Power,
    HeatFlux,
    None,
}

impl Dim {
    pub fn si_unit(self) -> &'static str {
        match self {
            Dim::Pressure => "Pa",
            Dim::Temperature => "K",
            Dim::Length => "m",
            Dim::Area => "m2",
            Dim::Volume => "m3",
            Dim::MassFlow => "kg/s",
            Dim::Time => "s",
            Dim::Power => "W",
            Dim::HeatFlux => "W/m2",
            Dim::None => "-",
        }
    }
}

/// (dimension, factor, offset): SI = value * factor + offset
fn unit_info(u: &str) -> Option<(Dim, f64, f64)> {
    use Dim::*;
    let u = u.trim();
    Some(match u {
        "Pa" | "pa" => (Pressure, 1.0, 0.0),
        "kPa" => (Pressure, 1e3, 0.0),
        "MPa" => (Pressure, 1e6, 0.0),
        "bar" | "bara" => (Pressure, 1e5, 0.0),
        "mbar" => (Pressure, 1e2, 0.0),
        "psi" | "psia" => (Pressure, 6894.757293168, 0.0),
        "atm" => (Pressure, 101325.0, 0.0),
        "K" => (Temperature, 1.0, 0.0),
        "degC" | "C" | "°C" | "degc" => (Temperature, 1.0, 273.15),
        "degF" | "F" | "°F" => (Temperature, 5.0 / 9.0, 273.15 - 32.0 * 5.0 / 9.0),
        "R" | "degR" => (Temperature, 5.0 / 9.0, 0.0),
        "m" => (Length, 1.0, 0.0),
        "cm" => (Length, 1e-2, 0.0),
        "mm" => (Length, 1e-3, 0.0),
        "um" | "µm" => (Length, 1e-6, 0.0),
        "in" => (Length, 0.0254, 0.0),
        "ft" => (Length, 0.3048, 0.0),
        "m2" | "m^2" => (Area, 1.0, 0.0),
        "cm2" | "cm^2" => (Area, 1e-4, 0.0),
        "mm2" | "mm^2" => (Area, 1e-6, 0.0),
        "in2" | "in^2" => (Area, 6.4516e-4, 0.0),
        "m3" | "m^3" => (Volume, 1.0, 0.0),
        "L" | "l" | "liter" | "litre" => (Volume, 1e-3, 0.0),
        "mL" | "ml" | "cm3" | "cc" => (Volume, 1e-6, 0.0),
        "mm3" => (Volume, 1e-9, 0.0),
        "in3" => (Volume, 1.6387064e-5, 0.0),
        "kg/s" => (MassFlow, 1.0, 0.0),
        "g/s" => (MassFlow, 1e-3, 0.0),
        "kg/h" => (MassFlow, 1.0 / 3600.0, 0.0),
        "kg/min" => (MassFlow, 1.0 / 60.0, 0.0),
        "lb/s" | "lbm/s" => (MassFlow, 0.45359237, 0.0),
        "s" => (Time, 1.0, 0.0),
        "ms" => (Time, 1e-3, 0.0),
        "us" | "µs" => (Time, 1e-6, 0.0),
        "min" => (Time, 60.0, 0.0),
        "h" | "hr" => (Time, 3600.0, 0.0),
        "W" => (Power, 1.0, 0.0),
        "kW" => (Power, 1e3, 0.0),
        "MW" => (Power, 1e6, 0.0),
        "W/m2" => (HeatFlux, 1.0, 0.0),
        "kW/m2" => (HeatFlux, 1e3, 0.0),
        "MW/m2" => (HeatFlux, 1e6, 0.0),
        "" | "-" | "1" => (None, 1.0, 0.0),
        "%" => (None, 0.01, 0.0),
        _ => return Option::None,
    })
}

/// Parse a quantity string like "300 bar" or "-12.5degC" into SI.
pub fn parse_str(s: &str, dim: Dim) -> Result<f64> {
    let s = s.trim();
    let split = s
        .char_indices()
        .find(|&(i, c)| {
            !(c.is_ascii_digit()
                || c == '.'
                || c == '+'
                || c == '-' && (i == 0 || matches!(s.as_bytes()[i - 1], b'e' | b'E'))
                || (c == 'e' || c == 'E')
                    && i > 0
                    && s.as_bytes()[i - 1].is_ascii_digit()
                    && s[i + 1..].starts_with(|n: char| n.is_ascii_digit() || n == '-' || n == '+'))
        })
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    let val: f64 = num
        .trim()
        .parse()
        .map_err(|_| Error::Parse(format!("cannot read number in '{s}'")))?;
    let unit = unit.trim();
    if unit.is_empty() {
        return Ok(val);
    }
    let (d, f, o) =
        unit_info(unit).ok_or_else(|| Error::Parse(format!("unknown unit '{unit}' in '{s}'")))?;
    if d != dim && dim != Dim::None {
        return Err(Error::Parse(format!(
            "'{s}': expected a {:?} quantity ({}), got {:?}",
            dim,
            dim.si_unit(),
            d
        )));
    }
    Ok(val * f + o)
}

pub fn parse_value(v: &Value, dim: Dim) -> Result<f64> {
    match v {
        Value::Number(n) => n.as_f64().ok_or_else(|| Error::Parse("bad number".into())),
        Value::String(s) => parse_str(s, dim),
        _ => Err(Error::Parse(format!(
            "expected a {:?} quantity, got {v}",
            dim
        ))),
    }
}

/// Constant or piecewise-linear time function.
#[derive(Debug, Clone)]
pub enum Schedule {
    Const(f64),
    Table {
        t: Vec<f64>,
        v: Vec<f64>,
        step: bool,
    },
}

impl Schedule {
    pub fn parse(v: &Value, dim: Dim) -> Result<Schedule> {
        match v {
            Value::Array(rows) => Self::parse_rows(rows, dim, false),
            Value::Object(m) => {
                let rows = m
                    .get("table")
                    .and_then(|r| r.as_array())
                    .ok_or_else(|| Error::Parse("schedule object needs a 'table' array".into()))?;
                let step = m.get("interp").and_then(|s| s.as_str()) == Some("step");
                Self::parse_rows(rows, dim, step)
            }
            _ => Ok(Schedule::Const(parse_value(v, dim)?)),
        }
    }

    fn parse_rows(rows: &[Value], dim: Dim, step: bool) -> Result<Schedule> {
        let mut t = Vec::new();
        let mut v = Vec::new();
        for r in rows {
            let pair = r
                .as_array()
                .filter(|a| a.len() == 2)
                .ok_or_else(|| Error::Parse("schedule rows must be [t, value]".into()))?;
            t.push(parse_value(&pair[0], Dim::Time)?);
            v.push(parse_value(&pair[1], dim)?);
        }
        if t.is_empty() {
            return Err(Error::Parse("empty schedule table".into()));
        }
        if t.windows(2).any(|w| w[1] < w[0]) {
            return Err(Error::Parse("schedule times must be non-decreasing".into()));
        }
        if t.len() == 1 {
            return Ok(Schedule::Const(v[0]));
        }
        Ok(Schedule::Table { t, v, step })
    }

    #[inline]
    pub fn at(&self, time: f64) -> f64 {
        match self {
            Schedule::Const(c) => *c,
            Schedule::Table { t, v, step } => {
                let n = t.len();
                if time <= t[0] {
                    return v[0];
                }
                if time >= t[n - 1] {
                    return v[n - 1];
                }
                let i = t.partition_point(|&x| x <= time) - 1;
                if *step {
                    return v[i];
                }
                let dt = t[i + 1] - t[i];
                if dt <= 0.0 {
                    return v[i + 1];
                }
                v[i] + (time - t[i]) / dt * (v[i + 1] - v[i])
            }
        }
    }

    pub fn is_const(&self) -> bool {
        matches!(self, Schedule::Const(_))
    }

    pub fn breakpoints(&self) -> Vec<f64> {
        match self {
            Schedule::Const(_) => vec![],
            Schedule::Table { t, .. } => t.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn units() {
        assert!((parse_str("300 bar", Dim::Pressure).unwrap() - 3e7).abs() < 1e-6);
        assert!((parse_str("20 degC", Dim::Temperature).unwrap() - 293.15).abs() < 1e-9);
        assert!((parse_str("1.5e-3m", Dim::Length).unwrap() - 1.5e-3).abs() < 1e-12);
        assert!((parse_str("-1.5e-3 m", Dim::Length).unwrap() + 1.5e-3).abs() < 1e-12);
        assert!((parse_str("6 mm", Dim::Length).unwrap() - 6e-3).abs() < 1e-12);
        assert!(parse_str("6 bar", Dim::Length).is_err());
        assert!((parse_str("50 L", Dim::Volume).unwrap() - 0.05).abs() < 1e-12);
    }
}
