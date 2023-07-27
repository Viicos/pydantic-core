use pyo3::prelude::*;
use pyo3::types::PyDict;
use speedate::{Date, Time};
use strum::EnumMessage;

use crate::build_tools::{is_strict, py_schema_error_type};
use crate::errors::{ErrorType, ValError, ValResult};
use crate::input::{EitherDate, Input};

use crate::recursion_guard::RecursionGuard;

use crate::validators::datetime::{NowConstraint, NowOp};

use super::{BuildValidator, CombinedValidator, Definitions, DefinitionsBuilder, Extra, Validator};

#[derive(Debug, Clone)]
pub struct DateValidator {
    strict: bool,
    constraints: Option<DateConstraints>,
}

impl BuildValidator for DateValidator {
    const EXPECTED_TYPE: &'static str = "date";

    fn build(
        schema: &PyDict,
        config: Option<&PyDict>,
        _definitions: &mut DefinitionsBuilder<CombinedValidator>,
    ) -> PyResult<CombinedValidator> {
        Ok(Self {
            strict: is_strict(schema, config)?,
            constraints: DateConstraints::from_py(schema)?,
        }
        .into())
    }
}

impl_py_gc_traverse!(DateValidator {});

impl Validator for DateValidator {
    fn validate<'s, 'data>(
        &'s self,
        py: Python<'data>,
        input: &'data impl Input<'data>,
        extra: &Extra,
        _definitions: &'data Definitions<CombinedValidator>,
        _recursion_guard: &'s mut RecursionGuard,
    ) -> ValResult<'data, PyObject> {
        let date = match input.validate_date(extra.strict.unwrap_or(self.strict)) {
            Ok(date) => date,
            // if the date error was an internal error, return that immediately
            Err(ValError::InternalErr(internal_err)) => return Err(ValError::InternalErr(internal_err)),
            Err(date_err) => match self.strict {
                // if we're in strict mode, we doing try coercing from a date
                true => return Err(date_err),
                // otherwise, try creating a date from a datetime input
                false => date_from_datetime(input, date_err),
            }?,
        };
        if let Some(constraints) = &self.constraints {
            let raw_date = date.as_raw()?;

            if let Some(ref today_constraint) = constraints.today {
                let offset = today_constraint.utc_offset(py)?;
                let today = Date::today(offset).map_err(|e| {
                    py_schema_error_type!("Date::today() error: {}", e.get_documentation().unwrap_or("unknown"))
                })?;
                // `if let Some(c)` to match behaviour of gt/lt/le/ge
                if let Some(c) = raw_date.partial_cmp(&today) {
                    let date_compliant = today_constraint.op.compare(c);
                    if !date_compliant {
                        let error_type = match today_constraint.op {
                            NowOp::Past => ErrorType::DatePast,
                            NowOp::Future => ErrorType::DateFuture,
                        };
                        return Err(ValError::new(error_type, input));
                    }
                }
            }
        }
        Ok(date.try_into_py(py)?)
    }

    fn different_strict_behavior(
        &self,
        _definitions: Option<&DefinitionsBuilder<CombinedValidator>>,
        ultra_strict: bool,
    ) -> bool {
        !ultra_strict
    }

    fn get_name(&self) -> &str {
        Self::EXPECTED_TYPE
    }

    fn complete(&mut self, _definitions: &DefinitionsBuilder<CombinedValidator>) -> PyResult<()> {
        Ok(())
    }
}

/// In lax mode, if the input is not a date, we try parsing the input as a datetime, then check it is an
/// "exact date", e.g. has a zero time component.
fn date_from_datetime<'data>(
    input: &'data impl Input<'data>,
    date_err: ValError<'data>,
) -> ValResult<'data, EitherDate<'data>> {
    let either_dt = match input.validate_datetime(false, speedate::MicrosecondsPrecisionOverflowBehavior::Truncate) {
        Ok(dt) => dt,
        Err(dt_err) => {
            return match dt_err {
                ValError::LineErrors(mut line_errors) => {
                    // if we got a errors while parsing the datetime,
                    // convert DateTimeParsing -> DateFromDatetimeParsing but keep the rest of the error unchanged
                    for line_error in &mut line_errors {
                        match line_error.error_type {
                            ErrorType::DatetimeParsing { ref error } => {
                                line_error.error_type = ErrorType::DateFromDatetimeParsing {
                                    error: error.to_string(),
                                };
                            }
                            _ => {
                                return Err(date_err);
                            }
                        }
                    }
                    Err(ValError::LineErrors(line_errors))
                }
                other => Err(other),
            };
        }
    };
    let dt = either_dt.as_raw()?;
    let zero_time = Time {
        hour: 0,
        minute: 0,
        second: 0,
        microsecond: 0,
        tz_offset: dt.time.tz_offset,
    };
    if dt.time == zero_time {
        Ok(EitherDate::Raw(dt.date))
    } else {
        Err(ValError::new(ErrorType::DateFromDatetimeInexact, input))
    }
}

#[derive(Debug, Clone)]
struct DateConstraints {
    today: Option<NowConstraint>,
}

impl DateConstraints {
    fn from_py(schema: &PyDict) -> PyResult<Option<Self>> {
        let c = Self {
            today: NowConstraint::from_py(schema)?,
        };
        if c.today.is_some() {
            Ok(Some(c))
        } else {
            Ok(None)
        }
    }
}
