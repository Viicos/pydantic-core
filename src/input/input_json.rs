use std::borrow::Cow;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyType};
use speedate::MicrosecondsPrecisionOverflowBehavior;
use strum::EnumMessage;

use crate::errors::{ErrorType, ErrorTypeDefaults, InputValue, LocItem, ValError, ValResult};
use crate::validators::decimal::create_decimal;

use super::datetime::{
    bytes_as_date, bytes_as_datetime, bytes_as_time, bytes_as_timedelta, float_as_datetime, float_as_duration,
    float_as_time, int_as_datetime, int_as_duration, int_as_time, EitherDate, EitherDateTime, EitherTime,
};
use super::parse_json::JsonArray;
use super::return_enums::ValidationMatch;
use super::shared::{float_as_int, int_as_bool, map_json_err, str_as_bool, str_as_float, str_as_int};
use super::{
    EitherBytes, EitherFloat, EitherInt, EitherString, EitherTimedelta, GenericArguments, GenericIterable,
    GenericIterator, GenericMapping, Input, JsonArgs, JsonInput,
};

impl<'a> Input<'a> for JsonInput {
    /// This is required by since JSON object keys are always strings, I don't think it can be called
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn as_loc_item(&self) -> LocItem {
        match self {
            JsonInput::Int(i) => (*i).into(),
            JsonInput::String(s) => s.as_str().into(),
            v => format!("{v:?}").into(),
        }
    }

    fn as_error_value(&'a self) -> InputValue<'a> {
        InputValue::JsonInput(self)
    }

    fn is_none(&self) -> bool {
        matches!(self, JsonInput::Null)
    }

    fn as_kwargs(&'a self, py: Python<'a>) -> Option<&'a PyDict> {
        match self {
            JsonInput::Object(object) => {
                let dict = PyDict::new(py);
                for (k, v) in object.iter() {
                    dict.set_item(k, v.to_object(py)).unwrap();
                }
                Some(dict)
            }
            _ => None,
        }
    }

    fn validate_args(&'a self) -> ValResult<'a, GenericArguments<'a>> {
        match self {
            JsonInput::Object(object) => Ok(JsonArgs::new(None, Some(object)).into()),
            JsonInput::Array(array) => Ok(JsonArgs::new(Some(array), None).into()),
            _ => Err(ValError::new(ErrorTypeDefaults::ArgumentsType, self)),
        }
    }

    fn validate_dataclass_args(&'a self, class_name: &str) -> ValResult<'a, GenericArguments<'a>> {
        match self {
            JsonInput::Object(object) => Ok(JsonArgs::new(None, Some(object)).into()),
            _ => {
                let class_name = class_name.to_string();
                Err(ValError::new(
                    ErrorType::DataclassType {
                        class_name,
                        context: None,
                    },
                    self,
                ))
            }
        }
    }

