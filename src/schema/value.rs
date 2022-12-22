use std::borrow::Cow;
use std::fmt;
use std::net::Ipv6Addr;

use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Map;

use crate::schema::Facet;
use crate::tokenizer::PreTokenizedString;
use crate::DateTime;

/// Value represents the value of a any field.
/// It is an enum over all over all of the possible field type.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    /// The str type is used for any text information.
    Str(Cow<'a, str>),
    /// Pre-tokenized str type,
    PreTokStr(PreTokenizedString),
    /// Unsigned 64-bits Integer `u64`
    U64(u64),
    /// Signed 64-bits Integer `i64`
    I64(i64),
    /// 64-bits Float `f64`
    F64(f64),
    /// Bool value
    Bool(bool),
    /// Date/time with microseconds precision
    Date(DateTime),
    /// Facet
    Facet(Facet),
    /// Arbitrarily sized byte array
    // TODO allow Cow<'a, [u8]>
    Bytes(Vec<u8>),
    /// Json object value.
    // TODO allow Cow keys and borrowed values
    JsonObject(serde_json::Map<String, serde_json::Value>),
    /// IpV6 Address. Internally there is no IpV4, it needs to be converted to `Ipv6Addr`.
    IpAddr(Ipv6Addr),
}

impl<'a> Value<'a> {
    /// Convert a borrowing [`Value`] to an owning one.
    pub fn into_owned(self) -> Value<'static> {
        use Value::*;
        match self {
            Str(val) => Str(Cow::Owned(val.into_owned())),
            PreTokStr(val) => PreTokStr(val),
            U64(val) => U64(val),
            I64(val) => I64(val),
            F64(val) => F64(val),
            Bool(val) => Bool(val),
            Date(val) => Date(val),
            Facet(val) => Facet(val),
            Bytes(val) => Bytes(val),
            JsonObject(val) => JsonObject(val),
            IpAddr(val) => IpAddr(val),
        }
    }
}

impl<'a> Eq for Value<'a> {}

impl<'a> Serialize for Value<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        match *self {
            Value::Str(ref v) => serializer.serialize_str(v),
            Value::PreTokStr(ref v) => v.serialize(serializer),
            Value::U64(u) => serializer.serialize_u64(u),
            Value::I64(u) => serializer.serialize_i64(u),
            Value::F64(u) => serializer.serialize_f64(u),
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::Date(ref date) => time::serde::rfc3339::serialize(&date.into_utc(), serializer),
            Value::Facet(ref facet) => facet.serialize(serializer),
            Value::Bytes(ref bytes) => serializer.serialize_str(&base64::encode(bytes)),
            Value::JsonObject(ref obj) => obj.serialize(serializer),
            Value::IpAddr(ref obj) => {
                // Ensure IpV4 addresses get serialized as IpV4, but excluding IpV6 loopback.
                if let Some(ip_v4) = obj.to_ipv4_mapped() {
                    ip_v4.serialize(serializer)
                } else {
                    obj.serialize(serializer)
                }
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a string or u32")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Value::I64(v))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Value::U64(v))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
                Ok(Value::F64(v))
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
                Ok(Value::Bool(v))
            }

            // TODO add visit_borrowed_str
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Value::Str(Cow::Owned(v.to_owned())))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
                Ok(Value::Str(Cow::Owned(v)))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl<'a> Value<'a> {
    /// Returns the text value, provided the value is of the `Str` type.
    /// (Returns `None` if the value is not of the `Str` type).
    pub fn as_text(&self) -> Option<&str> {
        if let Value::Str(text) = self {
            Some(text)
        } else {
            None
        }
    }

    /// Returns the facet value, provided the value is of the `Facet` type.
    /// (Returns `None` if the value is not of the `Facet` type).
    pub fn as_facet(&self) -> Option<&Facet> {
        if let Value::Facet(facet) = self {
            Some(facet)
        } else {
            None
        }
    }

    /// Returns the tokenized text, provided the value is of the `PreTokStr` type.
    /// (Returns `None` if the value is not of the `PreTokStr` type.)
    pub fn tokenized_text(&self) -> Option<&PreTokenizedString> {
        if let Value::PreTokStr(tokenized_text) = self {
            Some(tokenized_text)
        } else {
            None
        }
    }

    /// Returns the u64-value, provided the value is of the `U64` type.
    /// (Returns `None` if the value is not of the `U64` type)
    pub fn as_u64(&self) -> Option<u64> {
        if let Value::U64(val) = self {
            Some(*val)
        } else {
            None
        }
    }

    /// Returns the i64-value, provided the value is of the `I64` type.
    ///
    /// Returns `None` if the value is not of type `I64`.
    pub fn as_i64(&self) -> Option<i64> {
        if let Value::I64(val) = self {
            Some(*val)
        } else {
            None
        }
    }

    /// Returns the f64-value, provided the value is of the `F64` type.
    ///
    /// Returns `None` if the value is not of type `F64`.
    pub fn as_f64(&self) -> Option<f64> {
        if let Value::F64(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    /// Returns the bool value, provided the value is of the `Bool` type.
    ///
    /// Returns `None` if the value is not of type `Bool`.
    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    /// Returns the Date-value, provided the value is of the `Date` type.
    ///
    /// Returns `None` if the value is not of type `Date`.
    pub fn as_date(&self) -> Option<DateTime> {
        if let Value::Date(date) = self {
            Some(*date)
        } else {
            None
        }
    }

    /// Returns the Bytes-value, provided the value is of the `Bytes` type.
    ///
    /// Returns `None` if the value is not of type `Bytes`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        if let Value::Bytes(bytes) = self {
            Some(bytes)
        } else {
            None
        }
    }

    /// Returns the json object, provided the value is of the `JsonObject` type.
    ///
    /// Returns `None` if the value is not of type `JsonObject`.
    pub fn as_json(&self) -> Option<&Map<String, serde_json::Value>> {
        if let Value::JsonObject(json) = self {
            Some(json)
        } else {
            None
        }
    }

    /// Returns the ip addr, provided the value is of the `Ip` type.
    /// (Returns None if the value is not of the `Ip` type)
    pub fn as_ip_addr(&self) -> Option<Ipv6Addr> {
        if let Value::IpAddr(val) = self {
            Some(*val)
        } else {
            None
        }
    }
}

