use std::{fmt::Display, num::TryFromIntError, path::Path};

use chrono::{NaiveTime, Timelike};
use datafusion::{
    arrow::{
        self,
        array::{ArrowPrimitiveType, AsArray, RecordBatch},
        datatypes::{
            ArrowNativeType, DataType, Float64Type, Int8Type, Time32MillisecondType,
            Time32SecondType, ToByteSlice,
        },
    },
    prelude::{DataFrame, ParquetReadOptions, SessionContext},
};
use nu_plugin::EvaluatedCall;
use nu_protocol::{IntoValue, LabeledError, ShellError, Span, TryIntoValue, Value, record};

use crate::unicode::commands::ucd::index::get_index_dir;

pub async fn ucd_df(
    ctx: &SessionContext,
    call: &EvaluatedCall,
    path: impl AsRef<Path>,
) -> Result<DataFrame, LabeledError> {
    let unicode_data_path = get_index_dir(call)?
        .join(path)
        .into_os_string()
        .into_string()
        .map_err(|err| LabeledError::new(format!("non-UTF-8 path: {:?}", err)))?;

    ctx.read_parquet(unicode_data_path, ParquetReadOptions::default())
        .await
        .map_err(|err| LabeledError::new(format!("error reading {}", err)))
}

pub(crate) struct Array(Box<dyn datafusion::arrow::array::Array>);