    fn parse_json(&'a self) -> ValResult<'a, JsonInput> {
        match self {
            JsonInput::String(s) => serde_json::from_str(s.as_str()).map_err(|e| map_json_err(self, e)),
            _ => Err(ValError::new(ErrorTypeDefaults::JsonType, self)),
        }
    }

    fn exact_str(&'a self) -> ValResult<EitherString<'a>> {
        match self {
            // Justification for `strict` instead of `exact` is that in JSON strings can also
            // represent other datatypes such as UUID and date more exactly, so string is a
            // converting input
            JsonInput::String(s) => Ok(s.as_str().into()),
            _ => Err(ValError::new(ErrorTypeDefaults::StringType, self)),
        }
    }

    fn validate_str(&'a self, _strict: bool) -> ValResult<ValidationMatch<EitherString<'a>>> {
        // Justification for `strict` instead of `exact` is that in JSON strings can also
        // represent other datatypes such as UUID and date more exactly, so string is a
        // converting input
        self.exact_str().map(ValidationMatch::strict)
    }

    fn validate_bytes(&'a self, _strict: bool) -> ValResult<EitherBytes<'a>> {
        match self {
            JsonInput::String(s) => Ok(s.as_bytes().into()),
            _ => Err(ValError::new(ErrorTypeDefaults::BytesType, self)),
        }
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_bytes(&'a self) -> ValResult<EitherBytes<'a>> {
        self.validate_bytes(false)
    }

    fn validate_bool(&self, strict: bool) -> ValResult<'_, ValidationMatch<bool>> {
        match self {
            JsonInput::Bool(b) => Ok(ValidationMatch::exact(*b)),
            JsonInput::String(s) if !strict => str_as_bool(self, s).map(ValidationMatch::lax),
            JsonInput::Int(int) if !strict => int_as_bool(self, *int).map(ValidationMatch::lax),
            JsonInput::Float(float) if !strict => match float_as_int(self, *float) {
                Ok(int) => int
                    .as_bool()
                    .ok_or_else(|| ValError::new(ErrorTypeDefaults::BoolParsing, self))
                    .map(ValidationMatch::lax),
                _ => Err(ValError::new(ErrorTypeDefaults::BoolType, self)),
            },
            _ => Err(ValError::new(ErrorTypeDefaults::BoolType, self)),
        }
    }

    fn validate_int(&'a self, strict: bool) -> ValResult<'a, ValidationMatch<EitherInt<'a>>> {
        match self {
            JsonInput::Int(i) => Ok(ValidationMatch::exact(EitherInt::I64(*i))),
            JsonInput::Uint(u) => Ok(ValidationMatch::exact(EitherInt::U64(*u))),
            JsonInput::BigInt(b) => Ok(ValidationMatch::exact(EitherInt::BigInt(b.clone()))),
            JsonInput::Bool(b) if !strict => Ok(ValidationMatch::lax(EitherInt::I64((*b).into()))),
            JsonInput::Float(f) if !strict => float_as_int(self, *f).map(ValidationMatch::lax),
            JsonInput::String(str) if !strict => str_as_int(self, str).map(ValidationMatch::lax),
            _ => Err(ValError::new(ErrorTypeDefaults::IntType, self)),
        }
    }

    fn validate_float(&'a self, strict: bool) -> ValResult<'a, ValidationMatch<EitherFloat<'a>>> {
        match self {
            JsonInput::Float(f) => Ok(ValidationMatch::exact(EitherFloat::F64(*f))),
            JsonInput::Int(i) => Ok(ValidationMatch::strict(EitherFloat::F64(*i as f64))),
            JsonInput::Uint(u) => Ok(ValidationMatch::strict(EitherFloat::F64(*u as f64))),
            JsonInput::Bool(b) if !strict => Ok(ValidationMatch::lax(EitherFloat::F64(if *b { 1.0 } else { 0.0 }))),
            JsonInput::String(str) if !strict => str_as_float(self, str).map(ValidationMatch::lax),
            _ => Err(ValError::new(ErrorTypeDefaults::FloatType, self)),
        }
    }

    fn strict_decimal(&'a self, decimal_type: &'a PyType) -> ValResult<&'a PyAny> {
        let py = decimal_type.py();
        match self {
            JsonInput::Float(f) => create_decimal(PyString::new(py, &f.to_string()), self, decimal_type),

            JsonInput::String(..) | JsonInput::Int(..) | JsonInput::Uint(..) | JsonInput::BigInt(..) => {
                create_decimal(self.to_object(py).into_ref(py), self, decimal_type)
            }
            _ => Err(ValError::new(ErrorTypeDefaults::DecimalType, self)),
        }
    }

    fn validate_dict(&'a self, _strict: bool) -> ValResult<GenericMapping<'a>> {
        match self {
            JsonInput::Object(dict) => Ok(dict.into()),
            _ => Err(ValError::new(ErrorTypeDefaults::DictType, self)),
        }
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_dict(&'a self) -> ValResult<GenericMapping<'a>> {
        self.validate_dict(false)
    }

    fn validate_list(&'a self, _strict: bool) -> ValResult<GenericIterable<'a>> {
        match self {
            JsonInput::Array(a) => Ok(GenericIterable::JsonArray(a)),
            _ => Err(ValError::new(ErrorTypeDefaults::ListType, self)),
        }
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_list(&'a self) -> ValResult<GenericIterable<'a>> {
        self.validate_list(false)
    }

    fn validate_tuple(&'a self, _strict: bool) -> ValResult<GenericIterable<'a>> {
        // just as in set's case, List has to be allowed
        match self {
            JsonInput::Array(a) => Ok(GenericIterable::JsonArray(a)),
            _ => Err(ValError::new(ErrorTypeDefaults::TupleType, self)),
        }
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_tuple(&'a self) -> ValResult<GenericIterable<'a>> {
        self.validate_tuple(false)
    }

    fn validate_set(&'a self, _strict: bool) -> ValResult<GenericIterable<'a>> {
        // we allow a list here since otherwise it would be impossible to create a set from JSON
        match self {
            JsonInput::Array(a) => Ok(GenericIterable::JsonArray(a)),
            _ => Err(ValError::new(ErrorTypeDefaults::SetType, self)),
        }
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_set(&'a self) -> ValResult<GenericIterable<'a>> {
        self.validate_set(false)
    }

    fn validate_frozenset(&'a self, _strict: bool) -> ValResult<GenericIterable<'a>> {
        // we allow a list here since otherwise it would be impossible to create a frozenset from JSON
        match self {
            JsonInput::Array(a) => Ok(GenericIterable::JsonArray(a)),
            _ => Err(ValError::new(ErrorTypeDefaults::FrozenSetType, self)),
        }
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_frozenset(&'a self) -> ValResult<GenericIterable<'a>> {
        self.validate_frozenset(false)
    }

    fn extract_generic_iterable(&self) -> ValResult<GenericIterable> {
        match self {
            JsonInput::Array(a) => Ok(GenericIterable::JsonArray(a)),
            JsonInput::String(s) => Ok(GenericIterable::JsonString(s)),
            JsonInput::Object(object) => Ok(GenericIterable::JsonObject(object)),
            _ => Err(ValError::new(ErrorTypeDefaults::IterableType, self)),
        }
    }

    fn validate_iter(&self) -> ValResult<GenericIterator> {
        match self {
            JsonInput::Array(a) => Ok(a.clone().into()),
            JsonInput::String(s) => Ok(string_to_vec(s).into()),
            JsonInput::Object(object) => {
                // return keys iterator to match python's behavior
                let keys: Vec<JsonInput> = object.keys().map(|k| JsonInput::String(k.clone())).collect();
                Ok(keys.into())
            }
            _ => Err(ValError::new(ErrorTypeDefaults::IterableType, self)),
        }
    }

    fn validate_date(&self, _strict: bool) -> ValResult<EitherDate> {
        match self {
            JsonInput::String(v) => bytes_as_date(self, v.as_bytes()),
            _ => Err(ValError::new(ErrorTypeDefaults::DateType, self)),
        }
    }
    // NO custom `lax_date` implementation, if strict_date fails, the validator will fallback to lax_datetime
    // then check there's no remainder
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_date(&self) -> ValResult<EitherDate> {
        self.validate_date(false)
    }

    fn strict_time(
        &self,
        microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherTime> {
        match self {
            JsonInput::String(v) => bytes_as_time(self, v.as_bytes(), microseconds_overflow_behavior),
            _ => Err(ValError::new(ErrorTypeDefaults::TimeType, self)),
        }
    }
    fn lax_time(&self, microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior) -> ValResult<EitherTime> {
        match self {
            JsonInput::String(v) => bytes_as_time(self, v.as_bytes(), microseconds_overflow_behavior),
            JsonInput::Int(v) => int_as_time(self, *v, 0),
            JsonInput::Float(v) => float_as_time(self, *v),
            JsonInput::BigInt(_) => Err(ValError::new(
                ErrorType::TimeParsing {
                    error: Cow::Borrowed(
                        speedate::ParseError::TimeTooLarge
                            .get_documentation()
                            .unwrap_or_default(),
                    ),
                    context: None,
                },
                self,
            )),
            _ => Err(ValError::new(ErrorTypeDefaults::TimeType, self)),
        }
    }

    fn strict_datetime(
        &self,
        microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherDateTime> {
        match self {
            JsonInput::String(v) => bytes_as_datetime(self, v.as_bytes(), microseconds_overflow_behavior),
            _ => Err(ValError::new(ErrorTypeDefaults::DatetimeType, self)),
        }
    }
    fn lax_datetime(
        &self,
        microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherDateTime> {
        match self {
            JsonInput::String(v) => bytes_as_datetime(self, v.as_bytes(), microseconds_overflow_behavior),
            JsonInput::Int(v) => int_as_datetime(self, *v, 0),
            JsonInput::Float(v) => float_as_datetime(self, *v),
            _ => Err(ValError::new(ErrorTypeDefaults::DatetimeType, self)),
        }
    }

    fn strict_timedelta(
        &self,
        microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherTimedelta> {
        match self {
            JsonInput::String(v) => bytes_as_timedelta(self, v.as_bytes(), microseconds_overflow_behavior),
            _ => Err(ValError::new(ErrorTypeDefaults::TimeDeltaType, self)),
        }
    }
    fn lax_timedelta(
        &self,
        microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherTimedelta> {
        match self {
            JsonInput::String(v) => bytes_as_timedelta(self, v.as_bytes(), microseconds_overflow_behavior),
            JsonInput::Int(v) => Ok(int_as_duration(self, *v)?.into()),
            JsonInput::Float(v) => Ok(float_as_duration(self, *v)?.into()),
            _ => Err(ValError::new(ErrorTypeDefaults::TimeDeltaType, self)),
        }
    }
}

/// Required for Dict keys so the string can behave like an Input
impl<'a> Input<'a> for String {
    fn as_loc_item(&self) -> LocItem {
        self.to_string().into()
    }

    fn as_error_value(&'a self) -> InputValue<'a> {
        InputValue::String(self)
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn is_none(&self) -> bool {
        false
    }

    fn as_kwargs(&'a self, _py: Python<'a>) -> Option<&'a PyDict> {
        None
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn validate_args(&'a self) -> ValResult<'a, GenericArguments<'a>> {
        Err(ValError::new(ErrorTypeDefaults::ArgumentsType, self))
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn validate_dataclass_args(&'a self, class_name: &str) -> ValResult<'a, GenericArguments<'a>> {
        let class_name = class_name.to_string();
        Err(ValError::new(
            ErrorType::DataclassType {
                class_name,
                context: None,
            },
            self,
        ))
    }

    fn parse_json(&'a self) -> ValResult<'a, JsonInput> {
        serde_json::from_str(self.as_str()).map_err(|e| map_json_err(self, e))
    }

    fn validate_str(&'a self, _strict: bool) -> ValResult<ValidationMatch<EitherString<'a>>> {
        Ok(ValidationMatch::exact(self.as_str().into()))
    }

    fn validate_bytes(&'a self, _strict: bool) -> ValResult<EitherBytes<'a>> {
        Ok(self.as_bytes().into())
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_bytes(&'a self) -> ValResult<EitherBytes<'a>> {
        self.validate_bytes(false)
    }

    fn validate_bool(&self, strict: bool) -> ValResult<'_, ValidationMatch<bool>> {
        if strict {
            Err(ValError::new(ErrorTypeDefaults::BoolType, self))
        } else {
            str_as_bool(self, self).map(ValidationMatch::lax)
        }
    }

    fn validate_int(&'a self, strict: bool) -> ValResult<'a, ValidationMatch<EitherInt<'a>>> {
        if strict {
            Err(ValError::new(ErrorTypeDefaults::IntType, self))
        } else {
            match self.parse() {
                Ok(i) => Ok(ValidationMatch::lax(EitherInt::I64(i))),
                Err(_) => Err(ValError::new(ErrorTypeDefaults::IntParsing, self)),
            }
        }
    }

    fn validate_float(&'a self, strict: bool) -> ValResult<'a, ValidationMatch<EitherFloat<'a>>> {
        if strict {
            Err(ValError::new(ErrorTypeDefaults::FloatType, self))
        } else {
            str_as_float(self, self).map(ValidationMatch::lax)
        }
    }

    fn strict_decimal(&'a self, decimal_type: &'a PyType) -> ValResult<&'a PyAny> {
        let py = decimal_type.py();
        create_decimal(self.to_object(py).into_ref(py), self, decimal_type)
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn validate_dict(&'a self, _strict: bool) -> ValResult<GenericMapping<'a>> {
        Err(ValError::new(ErrorTypeDefaults::DictType, self))
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_dict(&'a self) -> ValResult<GenericMapping<'a>> {
        self.validate_dict(false)
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn validate_list(&'a self, _strict: bool) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::ListType, self))
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_list(&'a self) -> ValResult<GenericIterable<'a>> {
        self.validate_list(false)
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn validate_tuple(&'a self, _strict: bool) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::TupleType, self))
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_tuple(&'a self) -> ValResult<GenericIterable<'a>> {
        self.validate_tuple(false)
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn validate_set(&'a self, _strict: bool) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::SetType, self))
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_set(&'a self) -> ValResult<GenericIterable<'a>> {
        self.validate_set(false)
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn validate_frozenset(&'a self, _strict: bool) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::FrozenSetType, self))
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_frozenset(&'a self) -> ValResult<GenericIterable<'a>> {
        self.validate_frozenset(false)
    }

    fn extract_generic_iterable(&'a self) -> ValResult<GenericIterable<'a>> {
        Ok(GenericIterable::JsonString(self))
    }

    fn validate_iter(&self) -> ValResult<GenericIterator> {
        Ok(string_to_vec(self).into())
    }

    fn validate_date(&self, _strict: bool) -> ValResult<EitherDate> {
        bytes_as_date(self, self.as_bytes())
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_date(&self) -> ValResult<EitherDate> {
        self.validate_date(false)
    }

    fn validate_time(
        &self,
        _strict: bool,
        microseconds_overflow_behavior: speedate::MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherTime> {
        bytes_as_time(self, self.as_bytes(), microseconds_overflow_behavior)
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_time(
        &self,
        microseconds_overflow_behavior: speedate::MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherTime> {
        self.validate_time(false, microseconds_overflow_behavior)
    }

    fn validate_datetime(
        &self,
        _strict: bool,
        microseconds_overflow_behavior: speedate::MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherDateTime> {
        bytes_as_datetime(self, self.as_bytes(), microseconds_overflow_behavior)
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_datetime(
        &self,
        microseconds_overflow_behavior: speedate::MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherDateTime> {
        self.validate_datetime(false, microseconds_overflow_behavior)
    }

    fn validate_timedelta(
        &self,
        _strict: bool,
        microseconds_overflow_behavior: speedate::MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherTimedelta> {
        bytes_as_timedelta(self, self.as_bytes(), microseconds_overflow_behavior)
    }
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_timedelta(
        &self,
        microseconds_overflow_behavior: speedate::MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherTimedelta> {
        self.validate_timedelta(false, microseconds_overflow_behavior)
    }
}

fn string_to_vec(s: &str) -> JsonArray {
    s.chars().map(|c| JsonInput::String(c.to_string())).collect()
}
