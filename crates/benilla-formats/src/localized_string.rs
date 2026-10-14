//! Non-macOS carry for localized vanilla installs. A localized block has eight string offsets
//! followed by a flags word. Most installs populate one language only; never scan the flags.
use benilla_dbc::{Record, RecordSet, StringRef, Value};

pub(crate) fn str_at(rs: &RecordSet, r: &Record, first: usize) -> Option<String> {
    (first..first + 8).find_map(|column| {
        let offset = match r.get_value(column)? {
            Value::StringRef(StringRef(offset)) | Value::UInt32(offset) => *offset,
            Value::Int32(offset) => *offset as u32,
            _ => return None,
        };
        if offset == 0 {
            return None;
        }
        let text = rs.get_string(StringRef(offset)).ok()?;
        (!text.is_empty()).then(|| text.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use benilla_dbc::{DbcParser, FieldType, Schema, SchemaField};
    use std::io::Cursor;

    fn table(offsets: [u32; 9]) -> RecordSet {
        let strings = b"\0Shadow Bolt\0Trait de l'ombre\0not a locale\0";
        let mut bytes = b"WDBC".to_vec();
        for word in [1u32, 9, 36, strings.len() as u32] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for offset in offsets {
            bytes.extend_from_slice(&offset.to_le_bytes());
        }
        bytes.extend_from_slice(strings);
        let mut schema = Schema::new("Locales");
        for column in 0..9 {
            schema.add_field(SchemaField::new(
                format!("locale{column}"),
                if column == 0 {
                    FieldType::String
                } else {
                    FieldType::UInt32
                },
            ));
        }
        DbcParser::parse(&mut Cursor::new(bytes.as_slice()))
            .unwrap()
            .with_schema(schema)
            .unwrap()
            .parse_records()
            .unwrap()
    }

    #[test]
    fn french_only_install_uses_slot_two() {
        let set = table([0, 0, 13, 0, 0, 0, 0, 0, 29]);
        assert_eq!(
            str_at(&set, &set.records()[0], 0).as_deref(),
            Some("Trait de l'ombre")
        );
    }

    #[test]
    fn primary_locale_is_preserved_and_flags_are_not_read_as_text() {
        let set = table([1, 0, 13, 0, 0, 0, 0, 0, 29]);
        assert_eq!(
            str_at(&set, &set.records()[0], 0).as_deref(),
            Some("Shadow Bolt")
        );
        let empty = table([0, 0, 0, 0, 0, 0, 0, 0, 29]);
        assert_eq!(str_at(&empty, &empty.records()[0], 0), None);
    }
}