impl TryIntoValue for Array {
    fn try_into_value(self, span: Span) -> Result<Value, ShellError> {
        let len = self.0.len();

        let result = match self.0.data_type() {
            DataType::Null => Value::list(
                std::iter::repeat_n(Value::nothing(span), len).collect(),
                span,
            ),
            DataType::Boolean => Value::list(
                self.0
                    .as_boolean()
                    .values()
                    .iter()
                    .map(|b| Value::bool(b, span))
                    .collect(),
                span,
            ),
            DataType::Int8 => primitive_to_int::<arrow::datatypes::Int8Type>(self.0, span),
            DataType::Int16 => primitive_to_int::<arrow::datatypes::Int16Type>(self.0, span),
            DataType::Int32 => primitive_to_int::<arrow::datatypes::Int32Type>(self.0, span),
            DataType::Int64 => primitive_to_int::<arrow::datatypes::Int64Type>(self.0, span),
            DataType::UInt8 => primitive_to_int::<arrow::datatypes::UInt8Type>(self.0, span),
            DataType::UInt16 => primitive_to_int::<arrow::datatypes::UInt16Type>(self.0, span),
            DataType::UInt32 => primitive_to_int::<arrow::datatypes::UInt32Type>(self.0, span),
            DataType::UInt64 => primitive_to_int::<arrow::datatypes::UInt64Type>(self.0, span),
            DataType::Float16 => primitive_to_float::<arrow::datatypes::Float16Type>(self.0, span),
            DataType::Float32 => primitive_to_float::<arrow::datatypes::Float32Type>(self.0, span),
            DataType::Float64 => primitive_to_float::<arrow::datatypes::Float64Type>(self.0, span),
            DataType::Timestamp(time_unit, tz) => {
                let tz: Option<chrono_tz::Tz> = match tz.as_ref().map(|zone| zone.as_ref()) {
                    // per the Arrow spec, the empty string must be treated the same as no time zone given
                    //
                    // https://arrow.apache.org/docs/cpp/api/datatype.html#_CPPv4N5arrow13TimestampTypeE
                    None | Some("") => None,
                    Some(zone) => Some(zone.parse::<chrono_tz::Tz>().map_err(|err| {
                        LabeledError::new("invalid time zone")
                            .with_label(err.to_string(), Span::unknown())
                    })),
                }
                .transpose()?;

                match time_unit {
                    arrow::datatypes::TimeUnit::Second => {
                        to_timestamps::<arrow::datatypes::TimestampSecondType>(
                            self.0,
                            chrono::DateTime::from_timestamp_secs,
                            &tz,
                        )
                    }
                    arrow::datatypes::TimeUnit::Millisecond => {
                        to_timestamps::<arrow::datatypes::TimestampMillisecondType>(
                            self.0,
                            chrono::DateTime::from_timestamp_millis,
                            &tz,
                        )
                    }
                    arrow::datatypes::TimeUnit::Microsecond => {
                        to_timestamps::<arrow::datatypes::TimestampMicrosecondType>(
                            self.0,
                            chrono::DateTime::from_timestamp_micros,
                            &tz,
                        )
                    }
                    arrow::datatypes::TimeUnit::Nanosecond => {
                        to_timestamps::<arrow::datatypes::TimestampNanosecondType>(
                            self.0,
                            |i| Some(chrono::DateTime::from_timestamp_nanos(i)),
                            &tz,
                        )
                    }
                }?
            }
            DataType::Date32 => self
                .0
                .as_primitive::<arrow::datatypes::Date32Type>()
                .values()
                .into_iter()
                .map(|ts| {
                    let date = chrono::NaiveDate::from_epoch_days(*ts)
                        .ok_or_else(|| {
                            LabeledError::new("invalid date")
                                .with_label(format!("invalid date: {}", *ts), Span::unknown())
                        })?
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_utc()
                        .fixed_offset()
                        .into_value(Span::unknown());

                    Ok(date)
                })
                .collect::<Result<Vec<_>, LabeledError>>()?
                .into_value(Span::unknown()),
            // according to datafusion, the Date64 type is just a Unix epoch
            // timestamp in milliseconds that is supposed to be divisible by one
            // day in milliseconds
            DataType::Date64 => to_timestamps::<arrow::datatypes::Date64Type>(
                self.0,
                chrono::DateTime::from_timestamp_millis,
                &None,
            )?,
            DataType::Time32(time_unit) => match time_unit {
                arrow::datatypes::TimeUnit::Second => {
                    to_duration::<arrow::datatypes::Time32SecondType, _, _, _>(self.0, |i| {
                        chrono::Duration::seconds(i as i64)
                    })
                }
                arrow::datatypes::TimeUnit::Millisecond => {
                    to_duration::<arrow::datatypes::Time32MillisecondType, _, _, _>(self.0, |i| {
                        chrono::Duration::milliseconds(i as i64)
                    })
                }
                unit => Err(
                    LabeledError::new("invalid time unit for Time32").with_label(
                        format!(
                            "Time32 should only have second or millisecond units, but got: {:?}",
                            unit
                        ),
                        span,
                    ),
                ),
            }?,
            DataType::Time64(time_unit) => match time_unit {
                arrow::datatypes::TimeUnit::Microsecond => {
                    to_duration::<arrow::datatypes::Time64MicrosecondType,  _, _, _>(
                        self.0,
                        chrono::Duration::microseconds,
                    )
                }
                arrow::datatypes::TimeUnit::Nanosecond => {
                    to_duration::<arrow::datatypes::Time64NanosecondType, _, _, _>(
                        self.0,
                        chrono::Duration::nanoseconds,
                    )
                }
                unit => Err(
                    LabeledError::new("invalid time unit for Time64").with_label(
                        format!(
                            "Time64 should only have microsecond or nanosecond units, but got: {:?}",
                            unit
                        ),
                        span,
                    ),
                ),
            }?,
            DataType::Duration(time_unit) => {
                match time_unit {
                    arrow::datatypes::TimeUnit::Second => {
                        to_duration::<arrow::datatypes::DurationSecondType, _, _, _>(
                            self.0,
                            chrono::Duration::seconds,
                        )
                    },
                    arrow::datatypes::TimeUnit::Millisecond => {
                        to_duration::<arrow::datatypes::DurationMillisecondType, _, _, _>(
                            self.0,
                            chrono::Duration::milliseconds,
                        )
                    },
                    arrow::datatypes::TimeUnit::Microsecond => {
                        to_duration::<arrow::datatypes::DurationMicrosecondType, _, _, _>(
                            self.0,
                            chrono::Duration::microseconds,
                        )
                    }
                    arrow::datatypes::TimeUnit::Nanosecond => {
                        to_duration::<arrow::datatypes::DurationNanosecondType, _, _, _>(
                            self.0,
                            chrono::Duration::nanoseconds,
                        )
                    }
                }?
            },
            DataType::Interval(interval_unit) => {
                let unit = *interval_unit;
                interval_to_value(self.0, unit)?
            }?,
            DataType::Binary => todo!(),
            DataType::FixedSizeBinary(_) => todo!(),
            DataType::LargeBinary => todo!(),
            DataType::BinaryView => todo!(),
            DataType::Utf8 => todo!(),
            DataType::LargeUtf8 => todo!(),
            DataType::Utf8View => todo!(),
            DataType::List(field) => todo!(),
            DataType::ListView(field) => todo!(),
            DataType::FixedSizeList(field, _) => todo!(),
            DataType::LargeList(field) => todo!(),
            DataType::LargeListView(field) => todo!(),
            DataType::Struct(fields) => todo!(),
            DataType::Union(union_fields, union_mode) => todo!(),
            DataType::Dictionary(data_type, data_type1) => todo!(),
            DataType::Decimal32(_, _) => todo!(),
            DataType::Decimal64(_, _) => todo!(),
            DataType::Decimal128(_, _) => todo!(),
            DataType::Decimal256(_, _) => todo!(),
            DataType::Map(field, _) => todo!(),
            DataType::RunEndEncoded(field, field1) => todo!(),
        };

        Ok(result)
    }
}

