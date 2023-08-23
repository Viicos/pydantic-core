import re
from datetime import date, datetime

import pytest

from pydantic_core import SchemaValidator, ValidationError, core_schema

from .conftest import Err


def test_bool():
    v = SchemaValidator(core_schema.bool_schema())

    assert v.validate_string('true') is True
    assert v.validate_string('true', strict=True) is True
    assert v.validate_string('false') is False


@pytest.mark.parametrize(
    'schema,input_value,expected,strict',
    [
        (core_schema.int_schema(), '1', 1, False),
        (core_schema.int_schema(), '1', 1, True),
        (core_schema.int_schema(), 'xxx', Err('type=int_parsing'), True),
        (core_schema.float_schema(), '1.1', 1.1, False),
        (core_schema.float_schema(), '1.10', 1.1, False),
        (core_schema.float_schema(), '1.1', 1.1, True),
        (core_schema.float_schema(), '1.10', 1.1, True),
        (core_schema.date_schema(), '2017-01-01', date(2017, 1, 1), False),
        (core_schema.date_schema(), '2017-01-01', date(2017, 1, 1), True),
        (core_schema.datetime_schema(), '2017-01-01T12:13:14.567', datetime(2017, 1, 1, 12, 13, 14, 567_000), False),
        (core_schema.datetime_schema(), '2017-01-01T12:13:14.567', datetime(2017, 1, 1, 12, 13, 14, 567_000), True),
        (core_schema.date_schema(), '2017-01-01T12:13:14.567', Err('type=date_from_datetime_inexact'), False),
        (core_schema.date_schema(), '2017-01-01T12:13:14.567', Err('type=date_parsing'), True),
        (core_schema.date_schema(), '2017-01-01T00:00:00', date(2017, 1, 1), False),
        (core_schema.date_schema(), '2017-01-01T00:00:00', Err('type=date_parsing'), True),
    ],
    ids=repr,
)
def test_validate_string(schema, input_value, expected, strict):
    v = SchemaValidator(schema)
    if isinstance(expected, Err):
        with pytest.raises(ValidationError, match=re.escape(expected.message)):
            v.validate_string(input_value, strict=strict)
    else:
        assert v.validate_string(input_value, strict=strict) == expected


def test_dict():
    v = SchemaValidator(core_schema.dict_schema(core_schema.int_schema(), core_schema.date_schema()))

    assert v.validate_string({'1': '2017-01-01', '2': '2017-01-02'}) == {1: date(2017, 1, 1), 2: date(2017, 1, 2)}
