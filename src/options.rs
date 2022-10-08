use std::collections::HashMap;

use crate::util::Error;

pub type Options = HashMap<String, Value>;

pub fn from_yaml(options: serde_yaml::Mapping) -> Result<Options, Error> {
    options
        .into_iter()
        .map(|(key, value)| {
            let key = key.as_str().ok_or(Error::InvalidOptions)?.to_string();
            let value = from_yaml_value(value)?;
            Ok((key, value))
        })
        .collect()
}

fn from_yaml_value(value: serde_yaml::Value) -> Result<Value, Error> {
    match value {
        serde_yaml::Value::Null => Err(Error::InvalidOptions),
        serde_yaml::Value::Bool(value) => Ok(Value::Bool(value)),
        serde_yaml::Value::Number(value) => value
            .as_f64()
            .ok_or(Error::InvalidOptions)
            .map(|value| Value::Number(value as f32)),
        serde_yaml::Value::String(value) => Ok(Value::String(value)),
        serde_yaml::Value::Sequence(value) => Ok(Value::Sequence(
            value
                .into_iter()
                .map(from_yaml_value)
                .collect::<Result<_, _>>()?,
        )),
        serde_yaml::Value::Mapping(value) => Ok(Value::Mapping(
            value
                .into_iter()
                .map(|(key, value)| {
                    let key = key.as_str().ok_or(Error::InvalidOptions)?.to_string();
                    let value = from_yaml_value(value)?;
                    Ok((key, value))
                })
                .collect::<Result<_, _>>()?,
        )),
        serde_yaml::Value::Tagged(_) => Err(Error::InvalidOptions),
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Number(f32),
    String(String),
    Sequence(Vec<Value>),
    Mapping(HashMap<String, Value>),
}

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Value::Number(value) => {
                if value.fract() == 0.0 {
                    Some(*value as i32)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_slice(&self) -> Option<&[Value]> {
        match self {
            Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                Some(std::slice::from_ref(self))
            }
            Value::Sequence(value) => Some(value.as_slice()),
            Value::Mapping(_) => None,
        }
    }

    pub fn get<K: AsRef<str>>(&self, key: &K) -> Option<&Value> {
        match self {
            Value::Mapping(value) => value.get(key.as_ref()),
            _ => None,
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Number(value as f32)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Number(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl<T1, T2> From<(T1, T2)> for Value
where
    Value: From<T1>,
    Value: From<T2>,
{
    fn from((value1, value2): (T1, T2)) -> Self {
        Value::Sequence(vec![value1.into(), value2.into()])
    }
}