fn interval_to_value(
    array: Box<dyn arrow::array::Array>,
    interval_unit: arrow::datatypes::IntervalUnit,
) -> Result<Result<Value, LabeledError>, ShellError> {
    Ok(match interval_unit {
        arrow::datatypes::IntervalUnit::YearMonth => {
            let result = array
                .as_primitive::<arrow::datatypes::IntervalYearMonthType>()
                .values()
                .into_iter()
                .map(|ts| {
                    let interval = nu_protocol::record!(
                        "unit" => Value::string("months", Span::unknown()),
                        "value" => Value::int((*ts).into(), Span::unknown()),
                    );

                    Ok(Value::list(
                        vec![Value::record(interval, Span::unknown())],
                        Span::unknown(),
                    ))
                })
                .collect::<Result<Vec<_>, LabeledError>>()?
                .into_value(Span::unknown());

            Result::<_, LabeledError>::Ok(result)
        }
        arrow::datatypes::IntervalUnit::DayTime => {
            let result = array
                .as_primitive::<arrow::datatypes::IntervalDayTimeType>()
                .values()
                .into_iter()
                .map(|day_time| {
                    let duration = chrono::TimeDelta::days(day_time.days.into())
                        + chrono::TimeDelta::milliseconds(day_time.milliseconds.into());

                    let nanos = duration.num_nanoseconds().ok_or_else(|| {
                        LabeledError::new("overflow").with_label(
                            format!("duration '{}' overflowed nanosecond precision", duration),
                            Span::unknown(),
                        )
                    })?;

                    let interval = nu_protocol::record!(
                        "unit" => Value::string("time", Span::unknown()),
                        "value" => Value::duration(nanos, Span::unknown()),
                    );

                    Ok(Value::list(
                        vec![Value::record(interval, Span::unknown())],
                        Span::unknown(),
                    ))
                })
                .collect::<Result<Vec<_>, LabeledError>>()?
                .into_value(Span::unknown());

            Result::<_, LabeledError>::Ok(result)
        }
        arrow::datatypes::IntervalUnit::MonthDayNano => {
            todo!()
        }
    })
}

fn invalid_timestamp(ts: i64) -> LabeledError {
    LabeledError::new("invalid timestamp").with_label(
        format!("column contains invalid timestamp: {}", ts),
        Span::unknown(),
    )
}

fn to_timestamps<T: ArrowPrimitiveType<Native = i64>>(
    array: Box<dyn arrow::array::Array>,
    to_dt: impl Fn(i64) -> Option<chrono::DateTime<chrono::Utc>>,
    tz: &Option<chrono_tz::Tz>,
) -> Result<Value, LabeledError> {
    let val = array
        .as_primitive::<T>()
        .values()
        .into_iter()
        .map(|ts| {
            // per the Arrow spec, if a time zone is given,
            // the numeric value must be treated as UTC and
            // then converted to the time zone
            let mut dt = to_dt(*ts)
                .ok_or_else(|| invalid_timestamp(*ts))?
                .with_timezone(&chrono_tz::UTC);

            if let Some(zone) = tz {
                dt = dt.with_timezone(&zone);
            }

            Ok(Value::date(dt.fixed_offset(), Span::unknown()))
        })
        .collect::<Result<Vec<_>, LabeledError>>()?
        .into_value(Span::unknown());

    Ok(val)
}

fn to_duration<T, I, D, E>(
    array: Box<dyn arrow::array::Array>,
    to_duration: D,
) -> Result<Value, LabeledError>
where
    T: ArrowPrimitiveType<Native = I>,
    I: TryInto<T::Native, Error = E> + ArrowNativeType,
    D: Fn(T::Native) -> chrono::Duration,
    E: Display,
{
    let val = array
        .as_primitive::<T>()
        .values()
        .into_iter()
        .map(|ts| {
            let ts = (*ts).try_into().map_err(|err| {
                LabeledError::new("could not convert").with_label(err.to_string(), Span::unknown())
            })?;

            let duration = to_duration(ts);

            // we know this number has already been successfully converted
            // to an i32 we this point, so we know it can be converted back
            // out of the duration as well
            Ok(Value::duration(
                duration.num_nanoseconds().unwrap().try_into().unwrap(),
                Span::unknown(),
            ))
        })
        .collect::<Result<Vec<_>, LabeledError>>()?
        .into_value(Span::unknown());

    Ok(val)
}

fn primitive_to_int<T: ArrowPrimitiveType>(
    array: Box<dyn datafusion::arrow::array::Array>,
    span: Span,
) -> Value {
    Value::list(
        array
            .as_primitive::<T>()
            .values()
            .into_iter()
            .map(|i| {
                if let Some(num) = i.to_i64() {
                    Value::int(num, span)
                } else {
                    Value::binary(i.to_byte_slice(), span)
                }
            })
            .collect(),
        span,
    )
}

fn primitive_to_float<T: ArrowPrimitiveType>(
    array: Box<dyn datafusion::arrow::array::Array>,
    span: Span,
) -> Value {
    Value::list(
        array
            .as_primitive::<Float64Type>()
            .values()
            .into_iter()
            .map(|i| Value::float(*i, span))
            .collect(),
        span,
    )
}
