//! Arbitrary JSON (Fork)
//! =====================
//! This is a fork of [arbitrary-json](https://raw.githubusercontent.com/irevoire/arbitrary-json/refs/heads/main/src/lib.rs).
//!
//! Originally licensed WTFPL

use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use arbitrary::{Arbitrary, Error, Result, Unstructured, size_hint};
use serde_json::{Map, Number, Value};

/// [`serde_json::Value`] wrapper implementing [`Arbitrary`].
#[derive(Clone)]
pub struct ArbitraryValue(Value);

#[derive(Clone, Copy, PartialEq, Eq, Arbitrary)]
enum ValueVariant {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

impl<'a> Arbitrary<'a> for ArbitraryValue {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let variant: ValueVariant = u.arbitrary()?;

        let variant = match variant {
            ValueVariant::Null => Value::Null,
            ValueVariant::Bool => Value::Bool(u.arbitrary().unwrap_or(false)),
            ValueVariant::Number => Value::Number(ArbitraryNumber::arbitrary(u)?.0),
            ValueVariant::String => Value::String(u.arbitrary()?),
            ValueVariant::Array => Value::Array(u.arbitrary::<ArbitraryArray>()?.into()),
            ValueVariant::Object => Value::Object(u.arbitrary::<ArbitraryObject>()?.into()),
        };

        Ok(ArbitraryValue(variant))
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        // String, array, and object are unbounded variants
        (1, None)
    }
}

/// [`serde_json::value::Number`] implementing [`Arbitrary`].
#[derive(Clone)]
pub struct ArbitraryNumber(Number);

#[derive(Clone, Copy, PartialEq, Eq, Arbitrary)]
enum NumberVariant {
    Float,
    Unsigned,
    Signed,
}

impl<'a> Arbitrary<'a> for ArbitraryNumber {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let variant = u.arbitrary().unwrap_or(NumberVariant::Float);

        let num = match variant {
            NumberVariant::Float => {
                let float: f64 = u.arbitrary().unwrap_or_default();
                Number::from_f64(float).ok_or(Error::IncorrectFormat)?
            }
            NumberVariant::Unsigned => {
                let unsigned: u64 = u.arbitrary().unwrap_or_default();
                Number::from(unsigned)
            }
            NumberVariant::Signed => {
                let signed: i64 = u.arbitrary().unwrap_or_default();
                Number::from(signed)
            }
        };

        Ok(Self(num))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        let variant_hint = <NumberVariant as Arbitrary>::size_hint(depth);

        let float_itself = <f64 as Arbitrary>::size_hint(depth);
        let unsigned_itself = <u64 as Arbitrary>::size_hint(depth);
        let signed_itself = <i64 as Arbitrary>::size_hint(depth);

        let [float_total, unsigned_total, signed_total] =
            [float_itself, unsigned_itself, signed_itself]
                .map(|itself| size_hint::and(itself, variant_hint));

        size_hint::or_all(&[float_total, unsigned_total, signed_total])
    }
}

#[derive(Clone)]
pub struct ArbitraryObject(Map<String, Value>);

impl<'a> Arbitrary<'a> for ArbitraryObject {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let map = u
            .arbitrary_iter()?
            .map(|result| result.map(|(key, value): (String, ArbitraryValue)| (key, value.0)))
            .collect::<Result<Map<String, Value>>>()?;

        Ok(ArbitraryObject(map))
    }
}

#[derive(Clone)]
pub struct ArbitraryArray(Vec<Value>);

impl<'a> Arbitrary<'a> for ArbitraryArray {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let array = u
            .arbitrary_iter()?
            .map(|result| result.map(|json: ArbitraryValue| json.0))
            .collect::<Result<Vec<Value>>>()?;

        Ok(ArbitraryArray(array))
    }
}

macro_rules! impl_derefrom {
    ($arbitrary:ty, $serde:ty) => {
        impl Deref for $arbitrary {
            type Target = $serde;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $arbitrary {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl From<$serde> for $arbitrary {
            fn from(value: $serde) -> Self {
                Self(value)
            }
        }

        impl From<$arbitrary> for $serde {
            fn from(value: $arbitrary) -> Self {
                value.0
            }
        }

        impl Debug for $arbitrary {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

impl_derefrom!(ArbitraryValue, Value);
impl_derefrom!(ArbitraryNumber, Number);
impl_derefrom!(ArbitraryObject, Map<String, Value>);
impl_derefrom!(ArbitraryArray, Vec<Value>);
