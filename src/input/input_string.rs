use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyType};

use speedate::MicrosecondsPrecisionOverflowBehavior;

use crate::errors::{ErrorType, ErrorTypeDefaults, InputValue, LocItem, ValError, ValResult};
use crate::input::py_string_str;
use crate::tools::safe_repr;
use crate::validators::decimal::create_decimal;

use super::datetime::{
    bytes_as_date, bytes_as_datetime, bytes_as_time, bytes_as_timedelta, EitherDate, EitherDateTime, EitherTime,
};
use super::shared::{map_json_err, str_as_bool, str_as_float, string_to_vec};
use super::{
    EitherBytes, EitherFloat, EitherInt, EitherString, EitherTimedelta, GenericArguments, GenericIterable,
    GenericIterator, GenericMapping, Input, JsonInput,
};

/// Required for JSON Object keys so the string can behave like an Input
impl<'a> Input<'a> for String {
    fn as_loc_item(&self) -> LocItem {
        self.to_string().into()
    }

    fn as_error_value(&'a self) -> InputValue<'a> {
        InputValue::String(self)
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

    fn strict_str(&'a self) -> ValResult<EitherString<'a>> {
        Ok(self.as_str().into())
    }

    fn strict_bytes(&'a self) -> ValResult<EitherBytes<'a>> {
        Ok(self.as_bytes().into())
    }

    fn strict_bool(&self) -> ValResult<bool> {
        str_as_bool(self, self)
    }

    fn strict_int(&'a self) -> ValResult<EitherInt<'a>> {
        match self.parse() {
            Ok(i) => Ok(EitherInt::I64(i)),
            Err(_) => Err(ValError::new(ErrorTypeDefaults::IntParsing, self)),
        }
    }

    fn ultra_strict_float(&'a self) -> ValResult<EitherFloat<'a>> {
        self.strict_float()
    }
    fn strict_float(&'a self) -> ValResult<EitherFloat<'a>> {
        str_as_float(self, self)
    }

    fn strict_decimal(&'a self, decimal_type: &'a PyType) -> ValResult<&'a PyAny> {
        let py = decimal_type.py();
        create_decimal(self.to_object(py).into_ref(py), self, decimal_type)
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_dict(&'a self) -> ValResult<GenericMapping<'a>> {
        Err(ValError::new(ErrorTypeDefaults::DictType, self))
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_list(&'a self) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::ListType, self))
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_tuple(&'a self) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::TupleType, self))
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_set(&'a self) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::SetType, self))
    }

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn strict_frozenset(&'a self) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::FrozenSetType, self))
    }

    fn extract_generic_iterable(&'a self) -> ValResult<GenericIterable<'a>> {
        Ok(GenericIterable::JsonString(self))
    }

    fn validate_iter(&self) -> ValResult<GenericIterator> {
        Ok(string_to_vec(self).into())
    }

    fn strict_date(&self) -> ValResult<EitherDate> {
        bytes_as_date(self, self.as_bytes())
    }

    fn strict_time(
        &self,
        microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherTime> {
        bytes_as_time(self, self.as_bytes(), microseconds_overflow_behavior)
    }

    fn strict_datetime(
        &self,
        microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherDateTime> {
        bytes_as_datetime(self, self.as_bytes(), microseconds_overflow_behavior)
    }

    fn strict_timedelta(
        &self,
        microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherTimedelta> {
        bytes_as_timedelta(self, self.as_bytes(), microseconds_overflow_behavior)
    }
}

#[derive(Debug)]
pub enum StringMapping<'py> {
    String(&'py PyString),
    Mapping(&'py PyDict),
}

impl<'py> ToPyObject for StringMapping<'py> {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        match self {
            Self::String(s) => s.to_object(py),
            Self::Mapping(d) => d.to_object(py),
        }
    }
}

impl<'py> StringMapping<'py> {
    pub fn new_key(py_key: &'py PyAny) -> ValResult<'py, StringMapping> {
        if let Ok(py_str) = py_key.downcast::<PyString>() {
            Ok(Self::String(py_str))
        } else {
            Err(ValError::new(ErrorTypeDefaults::StringType, py_key))
        }
    }

    pub fn new_value(py_value: &'py PyAny) -> ValResult<'py, Self> {
        if let Ok(py_str) = py_value.downcast::<PyString>() {
            Ok(Self::String(py_str))
        } else if let Ok(value) = py_value.downcast::<PyDict>() {
            Ok(Self::Mapping(value))
        } else {
            Err(ValError::new(ErrorTypeDefaults::StringType, py_value))
        }
    }
}

impl<'a> Input<'a> for StringMapping<'a> {
    fn as_loc_item(&self) -> LocItem {
        match self {
            Self::String(s) => s.to_string_lossy().as_ref().into(),
            Self::Mapping(d) => safe_repr(d).to_string().into(),
        }
    }

    fn as_error_value(&'a self) -> InputValue<'a> {
        match self {
            Self::String(s) => s.as_error_value(),
            Self::Mapping(d) => InputValue::PyAny(d),
        }
    }

    fn as_kwargs(&'a self, _py: Python<'a>) -> Option<&'a PyDict> {
        None
    }

    fn validate_args(&'a self) -> ValResult<'a, GenericArguments<'a>> {
        // do we want to support this?
        Err(ValError::new(ErrorTypeDefaults::ArgumentsType, self))
    }

    fn validate_dataclass_args(&'a self, _dataclass_name: &str) -> ValResult<'a, GenericArguments<'a>> {
        self.validate_args()
    }

    fn parse_json(&'a self) -> ValResult<'a, JsonInput> {
        match self {
            Self::String(s) => {
                let str = py_string_str(s)?;
                serde_json::from_str(str).map_err(|e| map_json_err(self, e))
            }
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::JsonType, self)),
        }
    }

    fn strict_str(&'a self) -> ValResult<EitherString<'a>> {
        match self {
            Self::String(s) => Ok((*s).into()),
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::StringType, self)),
        }
    }

    fn strict_bytes(&'a self) -> ValResult<EitherBytes<'a>> {
        match self {
            Self::String(s) => py_string_str(s).map(|b| b.as_bytes().into()),
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::BytesType, self)),
        }
    }

    fn lax_bytes(&'a self) -> ValResult<EitherBytes<'a>> {
        match self {
            Self::String(s) => {
                let str = py_string_str(s)?;
                Ok(str.as_bytes().into())
            }
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::BytesType, self)),
        }
    }

    fn strict_bool(&self) -> ValResult<bool> {
        match self {
            Self::String(s) => str_as_bool(self, py_string_str(s)?),
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::BoolType, self)),
        }
    }

    fn strict_int(&'a self) -> ValResult<EitherInt<'a>> {
        match self {
            Self::String(s) => match py_string_str(s)?.parse() {
                Ok(i) => Ok(EitherInt::I64(i)),
                Err(_) => Err(ValError::new(ErrorTypeDefaults::IntParsing, self)),
            },
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::IntType, self)),
        }
    }

    fn ultra_strict_float(&'a self) -> ValResult<EitherFloat<'a>> {
        self.strict_float()
    }

    fn strict_float(&'a self) -> ValResult<EitherFloat<'a>> {
        match self {
            Self::String(s) => str_as_float(self, py_string_str(s)?),
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::FloatType, self)),
        }
    }

    fn strict_decimal(&'a self, decimal_type: &'a PyType) -> ValResult<&'a PyAny> {
        match self {
            Self::String(s) => create_decimal(s, self, decimal_type),
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::DecimalType, self)),
        }
    }

    fn strict_dict(&'a self) -> ValResult<GenericMapping<'a>> {
        match self {
            Self::String(_) => Err(ValError::new(ErrorTypeDefaults::DictType, self)),
            Self::Mapping(d) => Ok(GenericMapping::StringMapping(d)),
        }
    }

    fn strict_list(&'a self) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::ListType, self))
    }

    fn strict_tuple(&'a self) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::TupleType, self))
    }

    fn strict_set(&'a self) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::SetType, self))
    }

    fn strict_frozenset(&'a self) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::FrozenSetType, self))
    }

    fn extract_generic_iterable(&'a self) -> ValResult<GenericIterable<'a>> {
        Err(ValError::new(ErrorTypeDefaults::IterableType, self))
    }

    fn validate_iter(&self) -> ValResult<GenericIterator> {
        Err(ValError::new(ErrorTypeDefaults::IterableType, self))
    }

    fn strict_date(&self) -> ValResult<EitherDate> {
        match self {
            Self::String(s) => bytes_as_date(self, py_string_str(s)?.as_bytes()),
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::DateType, self)),
        }
    }

    fn strict_time(
        &self,
        microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherTime> {
        match self {
            Self::String(s) => bytes_as_time(self, py_string_str(s)?.as_bytes(), microseconds_overflow_behavior),
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::TimeType, self)),
        }
    }

    fn strict_datetime(
        &self,
        microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherDateTime> {
        match self {
            Self::String(s) => bytes_as_datetime(self, py_string_str(s)?.as_bytes(), microseconds_overflow_behavior),
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::DatetimeType, self)),
        }
    }

    fn strict_timedelta(
        &self,
        microseconds_overflow_behavior: MicrosecondsPrecisionOverflowBehavior,
    ) -> ValResult<EitherTimedelta> {
        match self {
            Self::String(s) => bytes_as_timedelta(self, py_string_str(s)?.as_bytes(), microseconds_overflow_behavior),
            Self::Mapping(_) => Err(ValError::new(ErrorTypeDefaults::TimeDeltaType, self)),
        }
    }
}