impl From<String> for Value<'static> {
    fn from(s: String) -> Value<'static> {
        Value::Str(Cow::Owned(s))
    }
}

impl From<Ipv6Addr> for Value<'static> {
    fn from(v: Ipv6Addr) -> Value<'static> {
        Value::IpAddr(v)
    }
}

impl From<u64> for Value<'static> {
    fn from(v: u64) -> Value<'static> {
        Value::U64(v)
    }
}

impl From<i64> for Value<'static> {
    fn from(v: i64) -> Value<'static> {
        Value::I64(v)
    }
}

impl From<f64> for Value<'static> {
    fn from(v: f64) -> Value<'static> {
        Value::F64(v)
    }
}

impl From<bool> for Value<'static> {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<DateTime> for Value<'static> {
    fn from(dt: DateTime) -> Value<'static> {
        Value::Date(dt)
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Value<'a> {
        Value::Str(Cow::Borrowed(s))
    }
}

// TODO change lifetime to 'a
impl<'a> From<&'a [u8]> for Value<'static> {
    fn from(bytes: &'a [u8]) -> Value<'static> {
        Value::Bytes(bytes.to_vec())
    }
}

impl From<Facet> for Value<'static> {
    fn from(facet: Facet) -> Value<'static> {
        Value::Facet(facet)
    }
}

impl From<Vec<u8>> for Value<'static> {
    fn from(bytes: Vec<u8>) -> Value<'static> {
        Value::Bytes(bytes)
    }
}

impl From<PreTokenizedString> for Value<'static> {
    fn from(pretokenized_string: PreTokenizedString) -> Value<'static> {
        Value::PreTokStr(pretokenized_string)
    }
}

impl From<serde_json::Map<String, serde_json::Value>> for Value<'static> {
    fn from(json_object: serde_json::Map<String, serde_json::Value>) -> Value<'static> {
        Value::JsonObject(json_object)
    }
}

impl From<serde_json::Value> for Value<'static> {
    fn from(json_value: serde_json::Value) -> Value<'static> {
        match json_value {
            serde_json::Value::Object(json_object) => Value::JsonObject(json_object),
            _ => {
                panic!("Expected a json object.");
            }
        }
    }
}

