use std::fmt;

use pyo3::types::{PyDict, PyString, PyType};
use pyo3::{intern, prelude::*};

use crate::errors::{InputValue, LocItem, ValResult};
use crate::{PyMultiHostUrl, PyUrl};

use super::datetime::{EitherDate, EitherDateTime, EitherTime, EitherTimedelta};
use super::generic_iterable::GenericIterable;
use super::return_enums::{EitherBytes, EitherString};
use super::{GenericArguments, GenericCollection, GenericIterator, GenericMapping, JsonInput};

#[derive(Debug, Clone, Copy)]
pub enum InputType {
    Python,
    Json,
}

impl IntoPy<PyObject> for InputType {
    fn into_py(self, py: Python<'_>) -> PyObject {
        match self {
            Self::Json => intern!(py, "json").into(),
            Self::Python => intern!(py, "python").into(),
        }
    }
}

/// all types have three methods: `validate_*`, `strict_*`, `lax_*`
/// the convention is to either implement:
/// * `strict_*` & `lax_*` if they have different behavior
/// * or, `validate_*` and `strict_*` to just call `validate_*` if the behavior for strict and lax is the same
pub trait Input<'a>: fmt::Debug + ToPyObject {
    fn as_loc_item(&self) -> LocItem;

    fn as_error_value(&'a self) -> InputValue<'a>;

    fn identity(&self) -> Option<usize> {
        None
    }

    fn is_none(&self) -> bool;

    #[cfg_attr(has_no_coverage, no_coverage)]
    fn input_get_attr(&self, _name: &PyString) -> Option<PyResult<&PyAny>> {
        None
    }

    fn is_exact_instance(&self, _class: &PyType) -> bool {
        false
    }

    fn is_python(&self) -> bool {
        false
    }

    fn as_kwargs(&'a self, py: Python<'a>) -> Option<&'a PyDict>;

    fn input_is_subclass(&self, _class: &PyType) -> PyResult<bool> {
        Ok(false)
    }

    fn input_as_url(&self) -> Option<PyUrl> {
        None
    }

    fn input_as_multi_host_url(&self) -> Option<PyMultiHostUrl> {
        None
    }

    fn callable(&self) -> bool {
        false
    }

    fn validate_args(&'a self) -> ValResult<'a, GenericArguments<'a>>;

    fn validate_dataclass_args(&'a self, dataclass_name: &str) -> ValResult<'a, GenericArguments<'a>>;

    fn parse_json(&'a self) -> ValResult<'a, JsonInput>;

    fn validate_str(&'a self, strict: bool) -> ValResult<EitherString<'a>> {
        if strict {
            self.strict_str()
        } else {
            self.lax_str()
        }
    }
    fn strict_str(&'a self) -> ValResult<EitherString<'a>>;
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn lax_str(&'a self) -> ValResult<EitherString<'a>> {
        self.strict_str()
    }

    fn as_str_strict(&self) -> Option<&str>;

    fn validate_bytes(&'a self, strict: bool) -> ValResult<EitherBytes<'a>> {
        if strict {
            self.strict_bytes()
        } else {
            self.lax_bytes()
        }
    }
    fn strict_bytes(&'a self) -> ValResult<EitherBytes<'a>>;
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn lax_bytes(&'a self) -> ValResult<EitherBytes<'a>> {
        self.strict_bytes()
    }

    fn validate_bool(&self, strict: bool) -> ValResult<bool> {
        if strict {
            self.strict_bool()
        } else {
            self.lax_bool()
        }
    }
    fn strict_bool(&self) -> ValResult<bool>;
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn lax_bool(&self) -> ValResult<bool> {
        self.strict_bool()
    }

    fn validate_int(&self, strict: bool) -> ValResult<i64> {
        if strict {
            self.strict_int()
        } else {
            self.lax_int()
        }
    }
    fn strict_int(&self) -> ValResult<i64>;
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn lax_int(&self) -> ValResult<i64> {
        self.strict_int()
    }

    fn as_int_strict(&self) -> Option<i64>;

    fn validate_float(&self, strict: bool, ultra_strict: bool) -> ValResult<f64> {
        if ultra_strict {
            self.ultra_strict_float()
        } else if strict {
            self.strict_float()
        } else {
            self.lax_float()
        }
    }
    fn ultra_strict_float(&self) -> ValResult<f64>;
    fn strict_float(&self) -> ValResult<f64>;
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn lax_float(&self) -> ValResult<f64> {
        self.strict_float()
    }

    fn validate_dict(&'a self, strict: bool) -> ValResult<GenericMapping<'a>> {
        if strict {
            self.strict_dict()
        } else {
            self.lax_dict()
        }
    }
    fn strict_dict(&'a self) -> ValResult<GenericMapping<'a>>;
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn lax_dict(&'a self) -> ValResult<GenericMapping<'a>> {
        self.strict_dict()
    }

    fn validate_model_fields(&'a self, strict: bool, _from_attributes: bool) -> ValResult<GenericMapping<'a>> {
        self.validate_dict(strict)
    }

    fn extract_iterable(&'a self) -> ValResult<GenericIterable<'a>>;

    fn validate_tuple(&'a self, strict: bool) -> ValResult<GenericCollection<'a>> {
        if strict {
            self.strict_tuple()
        } else {
            self.lax_tuple()
        }
    }
    fn strict_tuple(&'a self) -> ValResult<GenericCollection<'a>>;
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn lax_tuple(&'a self) -> ValResult<GenericCollection<'a>> {
        self.strict_tuple()
    }

    fn validate_iter(&self) -> ValResult<GenericIterator>;

    fn validate_date(&self, strict: bool) -> ValResult<EitherDate> {
        if strict {
            self.strict_date()
        } else {
            self.lax_date()
        }
    }
    fn strict_date(&self) -> ValResult<EitherDate>;
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn lax_date(&self) -> ValResult<EitherDate> {
        self.strict_date()
    }

    fn validate_time(&self, strict: bool) -> ValResult<EitherTime> {
        if strict {
            self.strict_time()
        } else {
            self.lax_time()
        }
    }
    fn strict_time(&self) -> ValResult<EitherTime>;
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn lax_time(&self) -> ValResult<EitherTime> {
        self.strict_time()
    }

    fn validate_datetime(&self, strict: bool) -> ValResult<EitherDateTime> {
        if strict {
            self.strict_datetime()
        } else {
            self.lax_datetime()
        }
    }
    fn strict_datetime(&self) -> ValResult<EitherDateTime>;
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn lax_datetime(&self) -> ValResult<EitherDateTime> {
        self.strict_datetime()
    }

    fn validate_timedelta(&self, strict: bool) -> ValResult<EitherTimedelta> {
        if strict {
            self.strict_timedelta()
        } else {
            self.lax_timedelta()
        }
    }
    fn strict_timedelta(&self) -> ValResult<EitherTimedelta>;
    #[cfg_attr(has_no_coverage, no_coverage)]
    fn lax_timedelta(&self) -> ValResult<EitherTimedelta> {
        self.strict_timedelta()
    }
}