mod binary_serialize {
    use std::borrow::Cow;
    use std::io::{self, Read, Write};
    use std::net::Ipv6Addr;

    use common::{f64_to_u64, u64_to_f64, BinarySerializable};
    use fastfield_codecs::MonotonicallyMappableToU128;

    use super::Value;
    use crate::schema::Facet;
    use crate::tokenizer::PreTokenizedString;
    use crate::DateTime;

    const TEXT_CODE: u8 = 0;
    const U64_CODE: u8 = 1;
    const I64_CODE: u8 = 2;
    const HIERARCHICAL_FACET_CODE: u8 = 3;
    const BYTES_CODE: u8 = 4;
    const DATE_CODE: u8 = 5;
    const F64_CODE: u8 = 6;
    const EXT_CODE: u8 = 7;
    const JSON_OBJ_CODE: u8 = 8;
    const BOOL_CODE: u8 = 9;
    const IP_CODE: u8 = 10;

    // extended types

    const TOK_STR_CODE: u8 = 0;

    impl<'a> BinarySerializable for Value<'a> {
        fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
            match *self {
                Value::Str(ref text) => {
                    TEXT_CODE.serialize(writer)?;
                    text.serialize(writer)
                }
                Value::PreTokStr(ref tok_str) => {
                    EXT_CODE.serialize(writer)?;
                    TOK_STR_CODE.serialize(writer)?;
                    if let Ok(text) = serde_json::to_string(tok_str) {
                        text.serialize(writer)
                    } else {
                        Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Failed to dump Value::PreTokStr(_) to json.",
                        ))
                    }
                }
                Value::U64(ref val) => {
                    U64_CODE.serialize(writer)?;
                    val.serialize(writer)
                }
                Value::I64(ref val) => {
                    I64_CODE.serialize(writer)?;
                    val.serialize(writer)
                }
                Value::F64(ref val) => {
                    F64_CODE.serialize(writer)?;
                    f64_to_u64(*val).serialize(writer)
                }
                Value::Bool(ref val) => {
                    BOOL_CODE.serialize(writer)?;
                    val.serialize(writer)
                }
                Value::Date(ref val) => {
                    DATE_CODE.serialize(writer)?;
                    let DateTime {
                        timestamp_micros, ..
                    } = val;
                    timestamp_micros.serialize(writer)
                }
                Value::Facet(ref facet) => {
                    HIERARCHICAL_FACET_CODE.serialize(writer)?;
                    facet.serialize(writer)
                }
                Value::Bytes(ref bytes) => {
                    BYTES_CODE.serialize(writer)?;
                    bytes.serialize(writer)
                }
                Value::JsonObject(ref map) => {
                    JSON_OBJ_CODE.serialize(writer)?;
                    serde_json::to_writer(writer, &map)?;
                    Ok(())
                }
                Value::IpAddr(ref ip) => {
                    IP_CODE.serialize(writer)?;
                    ip.to_u128().serialize(writer)
                }
            }
        }

        fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self> {
            let type_code = u8::deserialize(reader)?;
            match type_code {
                TEXT_CODE => {
                    let text = String::deserialize(reader)?;
                    Ok(Value::Str(Cow::Owned(text)))
                }
                U64_CODE => {
                    let value = u64::deserialize(reader)?;
                    Ok(Value::U64(value))
                }
                I64_CODE => {
                    let value = i64::deserialize(reader)?;
                    Ok(Value::I64(value))
                }
                F64_CODE => {
                    let value = u64_to_f64(u64::deserialize(reader)?);
                    Ok(Value::F64(value))
                }
                BOOL_CODE => {
                    let value = bool::deserialize(reader)?;
                    Ok(Value::Bool(value))
                }
                DATE_CODE => {
                    let timestamp_micros = i64::deserialize(reader)?;
                    Ok(Value::Date(DateTime::from_timestamp_micros(
                        timestamp_micros,
                    )))
                }
                HIERARCHICAL_FACET_CODE => Ok(Value::Facet(Facet::deserialize(reader)?)),
                BYTES_CODE => Ok(Value::Bytes(Vec::<u8>::deserialize(reader)?)),
                EXT_CODE => {
                    let ext_type_code = u8::deserialize(reader)?;
                    match ext_type_code {
                        TOK_STR_CODE => {
                            let str_val = String::deserialize(reader)?;
                            if let Ok(value) = serde_json::from_str::<PreTokenizedString>(&str_val)
                            {
                                Ok(Value::PreTokStr(value))
                            } else {
                                Err(io::Error::new(
                                    io::ErrorKind::Other,
                                    "Failed to parse string data as Value::PreTokStr(_).",
                                ))
                            }
                        }
                        _ => Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "No extended field type is associated with code {:?}",
                                ext_type_code
                            ),
                        )),
                    }
                }
                JSON_OBJ_CODE => {
                    // As explained in
                    // https://docs.serde.rs/serde_json/fn.from_reader.html
                    //
                    // `T::from_reader(..)` expects EOF after reading the object,
                    // which is not what we want here.
                    //
                    // For this reason we need to create our own `Deserializer`.
                    let mut de = serde_json::Deserializer::from_reader(reader);
                    let json_map = <serde_json::Map::<String, serde_json::Value> as serde::Deserialize>::deserialize(&mut de)?;
                    Ok(Value::JsonObject(json_map))
                }
                IP_CODE => {
                    let value = u128::deserialize(reader)?;
                    Ok(Value::IpAddr(Ipv6Addr::from_u128(value)))
                }

                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("No field type is associated with code {:?}", type_code),
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use crate::schema::{BytesOptions, Schema};
    use crate::time::format_description::well_known::Rfc3339;
    use crate::time::OffsetDateTime;
    use crate::{DateTime, Document};

    #[test]
    fn test_parse_bytes_doc() {
        let mut schema_builder = Schema::builder();
        let bytes_options = BytesOptions::default();
        let bytes_field = schema_builder.add_bytes_field("my_bytes", bytes_options);
        let schema = schema_builder.build();
        let mut doc = Document::default();
        doc.add_bytes(bytes_field, "this is a test".as_bytes());
        let json_string = schema.to_json(&doc);
        assert_eq!(json_string, r#"{"my_bytes":["dGhpcyBpcyBhIHRlc3Q="]}"#);
    }

    #[test]
    fn test_parse_empty_bytes_doc() {
        let mut schema_builder = Schema::builder();
        let bytes_options = BytesOptions::default();
        let bytes_field = schema_builder.add_bytes_field("my_bytes", bytes_options);
        let schema = schema_builder.build();
        let mut doc = Document::default();
        doc.add_bytes(bytes_field, "".as_bytes());
        let json_string = schema.to_json(&doc);
        assert_eq!(json_string, r#"{"my_bytes":[""]}"#);
    }

    #[test]
    fn test_parse_many_bytes_doc() {
        let mut schema_builder = Schema::builder();
        let bytes_options = BytesOptions::default();
        let bytes_field = schema_builder.add_bytes_field("my_bytes", bytes_options);
        let schema = schema_builder.build();
        let mut doc = Document::default();
        doc.add_bytes(
            bytes_field,
            "A bigger test I guess\nspanning on multiple lines\nhoping this will work".as_bytes(),
        );
        let json_string = schema.to_json(&doc);
        assert_eq!(
            json_string,
            r#"{"my_bytes":["QSBiaWdnZXIgdGVzdCBJIGd1ZXNzCnNwYW5uaW5nIG9uIG11bHRpcGxlIGxpbmVzCmhvcGluZyB0aGlzIHdpbGwgd29yaw=="]}"#
        );
    }

    #[test]
    fn test_serialize_date() {
        let value = Value::from(DateTime::from_utc(
            OffsetDateTime::parse("1996-12-20T00:39:57+00:00", &Rfc3339).unwrap(),
        ));
        let serialized_value_json = serde_json::to_string_pretty(&value).unwrap();
        assert_eq!(serialized_value_json, r#""1996-12-20T00:39:57Z""#);
        let value = Value::from(DateTime::from_utc(
            OffsetDateTime::parse("1996-12-20T00:39:57-01:00", &Rfc3339).unwrap(),
        ));
        let serialized_value_json = serde_json::to_string_pretty(&value).unwrap();
        // The time zone information gets lost by conversion into `Value::Date` and
        // implicitly becomes UTC.
        assert_eq!(serialized_value_json, r#""1996-12-20T01:39:57Z""#);
    }
}
